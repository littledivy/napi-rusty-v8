// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
use crate::colors;
use crate::inspector_server::InspectorServer;
use crate::js;
use crate::ops;
use crate::permissions::Permissions;
use crate::tokio_util::run_basic;
use crate::BootstrapOptions;
use deno_broadcast_channel::InMemoryBroadcastChannel;
use deno_core::error::AnyError;
use deno_core::error::JsError;
use deno_core::futures::channel::mpsc;
use deno_core::futures::future::poll_fn;
use deno_core::futures::stream::StreamExt;
use deno_core::futures::task::AtomicWaker;
use deno_core::located_script_name;
use deno_core::serde::Deserialize;
use deno_core::serde::Serialize;
use deno_core::serde_json::json;
use deno_core::v8;
use deno_core::CancelHandle;
use deno_core::CompiledWasmModuleStore;
use deno_core::Extension;
use deno_core::GetErrorClassFn;
use deno_core::JsErrorCreateFn;
use deno_core::JsRuntime;
use deno_core::ModuleId;
use deno_core::ModuleLoader;
use deno_core::ModuleSpecifier;
use deno_core::RuntimeOptions;
use deno_core::SharedArrayBufferStore;
use deno_tls::rustls::RootCertStore;
use deno_web::create_entangled_message_port;
use deno_web::BlobStore;
use deno_web::MessagePort;
use log::debug;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebWorkerType {
  Classic,
  Module,
}

#[derive(
  Debug, Default, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
pub struct WorkerId(u32);
impl fmt::Display for WorkerId {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "worker-{}", self.0)
  }
}
impl WorkerId {
  pub fn next(&self) -> Option<WorkerId> {
    self.0.checked_add(1).map(WorkerId)
  }
}

/// Events that are sent to host from child
/// worker.
pub enum WorkerControlEvent {
  Error(AnyError),
  TerminalError(AnyError),
  Close,
}

use deno_core::serde::Serializer;

impl Serialize for WorkerControlEvent {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    let type_id = match &self {
      WorkerControlEvent::TerminalError(_) => 1_i32,
      WorkerControlEvent::Error(_) => 2_i32,
      WorkerControlEvent::Close => 3_i32,
    };

    match self {
      WorkerControlEvent::TerminalError(error)
      | WorkerControlEvent::Error(error) => {
        let value = match error.downcast_ref::<JsError>() {
          Some(js_error) => json!({
            "message": js_error.message,
            "fileName": js_error.script_resource_name,
            "lineNumber": js_error.line_number,
            "columnNumber": js_error.start_column,
          }),
          None => json!({
            "message": error.to_string(),
          }),
        };

        Serialize::serialize(&(type_id, value), serializer)
      }
      _ => Serialize::serialize(&(type_id, ()), serializer),
    }
  }
}

// Channels used for communication with worker's parent
#[derive(Clone)]
pub struct WebWorkerInternalHandle {
  sender: mpsc::Sender<WorkerControlEvent>,
  pub port: Rc<MessagePort>,
  pub cancel: Rc<CancelHandle>,
  termination_signal: Arc<AtomicBool>,
  has_terminated: Arc<AtomicBool>,
  terminate_waker: Arc<AtomicWaker>,
  isolate_handle: v8::IsolateHandle,
  pub worker_type: WebWorkerType,
}

impl WebWorkerInternalHandle {
  /// Post WorkerEvent to parent as a worker
  pub fn post_event(&self, event: WorkerControlEvent) -> Result<(), AnyError> {
    let mut sender = self.sender.clone();
    // If the channel is closed,
    // the worker must have terminated but the termination message has not yet been received.
    //
    // Therefore just treat it as if the worker has terminated and return.
    if sender.is_closed() {
      self.has_terminated.store(true, Ordering::SeqCst);
      return Ok(());
    }
    sender.try_send(event)?;
    Ok(())
  }

  /// Check if this worker is terminated or being terminated
  pub fn is_terminated(&self) -> bool {
    self.has_terminated.load(Ordering::SeqCst)
  }

