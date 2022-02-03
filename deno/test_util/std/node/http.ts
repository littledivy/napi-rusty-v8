import { core } from "./_core.ts";
import { _normalizeArgs, ListenOptions } from "./net.ts";
import { Buffer } from "./buffer.ts";
import { ERR_SERVER_NOT_RUNNING } from "./_errors.ts";
import { EventEmitter } from "./events.ts";
import { nextTick } from "./_next_tick.ts";
import { Status as STATUS_CODES } from "../http/http_status.ts";
import { validatePort } from "./internal/validators.js";
import {
  Readable as NodeReadable,
  Writable as NodeWritable,
} from "./stream.ts";
import { OutgoingMessage } from "./_http_outgoing.ts";
import { Agent } from "./_http_agent.js";
import { urlToHttpOptions } from "./internal/url.ts";

const METHODS = [
  "ACL",
  "BIND",
  "CHECKOUT",
  "CONNECT",
  "COPY",
  "DELETE",
  "GET",
  "HEAD",
  "LINK",
  "LOCK",
  "M-SEARCH",
  "MERGE",
  "MKACTIVITY",
  "MKCALENDAR",
  "MKCOL",
  "MOVE",
  "NOTIFY",
  "OPTIONS",
  "PATCH",
  "POST",
  "PROPFIND",
  "PROPPATCH",
  "PURGE",
  "PUT",
  "REBIND",
  "REPORT",
  "SEARCH",
  "SOURCE",
  "SUBSCRIBE",
  "TRACE",
  "UNBIND",
  "UNLINK",
  "UNLOCK",
  "UNSUBSCRIBE",
];

type Chunk = string | Buffer | Uint8Array;

function chunkToU8(chunk: Chunk): Uint8Array {
  if (typeof chunk === "string") {
    return core.encode(chunk);
  }
  return chunk;
}

export interface RequestOptions {
  agent?: Agent;
  auth?: string;
  createConnection?: () => unknown;
  defaultPort?: number;
  family?: number;
  headers?: Record<string, string>;
  hints?: number;
  host?: string;
  hostname?: string;
  insecureHTTPParser?: boolean;
  localAddress?: string;
  localPort?: number;
  lookup?: () => void;
  maxHeaderSize?: number;
  method?: string;
  path?: string;
  port?: number;
  protocol?: string;
  setHost?: boolean;
  socketPath?: string;
  timeout?: number;
  signal?: AbortSignal;
  href?: string;
}

/** ClientRequest represents the http(s) request from the client */
class ClientRequest extends NodeWritable {
  body: null | ReadableStream = null;
  controller: ReadableStreamDefaultController | null = null;
  constructor(
    public opts: RequestOptions,
    public cb: (res: IncomingMessageForClient) => void,
  ) {
    super();
  }

  // deno-lint-ignore no-explicit-any
  _write(chunk: any, _enc: string, cb: () => void) {
    if (this.controller) {
      this.controller.enqueue(chunk);
      cb();
      return;
    }

    this.body = new ReadableStream({
      start: (controller) => {
        this.controller = controller;
        controller.enqueue(chunk);
        cb();
      },
    });
  }

  async _final() {
    const client = await this._createCustomClient();
    const opts = { body: this.body, method: this.opts.method, client };
    const res = new IncomingMessageForClient(
      await fetch(this.opts.href!, opts),
    );
    if (client) {
      res.on("end", () => {
        client.close();
      });
    }
    this.cb(res);
  }

  _createCustomClient(): Promise<Deno.HttpClient | undefined> {
    return Promise.resolve(undefined);
  }
}

/** IncomingMessage for http(s) client */
export class IncomingMessageForClient extends NodeReadable {
  reader: ReadableStreamDefaultReader | undefined;
  constructor(public resp: Response) {
    super();
    this.reader = resp.body?.getReader();
  }

  async _read(_size: number) {
    if (this.reader === undefined) {
      this.push(null);
      return;
    }
    try {
      const res = await this.reader.read();
      if (res.done) {
        this.push(null);
        return;
      }
      this.push(res.value);
    } catch (e) {
      // deno-lint-ignore no-explicit-any
      this.destroy(e as any);
    }
  }

  get headers() {
    return this.resp.headers;
  }

  get statusCode() {
    return this.resp.status;
  }

  get statusMessage() {
    return this.resp.statusText;
  }
}

export class ServerResponse extends NodeWritable {
  statusCode?: number = undefined;
  statusMessage?: string = undefined;
  #headers = new Headers({});
  private readable: ReadableStream;
  headersSent = false;
  #reqEvent: Deno.RequestEvent;
  #firstChunk: Chunk | null = null;

