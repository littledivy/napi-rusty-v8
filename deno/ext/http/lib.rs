// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use bytes::Bytes;
use deno_core::error::custom_error;
use deno_core::error::AnyError;
use deno_core::futures::channel::mpsc;
use deno_core::futures::channel::oneshot;
use deno_core::futures::future::pending;
use deno_core::futures::future::select;
use deno_core::futures::future::Either;
use deno_core::futures::future::Pending;
use deno_core::futures::future::RemoteHandle;
use deno_core::futures::future::Shared;
use deno_core::futures::never::Never;
use deno_core::futures::pin_mut;
use deno_core::futures::ready;
use deno_core::futures::stream::Peekable;
use deno_core::futures::FutureExt;
use deno_core::futures::StreamExt;
use deno_core::futures::TryFutureExt;
use deno_core::include_js_files;
use deno_core::op_async;
use deno_core::op_sync;
use deno_core::AsyncRefCell;
use deno_core::ByteString;
use deno_core::CancelFuture;
use deno_core::CancelHandle;
use deno_core::CancelTryFuture;
use deno_core::Extension;
use deno_core::OpState;
use deno_core::RcRef;
use deno_core::Resource;
use deno_core::ResourceId;
use deno_core::StringOrBuffer;
use deno_core::ZeroCopyBuf;
use deno_websocket::ws_create_server_stream;
use hyper::server::conn::Http;
use hyper::service::Service;
use hyper::Body;
use hyper::Request;
use hyper::Response;
use serde::Deserialize;
use serde::Serialize;
use std::borrow::Cow;
use std::cell::RefCell;
use std::cmp::min;
use std::error::Error;
use std::future::Future;
use std::io;
use std::mem::replace;
use std::mem::take;
use std::net::SocketAddr;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::task::spawn_local;

pub fn init() -> Extension {
  Extension::builder()
    .js(include_js_files!(
      prefix "deno:ext/http",
      "01_http.js",
    ))
    .ops(vec![
      ("op_http_accept", op_async(op_http_accept)),
      ("op_http_read", op_async(op_http_read)),
      ("op_http_write_headers", op_async(op_http_write_headers)),
      ("op_http_write", op_async(op_http_write)),
      ("op_http_shutdown", op_async(op_http_shutdown)),
      (
        "op_http_websocket_accept_header",
        op_sync(op_http_websocket_accept_header),
      ),
      (
        "op_http_upgrade_websocket",
        op_async(op_http_upgrade_websocket),
      ),
    ])
    .build()
}

struct HttpConnResource {
  addr: SocketAddr,
  scheme: &'static str,
  acceptors_tx: mpsc::UnboundedSender<HttpAcceptor>,
  closed_fut: Shared<RemoteHandle<Result<(), Arc<hyper::Error>>>>,
  cancel_handle: Rc<CancelHandle>, // Closes gracefully and cancels accept ops.
}