  /// Check if this worker must terminate (because the termination signal is
  /// set), and terminates it if so. Returns whether the worker is terminated or
  /// being terminated, as with [`Self::is_terminated()`].
  pub fn terminate_if_needed(&mut self) -> bool {
    let has_terminated = self.is_terminated();

    if !has_terminated && self.termination_signal.load(Ordering::SeqCst) {
      self.terminate();
      return true;
    }

    has_terminated
  }

  /// Terminate the worker
  /// This function will set terminated to true, terminate the isolate and close the message channel
  pub fn terminate(&mut self) {
    self.cancel.cancel();

    // This function can be called multiple times by whomever holds
    // the handle. However only a single "termination" should occur so
    // we need a guard here.
    let already_terminated = self.has_terminated.swap(true, Ordering::SeqCst);

    if !already_terminated {
      // Stop javascript execution
      self.isolate_handle.terminate_execution();
    }

    // Wake parent by closing the channel
    self.sender.close_channel();
  }
}

pub struct SendableWebWorkerHandle {
  port: MessagePort,
  receiver: mpsc::Receiver<WorkerControlEvent>,
  termination_signal: Arc<AtomicBool>,
  has_terminated: Arc<AtomicBool>,
  terminate_waker: Arc<AtomicWaker>,
  isolate_handle: v8::IsolateHandle,
}

impl From<SendableWebWorkerHandle> for WebWorkerHandle {
  fn from(handle: SendableWebWorkerHandle) -> Self {
    WebWorkerHandle {
      receiver: Rc::new(RefCell::new(handle.receiver)),
      port: Rc::new(handle.port),
      termination_signal: handle.termination_signal,
      has_terminated: handle.has_terminated,
      terminate_waker: handle.terminate_waker,
      isolate_handle: handle.isolate_handle,
    }
  }
}

/// This is the handle to the web worker that the parent thread uses to
/// communicate with the worker. It is created from a `SendableWebWorkerHandle`
/// which is sent to the parent thread from the worker thread where it is
/// created. The reason for this separation is that the handle first needs to be
/// `Send` when transferring between threads, and then must be `Clone` when it
/// has arrived on the parent thread. It can not be both at once without large
/// amounts of Arc<Mutex> and other fun stuff.
#[derive(Clone)]
pub struct WebWorkerHandle {
  pub port: Rc<MessagePort>,
  receiver: Rc<RefCell<mpsc::Receiver<WorkerControlEvent>>>,
  termination_signal: Arc<AtomicBool>,
  has_terminated: Arc<AtomicBool>,
  terminate_waker: Arc<AtomicWaker>,
  isolate_handle: v8::IsolateHandle,
}

impl WebWorkerHandle {
  /// Get the WorkerEvent with lock
  /// Return error if more than one listener tries to get event
  pub async fn get_control_event(
    &self,
  ) -> Result<Option<WorkerControlEvent>, AnyError> {
    #![allow(clippy::await_holding_refcell_ref)] // TODO(ry) remove!
    let mut receiver = self.receiver.borrow_mut();
    Ok(receiver.next().await)
  }

  /// Terminate the worker
  /// This function will set the termination signal, close the message channel,
  /// and schedule to terminate the isolate after two seconds.
  pub fn terminate(self) {
    use std::thread::{sleep, spawn};
    use std::time::Duration;

    let schedule_termination =
      !self.termination_signal.swap(true, Ordering::SeqCst);

    self.port.disentangle();

    if schedule_termination && !self.has_terminated.load(Ordering::SeqCst) {
      // Wake up the worker's event loop so it can terminate.
      self.terminate_waker.wake();

      let has_terminated = self.has_terminated.clone();

      // Schedule to terminate the isolate's execution.
      spawn(move || {
        sleep(Duration::from_secs(2));

        // A worker's isolate can only be terminated once, so we need a guard
        // here.
        let already_terminated = has_terminated.swap(true, Ordering::SeqCst);

        if !already_terminated {
          // Stop javascript execution
          self.isolate_handle.terminate_execution();
        }
      });
    }
  }
}