  constructor(reqEvent: Deno.RequestEvent) {
    let controller: ReadableByteStreamController;
    const readable = new ReadableStream({
      start(c) {
        controller = c as ReadableByteStreamController;
      },
    });
    super({
      autoDestroy: true,
      defaultEncoding: "utf-8",
      emitClose: true,
      write: (chunk, _encoding, cb) => {
        if (!this.headersSent) {
          if (this.#firstChunk === null) {
            this.#firstChunk = chunk;
            return cb();
          } else {
            controller.enqueue(chunkToU8(this.#firstChunk));
            this.#firstChunk = null;
            this.respond(false);
          }
        }
        controller.enqueue(chunkToU8(chunk));
        return cb();
      },
      final: (cb) => {
        if (this.#firstChunk) {
          this.respond(true, this.#firstChunk);
        } else if (!this.headersSent) {
          this.respond(true);
        }
        controller.close();
        return cb();
      },
      destroy: (err, cb) => {
        if (err) {
          controller.error(err);
        }
        return cb(null);
      },
    });
    this.readable = readable;
    this.#reqEvent = reqEvent;
  }

  setHeader(name: string, value: string) {
    this.#headers.set(name, value);
    return this;
  }

  getHeader(name: string) {
    return this.#headers.get(name);
  }
  removeHeader(name: string) {
    return this.#headers.delete(name);
  }
  getHeaderNames() {
    return Array.from(this.#headers.keys());
  }
  hasHeader(name: string) {
    return this.#headers.has(name);
  }

  writeHead(status: number, headers: Record<string, string>) {
    this.statusCode = status;
    for (const k in headers) {
      this.#headers.set(k, headers[k]);
    }
    return this;
  }

  #ensureHeaders(singleChunk?: Chunk) {
    if (this.statusCode === undefined) {
      this.statusCode = 200;
      this.statusMessage = "OK";
    }
    if (typeof singleChunk === "string" && !this.hasHeader("content-type")) {
      this.setHeader("content-type", "text/plain;charset=UTF-8");
    }
  }

  respond(final: boolean, singleChunk?: Chunk) {
    this.headersSent = true;
    this.#ensureHeaders(singleChunk);
    const body = singleChunk ?? (final ? null : this.readable);
    this.#reqEvent.respondWith(
      new Response(body, {
        headers: this.#headers,
        status: this.statusCode,
        statusText: this.statusMessage,
      }),
    );
  }

  // deno-lint-ignore no-explicit-any
  end(chunk?: any, encoding?: any, cb?: any): this {
    if (!chunk && this.#headers.has("transfer-encoding")) {
      // FIXME(bnoordhuis) Node sends a zero length chunked body instead, i.e.,
      // the trailing "0\r\n", but respondWith() just hangs when I try that.
      this.#headers.set("content-length", "0");
      this.#headers.delete("transfer-encoding");
    }

    // @ts-expect-error The signature for cb is stricter than the one implemented here
    return super.end(chunk, encoding, cb);
  }
}

// TODO(@AaronO): optimize
export class IncomingMessageForServer extends NodeReadable {
  private req: Request;
  url: string;

  constructor(req: Request) {
    // Check if no body (GET/HEAD/OPTIONS/...)
    const reader = req.body?.getReader();
    super({
      autoDestroy: true,
      emitClose: true,
      objectMode: false,
      read: async function (_size) {
        if (!reader) {
          return this.push(null);
        }

        try {
          const { value } = await reader!.read();
          this.push(value !== undefined ? Buffer.from(value) : null);
        } catch (err) {
          this.destroy(err as Error);
        }
      },
      destroy: (err, cb) => {
        reader?.cancel().finally(() => cb(err));
      },
    });
    this.req = req;
    // TODO: consider more robust path extraction, e.g:
    // url: (new URL(request.url).pathname),
    this.url = req.url.slice(this.req.url.indexOf("/", 8));
  }

  get aborted() {
    return false;
  }
  get httpVersion() {
    return "1.1";
  }

  get headers() {
    return Object.fromEntries(this.req.headers.entries());
  }
  get method() {
    return this.req.method;
  }
}

type ServerHandler = (
  req: IncomingMessageForServer,
  res: ServerResponse,
) => void;

export function Server(handler?: ServerHandler): ServerImpl {
  return new ServerImpl(handler);
}

class ServerImpl extends EventEmitter {
  #httpConnections: Set<Deno.HttpConn> = new Set();
  #listener?: Deno.Listener;

  constructor(handler?: ServerHandler) {
    super();

    if (handler !== undefined) {
      this.on("request", handler);
    }
  }

  listen(...args: unknown[]): this {
    // TODO(bnoordhuis) Delegate to net.Server#listen().
    const normalized = _normalizeArgs(args);
    const options = normalized[0] as Partial<ListenOptions>;
    const cb = normalized[1];

    if (cb !== null) {
      // @ts-ignore change EventEmitter's sig to use CallableFunction
      this.once("listening", cb);
    }

    let port = 0;
    if (typeof options.port === "number" || typeof options.port === "string") {
      validatePort(options.port, "options.port");
      port = options.port | 0;
    }

    // TODO(bnoordhuis) Node prefers [::] when host is omitted,
    // we on the other hand default to 0.0.0.0.
    const hostname = options.host ?? "";

    this.#listener = Deno.listen({ port, hostname });
    nextTick(() => this.#listenLoop());

    return this;
  }

  async #listenLoop() {
    const go = async (httpConn: Deno.HttpConn) => {
      try {
        for (;;) {
          let reqEvent = null;
          try {
            // Note: httpConn.nextRequest() calls httpConn.close() on error.
            reqEvent = await httpConn.nextRequest();
          } catch {
            // Connection closed.
            // TODO(bnoordhuis) Emit "clientError" event on the http.Server
            // instance? Node emits it when request parsing fails and expects
            // the listener to send a raw 4xx HTTP response on the underlying
            // net.Socket but we don't have one to pass to the listener.
          }
          if (reqEvent === null) {
            break;
          }
          const req = new IncomingMessageForServer(reqEvent.request);
          const res = new ServerResponse(reqEvent);
          this.emit("request", req, res);
        }
      } finally {
        this.#httpConnections.delete(httpConn);
      }
    };

    const listener = this.#listener;

    if (listener !== undefined) {
      this.emit("listening");

      for await (const conn of listener) {
        let httpConn: Deno.HttpConn;
        try {
          httpConn = Deno.serveHttp(conn);
        } catch {
          continue; /// Connection closed.
        }

        this.#httpConnections.add(httpConn);
        go(httpConn);
      }
    }
  }

  get listening() {
    return this.#listener !== undefined;
  }

  close(cb?: (err?: Error) => void): this {
    const listening = this.listening;

    if (typeof cb === "function") {
      if (listening) {
        this.once("close", cb);
      } else {
        this.once("close", function close() {
          cb(new ERR_SERVER_NOT_RUNNING());
        });
      }
    }

    nextTick(() => this.emit("close"));

    if (listening) {
      this.#listener!.close();
      this.#listener = undefined;

      for (const httpConn of this.#httpConnections) {
        try {
          httpConn.close();
        } catch {
          // Already closed.
        }
      }

      this.#httpConnections.clear();
    }

    return this;
  }

  address() {
    const addr = this.#listener!.addr as Deno.NetAddr;
    return {
      port: addr.port,
      address: addr.hostname,
    };
  }
}

Server.prototype = ServerImpl.prototype;

export function createServer(handler?: ServerHandler) {
  return Server(handler);
}

/** Makes an HTTP request. */
export function request(
  url: string | URL,
  cb?: (res: IncomingMessageForClient) => void,
): ClientRequest;
export function request(
  opts: RequestOptions,
  cb?: (res: IncomingMessageForClient) => void,
): ClientRequest;
export function request(
  url: string | URL,
  opts: RequestOptions,
  cb?: (res: IncomingMessageForClient) => void,
): ClientRequest;
// deno-lint-ignore no-explicit-any
export function request(...args: any[]) {
  let options = {};
  if (typeof args[0] === "string") {
    options = urlToHttpOptions(new URL(args.shift()));
  } else if (args[0] instanceof URL) {
    options = urlToHttpOptions(args.shift());
  }
  if (args[0] && typeof args[0] !== "function") {
    Object.assign(options, args.shift());
  }
  args.unshift(options);
  return new ClientRequest(args[0], args[1]);
}

/** Makes a `GET` HTTP request. */
export function get(
  url: string | URL,
  cb?: (res: IncomingMessageForClient) => void,
): ClientRequest;
export function get(
  opts: RequestOptions,
  cb?: (res: IncomingMessageForClient) => void,
): ClientRequest;
export function get(
  url: string | URL,
  opts: RequestOptions,
  cb?: (res: IncomingMessageForClient) => void,
): ClientRequest;
// deno-lint-ignore no-explicit-any
export function get(...args: any[]) {
  const req = request(args[0], args[1], args[2]);
  req.end();
  return req;
}

export { Agent, ClientRequest, METHODS, OutgoingMessage, STATUS_CODES };
export default {
  Agent,
  ClientRequest,
  STATUS_CODES,
  METHODS,
  createServer,
  Server,
  IncomingMessage: IncomingMessageForServer,
  OutgoingMessage,
  ServerResponse,
  request,
  get,
};