impl HttpConnResource {
  fn new<S>(io: S, scheme: &'static str, addr: SocketAddr) -> Self
  where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
  {
    let (acceptors_tx, acceptors_rx) = mpsc::unbounded::<HttpAcceptor>();
    let service = HttpService::new(acceptors_rx);

    let conn_fut = Http::new()
      .with_executor(LocalExecutor)
      .serve_connection(io, service)
      .with_upgrades();

    // When the cancel handle is used, the connection shuts down gracefully.
    // No new HTTP streams will be accepted, but existing streams will be able
    // to continue operating and eventually shut down cleanly.
    let cancel_handle = CancelHandle::new_rc();
    let shutdown_fut = never().or_cancel(&cancel_handle).fuse();

    // A local task that polls the hyper connection future to completion.
    let task_fut = async move {
      pin_mut!(shutdown_fut);
      pin_mut!(conn_fut);
      let result = match select(conn_fut, shutdown_fut).await {
        Either::Left((result, _)) => result,
        Either::Right((_, mut conn_fut)) => {
          conn_fut.as_mut().graceful_shutdown();
          conn_fut.await
        }
      };
      filter_enotconn(result).map_err(Arc::from)
    };
    let (task_fut, closed_fut) = task_fut.remote_handle();
    let closed_fut = closed_fut.shared();
    spawn_local(task_fut);

    Self {
      addr,
      scheme,
      acceptors_tx,
      closed_fut,
      cancel_handle,
    }
  }

  // Accepts a new incoming HTTP request.
  async fn accept(
    self: &Rc<Self>,
  ) -> Result<Option<HttpStreamResource>, AnyError> {
    let fut = async {
      let (request_tx, request_rx) = oneshot::channel();
      let (response_tx, response_rx) = oneshot::channel();

      let acceptor = HttpAcceptor::new(request_tx, response_rx);
      self.acceptors_tx.unbounded_send(acceptor).ok()?;

      let request = request_rx.await.ok()?;
      let stream = HttpStreamResource::new(self, request, response_tx);
      Some(stream)
    };

    async {
      match fut.await {
        Some(stream) => Ok(Some(stream)),
        // Return the connection error, if any.
        None => self.closed().map_ok(|_| None).await,
      }
    }
    .try_or_cancel(&self.cancel_handle)
    .await
  }

  /// A future that completes when this HTTP connection is closed or errors.
  async fn closed(&self) -> Result<(), AnyError> {
    self.closed_fut.clone().map_err(AnyError::from).await
  }

  fn scheme(&self) -> &'static str {
    self.scheme
  }

  fn addr(&self) -> SocketAddr {
    self.addr
  }
}

impl Resource for HttpConnResource {
  fn name(&self) -> Cow<str> {
    "httpConn".into()
  }

  fn close(self: Rc<Self>) {
    self.cancel_handle.cancel();
  }
}

/// Creates a new HttpConn resource which uses `io` as its transport.
pub fn http_create_conn_resource<S>(
  state: &mut OpState,
  io: S,
  addr: SocketAddr,
  scheme: &'static str,
) -> Result<ResourceId, AnyError>
where
  S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
  let conn = HttpConnResource::new(io, scheme, addr);
  let rid = state.resource_table.add(conn);
  Ok(rid)
}

/// An object that implements the `hyper::Service` trait, through which Hyper
/// delivers incoming HTTP requests.
struct HttpService {
  acceptors_rx: Peekable<mpsc::UnboundedReceiver<HttpAcceptor>>,
}

impl HttpService {
  fn new(acceptors_rx: mpsc::UnboundedReceiver<HttpAcceptor>) -> Self {
    let acceptors_rx = acceptors_rx.peekable();
    Self { acceptors_rx }
  }
}

impl Service<Request<Body>> for HttpService {
  type Response = Response<Body>;
  type Error = oneshot::Canceled;
  type Future = oneshot::Receiver<Response<Body>>;

  fn poll_ready(
    &mut self,
    cx: &mut Context<'_>,
  ) -> Poll<Result<(), Self::Error>> {
    let acceptors_rx = Pin::new(&mut self.acceptors_rx);
    let result = ready!(acceptors_rx.poll_peek(cx))
      .map(|_| ())
      .ok_or(oneshot::Canceled);
    Poll::Ready(result)
  }

  fn call(&mut self, request: Request<Body>) -> Self::Future {
    let acceptor = self.acceptors_rx.next().now_or_never().flatten().unwrap();
    acceptor.call(request)
  }
}

/// A pair of one-shot channels which first transfer a HTTP request from the
/// Hyper service to the HttpConn resource, and then take the Response back to
/// the service.
struct HttpAcceptor {
  request_tx: oneshot::Sender<Request<Body>>,
  response_rx: oneshot::Receiver<Response<Body>>,
}

impl HttpAcceptor {
  fn new(
    request_tx: oneshot::Sender<Request<Body>>,
    response_rx: oneshot::Receiver<Response<Body>>,
  ) -> Self {
    Self {
      request_tx,
      response_rx,
    }
  }