fn create_handles(
  isolate_handle: v8::IsolateHandle,
  worker_type: WebWorkerType,
) -> (WebWorkerInternalHandle, SendableWebWorkerHandle) {
  let (parent_port, worker_port) = create_entangled_message_port();
  let (ctrl_tx, ctrl_rx) = mpsc::channel::<WorkerControlEvent>(1);
  let termination_signal = Arc::new(AtomicBool::new(false));
  let has_terminated = Arc::new(AtomicBool::new(false));
  let terminate_waker = Arc::new(AtomicWaker::new());
  let internal_handle = WebWorkerInternalHandle {
    sender: ctrl_tx,
    port: Rc::new(parent_port),
    termination_signal: termination_signal.clone(),
    has_terminated: has_terminated.clone(),
    terminate_waker: terminate_waker.clone(),
    isolate_handle: isolate_handle.clone(),
    cancel: CancelHandle::new_rc(),
    worker_type,
  };
  let external_handle = SendableWebWorkerHandle {
    receiver: ctrl_rx,
    port: worker_port,
    termination_signal,
    has_terminated,
    terminate_waker,
    isolate_handle,
  };
  (internal_handle, external_handle)
}

/// This struct is an implementation of `Worker` Web API
///
/// Each `WebWorker` is either a child of `MainWorker` or other
/// `WebWorker`.
pub struct WebWorker {
  id: WorkerId,
  pub js_runtime: JsRuntime,
  pub name: String,
  internal_handle: WebWorkerInternalHandle,
  pub use_deno_namespace: bool,
  pub worker_type: WebWorkerType,
  pub main_module: ModuleSpecifier,
}

pub struct WebWorkerOptions {
  pub bootstrap: BootstrapOptions,
  pub extensions: Vec<Extension>,
  pub unsafely_ignore_certificate_errors: Option<Vec<String>>,
  pub root_cert_store: Option<RootCertStore>,
  pub user_agent: String,
  pub seed: Option<u64>,
  pub module_loader: Rc<dyn ModuleLoader>,
  pub create_web_worker_cb: Arc<ops::worker_host::CreateWebWorkerCb>,
  pub js_error_create_fn: Option<Rc<JsErrorCreateFn>>,
  pub use_deno_namespace: bool,
  pub worker_type: WebWorkerType,
  pub maybe_inspector_server: Option<Arc<InspectorServer>>,
  pub get_error_class_fn: Option<GetErrorClassFn>,
  pub blob_store: BlobStore,
  pub broadcast_channel: InMemoryBroadcastChannel,
  pub shared_array_buffer_store: Option<SharedArrayBufferStore>,
  pub compiled_wasm_module_store: Option<CompiledWasmModuleStore>,
  pub maybe_exit_code: Option<Arc<AtomicI32>>,
}

impl WebWorker {
  pub fn bootstrap_from_options(
    name: String,
    permissions: Permissions,
    main_module: ModuleSpecifier,
    worker_id: WorkerId,
    options: WebWorkerOptions,
  ) -> (Self, SendableWebWorkerHandle) {
    let bootstrap_options = options.bootstrap.clone();
    let (mut worker, handle) =
      Self::from_options(name, permissions, main_module, worker_id, options);
    worker.bootstrap(&bootstrap_options);
    (worker, handle)
  }

