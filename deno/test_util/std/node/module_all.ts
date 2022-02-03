// This aliases are used in some node tests and represent a legacy alias
// for the stream modules
// deno-lint-ignore camelcase
import _http_agent from "./_http_agent.js";
// deno-lint-ignore camelcase
import _http_outgoing from "./_http_outgoing.ts";
// deno-lint-ignore camelcase
import _stream_duplex from "./internal/streams/duplex.js";
// deno-lint-ignore camelcase
import _stream_passthrough from "./internal/streams/passthrough.js";
// deno-lint-ignore camelcase
import _stream_readable from "./internal/streams/readable.js";
// deno-lint-ignore camelcase
import _stream_transform from "./internal/streams/transform.js";
// deno-lint-ignore camelcase
import _stream_writable from "./internal/streams/writable.js";

import assert from "./assert.ts";
import assertStrict from "./assert/strict.ts";
// deno-lint-ignore camelcase
import async_hooks from "./async_hooks.ts";
import buffer from "./buffer.ts";
import childProcess from "./child_process.ts";
import cluster from "./cluster.ts";
import console from "./console.ts";
import constants from "./constants.ts";
import crypto from "./crypto.ts";
import dgram from "./dgram.ts";
import dns from "./dns.ts";
import domain from "./domain.ts";
import events from "./events.ts";
import fs from "./fs.ts";
import fsPromises from "./fs/promises.ts";
import internalFsUtils from "./internal/fs/utils.js";
import http from "./http.ts";
import http2 from "./http2.ts";
import https from "./https.ts";
import inspector from "./inspector.ts";
import internalErrors from "./internal/errors.js";
import internalHttp from "./internal/http.ts";
import internalReadlineUtils from "./internal/readline/utils.js";
import internalStreamsAddAbortSignal from "./internal/streams/add-abort-signal.js";
import internalStreamsAddBufferList from "./internal/streams/buffer_list.js";
import internalStreamsState from "./internal/streams/state.js";
import internalTestBinding from "./internal/test/binding.ts";
import internalTimers from "./internal/timers.ts";
import internalUtilInspect from "./internal/util/inspect.js";
import internalUtil from "./internal/util.js";
import net from "./net.ts";
import os from "./os.ts";
import path from "./path.ts";
import perfHooks from "./perf_hooks.ts";
import process from "./process.ts";
import querystring from "./querystring.ts";
import readline from "./readline.ts";
import repl from "./repl.ts";
import stream from "./stream.ts";
import streamConsumers from "./stream/consumers.js";
import streamPromises from "./stream/promises.js";
import streamWeb from "./stream/web.ts";
import stringDecoder from "./string_decoder.ts";
import sys from "./sys.ts";
import timers from "./timers.ts";
import timersPromises from "./timers/promises.ts";
import tls from "./tls.ts";
import tty from "./tty.ts";
import url from "./url.ts";
import util from "./util.ts";
import v8 from "./v8.ts";
import vm from "./vm.ts";
import workerThreads from "./worker_threads.ts";
import wasi from "./wasi.ts";
import zlib from "./zlib.ts";

// Canonical mapping of supported modules
export default {
  _http_agent,
  _http_outgoing,
  _stream_duplex,
  _stream_passthrough,
  _stream_readable,
  _stream_transform,
  _stream_writable,
  assert,
  "assert/strict": assertStrict,
  async_hooks,
  buffer,
  crypto,
  console,
  constants,
  "child_process": childProcess,
  cluster,
  dgram,
  dns,
  domain,
  events,
  fs,
  "fs/promises": fsPromises,
  http,
  http2,
  https,
  inspector,
  "internal/errors": internalErrors,
  "internal/fs/utils": internalFsUtils,
  "internal/http": internalHttp,
  "internal/readline/utils": internalReadlineUtils,
  "internal/streams/add-abort-signal": internalStreamsAddAbortSignal,
  "internal/streams/buffer_list": internalStreamsAddBufferList,
  "internal/streams/state": internalStreamsState,
  "internal/test/binding": internalTestBinding,
  "internal/timers": internalTimers,
  "internal/util/inspect": internalUtilInspect,
  "internal/util": internalUtil,
  net,
  os,
  path,
  "perf_hooks": perfHooks,
  process,
  querystring,
  readline,
  repl,
  stream,
  "stream/consumers": streamConsumers,
  "stream/promises": streamPromises,
  "stream/web": streamWeb,
  "string_decoder": stringDecoder,
  sys,
  timers,
  "timers/promises": timersPromises,
  tls,
  tty,
  url,
  util,
  v8,
  vm,
  wasi,
  workerThreads,
  zlib,
} as Record<string, unknown>;