  fn call(self, request: Request<Body>) -> oneshot::Receiver<Response<Body>> {
    let Self {
      request_tx,
      response_rx,
    } = self;
    request_tx
      .send(request)
      .map(|_| response_rx)
      .unwrap_or_else(|_| oneshot::channel().1) // Make new canceled receiver.
  }
}

/// A resource representing a single HTTP request/response stream.
struct HttpStreamResource {
  conn: Rc<HttpConnResource>,
  rd: AsyncRefCell<HttpRequestReader>,
  wr: AsyncRefCell<HttpResponseWriter>,
  cancel_handle: CancelHandle,
}

impl HttpStreamResource {
  fn new(
    conn: &Rc<HttpConnResource>,
    request: Request<Body>,
    response_tx: oneshot::Sender<Response<Body>>,
  ) -> Self {
    Self {
      conn: conn.clone(),
      rd: HttpRequestReader::Headers(request).into(),
      wr: HttpResponseWriter::Headers(response_tx).into(),
      cancel_handle: CancelHandle::new(),
    }
  }
}

impl Resource for HttpStreamResource {
  fn name(&self) -> Cow<str> {
    "httpStream".into()
  }

  fn close(self: Rc<Self>) {
    self.cancel_handle.cancel();
  }
}

/// The read half of an HTTP stream.
enum HttpRequestReader {
  Headers(Request<Body>),
  Body(Peekable<Body>),
  Closed,
}

impl Default for HttpRequestReader {
  fn default() -> Self {
    Self::Closed
  }
}

/// The write half of an HTTP stream.
enum HttpResponseWriter {
  Headers(oneshot::Sender<Response<Body>>),
  Body(hyper::body::Sender),
  Closed,
}

impl Default for HttpResponseWriter {
  fn default() -> Self {
    Self::Closed
  }
}

// We use a tuple instead of struct to avoid serialization overhead of the keys.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct NextRequestResponse(
  // stream_rid:
  ResourceId,
  // method:
  // This is a String rather than a ByteString because reqwest will only return
  // the method as a str which is guaranteed to be ASCII-only.
  String,
  // headers:
  Vec<(ByteString, ByteString)>,
  // url:
  String,
);

async fn op_http_accept(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  _: (),
) -> Result<Option<NextRequestResponse>, AnyError> {
  let conn = state.borrow().resource_table.get::<HttpConnResource>(rid)?;

  let stream = match conn.accept().await {
    Ok(Some(stream)) => Rc::new(stream),
    Ok(None) => return Ok(None),
    Err(err) => return Err(err),
  };

  let rd = RcRef::map(&stream, |r| &r.rd).borrow().await;
  let request = match &*rd {
    HttpRequestReader::Headers(request) => request,
    _ => unreachable!(),
  };

  let method = request.method().to_string();
  let headers = req_headers(request);
  let url = req_url(request, conn.scheme(), conn.addr());

  let stream_rid = state.borrow_mut().resource_table.add_rc(stream);

  let r = NextRequestResponse(stream_rid, method, headers, url);
  Ok(Some(r))
}

fn req_url(
  req: &hyper::Request<hyper::Body>,
  scheme: &'static str,
  addr: SocketAddr,
) -> String {
  let host: Cow<str> = if let Some(auth) = req.uri().authority() {
    match addr.port() {
      443 if scheme == "https" => Cow::Borrowed(auth.host()),
      80 if scheme == "http" => Cow::Borrowed(auth.host()),
      _ => Cow::Borrowed(auth.as_str()), // Includes port number.
    }
  } else if let Some(host) = req.uri().host() {
    Cow::Borrowed(host)
  } else if let Some(host) = req.headers().get("HOST") {
    match host.to_str() {
      Ok(host) => Cow::Borrowed(host),
      Err(_) => Cow::Owned(
        host
          .as_bytes()
          .iter()
          .cloned()
          .map(char::from)
          .collect::<String>(),
      ),
    }
  } else {
    Cow::Owned(addr.to_string())
  };
  let path = req.uri().path_and_query().map_or("/", |p| p.as_str());
  [scheme, "://", &host, path].concat()
}