  pub fn from_options(
    name: String,
    permissions: Permissions,
    main_module: ModuleSpecifier,
    worker_id: WorkerId,
    mut options: WebWorkerOptions,
  ) -> (Self, SendableWebWorkerHandle) {
    // Permissions: many ops depend on this
    let unstable = options.bootstrap.unstable;
    let enable_testing_features = options.bootstrap.enable_testing_features;
    let perm_ext = Extension::builder()
      .state(move |state| {
        state.put::<Permissions>(permissions.clone());
        state.put(ops::UnstableChecker { unstable });
        state.put(ops::TestingFeaturesEnabled(enable_testing_features));
        Ok(())
      })
      .build();

    let mut extensions: Vec<Extension> = vec![
      // Web APIs
      deno_webidl::init(),
      deno_console::init(),
      deno_url::init(),
      deno_web::init(options.blob_store.clone(), Some(main_module.clone())),
      deno_fetch::init::<Permissions>(deno_fetch::Options {
        user_agent: options.user_agent.clone(),
        root_cert_store: options.root_cert_store.clone(),
        unsafely_ignore_certificate_errors: options
          .unsafely_ignore_certificate_errors
          .clone(),
        file_fetch_handler: Rc::new(deno_fetch::FsFetchHandler),
        ..Default::default()
      }),
      deno_websocket::init::<Permissions>(
        options.user_agent.clone(),
        options.root_cert_store.clone(),
        options.unsafely_ignore_certificate_errors.clone(),
      ),
      deno_broadcast_channel::init(options.broadcast_channel.clone(), unstable),
      deno_crypto::init(options.seed),
      deno_webgpu::init(unstable),
      deno_timers::init::<Permissions>(),
      // ffi
      deno_ffi::init::<Permissions>(unstable),
      // Permissions ext (worker specific state)
      perm_ext,
    ];

    // Runtime ops that are always initialized for WebWorkers
    let runtime_exts = vec![
      ops::web_worker::init(),
      ops::runtime::init(main_module.clone()),
      ops::worker_host::init(options.create_web_worker_cb.clone()),
      ops::io::init(),
    ];

    // Extensions providing Deno.* features
    let deno_ns_exts = if options.use_deno_namespace {
      vec![
        ops::fs_events::init(),
        ops::fs::init(),
        deno_tls::init(),
        deno_net::init::<Permissions>(
          options.root_cert_store.clone(),
          unstable,
          options.unsafely_ignore_certificate_errors.clone(),
        ),
        ops::os::init(Some(options.maybe_exit_code.expect(
          "Worker has access to OS ops but exit code was not passed.",
        ))),
        ops::permissions::init(),
        ops::process::init(),
        ops::signal::init(),
        ops::tty::init(),
        deno_http::init(),
        ops::http::init(),
        ops::io::init_stdio(),
      ]
    } else {
      vec![]
    };

    // Append exts
    extensions.extend(runtime_exts);
    extensions.extend(deno_ns_exts); // May be empty
    extensions.extend(std::mem::take(&mut options.extensions));

    let mut js_runtime = JsRuntime::new(RuntimeOptions {
      module_loader: Some(options.module_loader.clone()),
      startup_snapshot: Some(js::deno_isolate_init()),
      js_error_create_fn: options.js_error_create_fn.clone(),
      get_error_class_fn: options.get_error_class_fn,
      shared_array_buffer_store: options.shared_array_buffer_store.clone(),
      compiled_wasm_module_store: options.compiled_wasm_module_store.clone(),
      extensions,
      ..Default::default()
    });

    if let Some(server) = options.maybe_inspector_server.clone() {
      server.register_inspector(
        main_module.to_string(),
        &mut js_runtime,
        false,
      );
    }

    let (internal_handle, external_handle) = {
      let handle = js_runtime.v8_isolate().thread_safe_handle();
      let (internal_handle, external_handle) =
        create_handles(handle, options.worker_type);
      let op_state = js_runtime.op_state();
      let mut op_state = op_state.borrow_mut();
      op_state.put(internal_handle.clone());
      (internal_handle, external_handle)
    };

    (
      Self {
        id: worker_id,
        js_runtime,
        name,
        internal_handle,
        use_deno_namespace: options.use_deno_namespace,
        worker_type: options.worker_type,
        main_module,
      },
      external_handle,
    )
  }

  pub fn bootstrap(&mut self, options: &BootstrapOptions) {
    // Instead of using name for log we use `worker-${id}` because
    // WebWorkers can have empty string as name.
    let script = format!(
      "bootstrap.workerRuntime({}, \"{}\", {}, \"{}\")",
      options.as_json(),
      self.name,
      self.use_deno_namespace,
      self.id
    );
    self
      .execute_script(&located_script_name!(), &script)
      .expect("Failed to execute worker bootstrap script");
  }

  /// See [JsRuntime::execute_script](deno_core::JsRuntime::execute_script)
  pub fn execute_script(
    &mut self,
    name: &str,
    source_code: &str,
  ) -> Result<(), AnyError> {
    self.js_runtime.execute_script(name, source_code)?;
    Ok(())
  }