fn req_headers(
  req: &hyper::Request<hyper::Body>,
) -> Vec<(ByteString, ByteString)> {
  // We treat cookies specially, because we don't want them to get them
  // mangled by the `Headers` object in JS. What we do is take all cookie
  // headers and concat them into a single cookie header, separated by
  // semicolons.
  let cookie_sep = "; ".as_bytes();
  let mut cookies = vec![];

  let mut headers = Vec::with_capacity(req.headers().len());
  for (name, value) in req.headers().iter() {
    if name == hyper::header::COOKIE {
      cookies.push(value.as_bytes());
    } else {
      let name: &[u8] = name.as_ref();
      let value = value.as_bytes();
      headers.push((ByteString(name.to_owned()), ByteString(value.to_owned())));
    }
  }

  if !cookies.is_empty() {
    headers.push((
      ByteString("cookie".as_bytes().to_owned()),
      ByteString(cookies.join(cookie_sep)),
    ));
  }

  headers
}

// We use a tuple instead of struct to avoid serialization overhead of the keys.
#[derive(Deserialize)]
struct RespondArgs(
  // rid:
  u32,
  // status:
  u16,
  // headers:
  Vec<(ByteString, ByteString)>,
);

async fn op_http_write_headers(
  state: Rc<RefCell<OpState>>,
  args: RespondArgs,
  data: Option<StringOrBuffer>,
) -> Result<(), AnyError> {
  let RespondArgs(rid, status, headers) = args;
  let stream = state
    .borrow_mut()
    .resource_table
    .get::<HttpStreamResource>(rid)?;

  let mut builder = Response::builder().status(status);

  builder.headers_mut().unwrap().reserve(headers.len());
  for (key, value) in &headers {
    builder = builder.header(key.as_ref(), value.as_ref());
  }

  let body: Response<Body>;
  let new_wr: HttpResponseWriter;

  match data {
    Some(data) => {
      // If a buffer was passed, we use it to construct a response body.
      body = builder.body(data.into_bytes().into())?;
      new_wr = HttpResponseWriter::Closed;
    }
    None => {
      // If no buffer was passed, the caller will stream the response body.
      let (body_tx, body_rx) = Body::channel();
      body = builder.body(body_rx)?;
      new_wr = HttpResponseWriter::Body(body_tx);
    }
  }

  let mut old_wr = RcRef::map(&stream, |r| &r.wr).borrow_mut().await;
  let response_tx = match replace(&mut *old_wr, new_wr) {
    HttpResponseWriter::Headers(response_tx) => response_tx,
    _ => return Err(http_error("response headers already sent")),
  };

  match response_tx.send(body) {
    Ok(_) => Ok(()),
    Err(_) => {
      stream.conn.closed().await?;
      Err(http_error("connection closed while sending response"))
    }
  }
}

async fn op_http_write(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  buf: ZeroCopyBuf,
) -> Result<(), AnyError> {
  let stream = state
    .borrow()
    .resource_table
    .get::<HttpStreamResource>(rid)?;
  let mut wr = RcRef::map(&stream, |r| &r.wr).borrow_mut().await;

  loop {
    let body_tx = match &mut *wr {
      HttpResponseWriter::Body(body_tx) => body_tx,
      HttpResponseWriter::Headers(_) => {
        break Err(http_error("no response headers"))
      }
      HttpResponseWriter::Closed => {
        break Err(http_error("response already completed"))
      }
    };

    let bytes = Bytes::copy_from_slice(&buf[..]);
    match body_tx.send_data(bytes).await {
      Ok(_) => break Ok(()),
      Err(err) => {
        // Don't return "channel closed", that's an implementation detail.
        // Pull up the failure associated with the transport connection instead.
        assert!(err.is_closed());
        stream.conn.closed().await?;
        // If there was no connection error, drop body_tx.
        *wr = HttpResponseWriter::Closed;
      }
    }
  }
}

/// Gracefully closes the write half of the HTTP stream. Note that this does not
/// remove the HTTP stream resource from the resource table; it still has to be
/// closed with `Deno.core.close()`.
async fn op_http_shutdown(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  _: (),
) -> Result<(), AnyError> {
  let stream = state
    .borrow()
    .resource_table
    .get::<HttpStreamResource>(rid)?;
  let mut wr = RcRef::map(&stream, |r| &r.wr).borrow_mut().await;
  take(&mut *wr);
  Ok(())
}

async fn op_http_read(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  mut buf: ZeroCopyBuf,
) -> Result<usize, AnyError> {
  let stream = state
    .borrow_mut()
    .resource_table
    .get::<HttpStreamResource>(rid)?;
  let mut rd = RcRef::map(&stream, |r| &r.rd).borrow_mut().await;

  let body = loop {
    match &mut *rd {
      HttpRequestReader::Headers(_) => {}
      HttpRequestReader::Body(body) => break body,
      HttpRequestReader::Closed => return Ok(0),
    }
    match take(&mut *rd) {
      HttpRequestReader::Headers(request) => {
        let body = request.into_body().peekable();
        *rd = HttpRequestReader::Body(body);
      }
      _ => unreachable!(),
    };
  };

  let fut = async {
    let mut body = Pin::new(body);
    loop {
      match body.as_mut().peek_mut().await {
        Some(Ok(chunk)) if !chunk.is_empty() => {
          let len = min(buf.len(), chunk.len());
          buf[..len].copy_from_slice(&chunk.split_to(len));
          break Ok(len);
        }
        Some(_) => match body.as_mut().next().await.unwrap() {
          Ok(chunk) => assert!(chunk.is_empty()),
          Err(err) => break Err(AnyError::from(err)),
        },
        None => break Ok(0),
      }
    }
  };

  let cancel_handle = RcRef::map(&stream, |r| &r.cancel_handle);
  fut.try_or_cancel(cancel_handle).await
}

fn op_http_websocket_accept_header(
  _: &mut OpState,
  key: String,
  _: (),
) -> Result<String, AnyError> {
  let digest = ring::digest::digest(
    &ring::digest::SHA1_FOR_LEGACY_USE_ONLY,
    format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", key).as_bytes(),
  );
  Ok(base64::encode(digest))
}

async fn op_http_upgrade_websocket(
  state: Rc<RefCell<OpState>>,
  rid: ResourceId,
  _: (),
) -> Result<ResourceId, AnyError> {
  let stream = state
    .borrow_mut()
    .resource_table
    .get::<HttpStreamResource>(rid)?;
  let mut rd = RcRef::map(&stream, |r| &r.rd).borrow_mut().await;

  let request = match &mut *rd {
    HttpRequestReader::Headers(request) => request,
    _ => {
      return Err(http_error("cannot upgrade because request body was used"))
    }
  };

  let transport = hyper::upgrade::on(request).await?;
  let ws_rid = ws_create_server_stream(&state, transport).await?;
  Ok(ws_rid)
}

// Needed so hyper can use non Send futures
#[derive(Clone)]
struct LocalExecutor;

impl<Fut> hyper::rt::Executor<Fut> for LocalExecutor
where
  Fut: Future + 'static,
  Fut::Output: 'static,
{
  fn execute(&self, fut: Fut) {
    spawn_local(fut);
  }
}

fn http_error(message: &'static str) -> AnyError {
  custom_error("Http", message)
}

/// Filters out the ever-surprising 'shutdown ENOTCONN' errors.
fn filter_enotconn(
  result: Result<(), hyper::Error>,
) -> Result<(), hyper::Error> {
  if result
    .as_ref()
    .err()
    .and_then(|err| err.source())
    .and_then(|err| err.downcast_ref::<io::Error>())
    .filter(|err| err.kind() == io::ErrorKind::NotConnected)
    .is_some()
  {
    Ok(())
  } else {
    result
  }
}

/// Create a future that is forever pending.
fn never() -> Pending<Never> {
  pending()
}