  /// Loads and instantiates specified JavaScript module
  /// as "main" or "side" module.
  pub async fn preload_module(
    &mut self,
    module_specifier: &ModuleSpecifier,
    main: bool,
  ) -> Result<ModuleId, AnyError> {
    if main {
      self
        .js_runtime
        .load_main_module(module_specifier, None)
        .await
    } else {
      self
        .js_runtime
        .load_side_module(module_specifier, None)
        .await
    }
  }

  /// Loads, instantiates and executes specified JavaScript module.
  pub async fn execute_main_module(
    &mut self,
    module_specifier: &ModuleSpecifier,
  ) -> Result<(), AnyError> {
    let id = self.preload_module(module_specifier, true).await?;
    let mut receiver = self.js_runtime.mod_evaluate(id);
    tokio::select! {
      maybe_result = &mut receiver => {
        debug!("received worker module evaluate {:#?}", maybe_result);
        // If `None` is returned it means that runtime was destroyed before
        // evaluation was complete. This can happen in Web Worker when `self.close()`
        // is called at top level.
        maybe_result.unwrap_or(Ok(()))
      }

      event_loop_result = self.run_event_loop(false) => {
        if self.internal_handle.is_terminated() {
           return Ok(());
        }
        event_loop_result?;
        let maybe_result = receiver.await;
        maybe_result.unwrap_or(Ok(()))
      }
    }
  }

  fn poll_event_loop(
    &mut self,
    cx: &mut Context,
    wait_for_inspector: bool,
  ) -> Poll<Result<(), AnyError>> {
    // If awakened because we are terminating, just return Ok
    if self.internal_handle.terminate_if_needed() {
      return Poll::Ready(Ok(()));
    }

    self.internal_handle.terminate_waker.register(cx.waker());

    match self.js_runtime.poll_event_loop(cx, wait_for_inspector) {
      Poll::Ready(r) => {
        // If js ended because we are terminating, just return Ok
        if self.internal_handle.terminate_if_needed() {
          return Poll::Ready(Ok(()));
        }

        if let Err(e) = r {
          return Poll::Ready(Err(e));
        }

        panic!(
          "coding error: either js is polling or the worker is terminated"
        );
      }
      Poll::Pending => Poll::Pending,
    }
  }

  pub async fn run_event_loop(
    &mut self,
    wait_for_inspector: bool,
  ) -> Result<(), AnyError> {
    poll_fn(|cx| self.poll_event_loop(cx, wait_for_inspector)).await
  }
}

fn print_worker_error(error_str: String, name: &str) {
  eprintln!(
    "{}: Uncaught (in worker \"{}\") {}",
    colors::red_bold("error"),
    name,
    error_str.trim_start_matches("Uncaught "),
  );
}

/// This function should be called from a thread dedicated to this worker.
// TODO(bartlomieju): check if order of actions is aligned to Worker spec
pub fn run_web_worker(
  mut worker: WebWorker,
  specifier: ModuleSpecifier,
  maybe_source_code: Option<String>,
) -> Result<(), AnyError> {
  let name = worker.name.to_string();

  // TODO(bartlomieju): run following block using "select!"
  // with terminate

  let fut = async move {
    // Execute provided source code immediately
    let result = if let Some(source_code) = maybe_source_code {
      worker.execute_script(&located_script_name!(), &source_code)
    } else {
      // TODO(bartlomieju): add "type": "classic", ie. ability to load
      // script instead of module
      worker.execute_main_module(&specifier).await
    };

    let internal_handle = worker.internal_handle.clone();

    // If sender is closed it means that worker has already been closed from
    // within using "globalThis.close()"
    if internal_handle.is_terminated() {
      return Ok(());
    }

    let result = if result.is_ok() {
      worker.run_event_loop(true).await
    } else {
      result
    };

    if let Err(e) = result {
      print_worker_error(e.to_string(), &name);
      internal_handle
        .post_event(WorkerControlEvent::TerminalError(e))
        .expect("Failed to post message to host");

      // Failure to execute script is a terminal error, bye, bye.
      return Ok(());
    }

    debug!("Worker thread shuts down {}", &name);
    result
  };
  run_basic(fut)
}
