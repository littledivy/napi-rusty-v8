// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
// Copyright Joyent, Inc. and Node.js contributors. All rights reserved. MIT license.
import { warnNotImplemented } from "./_utils.ts";
import { EventEmitter } from "./events.ts";
import { validateString } from "./internal/validators.js";
import { ERR_INVALID_ARG_TYPE } from "./_errors.ts";
import { getOptionValue } from "./_options.ts";
import { assert } from "../_util/assert.ts";
import { fromFileUrl } from "../path/mod.ts";
import {
  _exiting,
  arch,
  chdir,
  cwd,
  env,
  nextTick as _nextTick,
  pid,
  platform,
  version,
  versions,
} from "./_process/process.ts";
export {
  _nextTick as nextTick,
  arch,
  chdir,
  cwd,
  env,
  pid,
  platform,
  version,
  versions,
};
import { stderr, stdin, stdout } from "./_process/streams.ts";
export { stderr, stdin, stdout };
import { getBinding } from "./internal_binding/mod.ts";
import type { BindingName } from "./internal_binding/mod.ts";

const notImplementedEvents = [
  "beforeExit",
  "disconnect",
  "message",
  "multipleResolves",
  "rejectionHandled",
  "uncaughtException",
  "uncaughtExceptionMonitor",
  "unhandledRejection",
];

// The first 2 items are placeholders.
// They will be overwritten by the below Object.defineProperty calls.
const argv = ["", "", ...Deno.args];
// Overwrites the 1st item with getter.
Object.defineProperty(argv, "0", { get: Deno.execPath });
// Overwrites the 2st item with getter.
Object.defineProperty(argv, "1", { get: () => fromFileUrl(Deno.mainModule) });

/** https://nodejs.org/api/process.html#process_process_exit_code */
export const exit = (code?: number) => {
  if (code || code === 0) {
    process.exitCode = code;
  }

  if (!process._exiting) {
    process._exiting = true;
    process.emit("exit", process.exitCode || 0);
  }

  Deno.exit(process.exitCode || 0);
};

function addReadOnlyProcessAlias(
  name: string,
  option: string,
  enumerable = true,
) {
  const value = getOptionValue(option);

  if (value) {
    Object.defineProperty(process, name, {
      writable: false,
      configurable: true,
      enumerable,
      value,
    });
  }
}

function createWarningObject(
  warning: string,
  type: string,
  code?: string,
  // deno-lint-ignore ban-types
  ctor?: Function,
  detail?: string,
): Error {
  assert(typeof warning === "string");

  // deno-lint-ignore no-explicit-any
  const warningErr: any = new Error(warning);
  warningErr.name = String(type || "Warning");

  if (code !== undefined) {
    warningErr.code = code;
  }
  if (detail !== undefined) {
    warningErr.detail = detail;
  }

  // @ts-ignore this function is not available in lib.dom.d.ts
  Error.captureStackTrace(warningErr, ctor || process.emitWarning);

  return warningErr;
}

function doEmitWarning(warning: Error) {
  process.emit("warning", warning);
}

/** https://nodejs.org/api/process.html#process_process_emitwarning_warning_options */
export function emitWarning(
  warning: string | Error,
  type:
    // deno-lint-ignore ban-types
    | { type: string; detail: string; code: string; ctor: Function }
    | string
    | null,
  code?: string,
  // deno-lint-ignore ban-types
  ctor?: Function,
) {
  let detail;

  if (type !== null && typeof type === "object" && !Array.isArray(type)) {
    ctor = type.ctor;
    code = type.code;

    if (typeof type.detail === "string") {
      detail = type.detail;
    }

    type = type.type || "Warning";
  } else if (typeof type === "function") {
    ctor = type;
    code = undefined;
    type = "Warning";
  }

  if (type !== undefined) {
    validateString(type, "type");
  }

  if (typeof code === "function") {
    ctor = code;
    code = undefined;
  } else if (code !== undefined) {
    validateString(code, "code");
  }

  if (typeof warning === "string") {
    warning = createWarningObject(warning, type as string, code, ctor, detail);
  } else if (!(warning instanceof Error)) {
    throw new ERR_INVALID_ARG_TYPE("warning", ["Error", "string"], warning);
  }

  if (warning.name === "DeprecationWarning") {
    // deno-lint-ignore no-explicit-any
    if ((process as any).noDeprecation) {
      return;
    }

    // deno-lint-ignore no-explicit-any
    if ((process as any).throwDeprecation) {
      // Delay throwing the error to guarantee that all former warnings were
      // properly logged.
      return process.nextTick(() => {
        throw warning;
      });
    }
  }

  process.nextTick(doEmitWarning, warning);
}

function hrtime(time?: [number, number]): [number, number] {
  const milli = performance.now();
  const sec = Math.floor(milli / 1000);
  const nano = Math.floor(milli * 1_000_000 - sec * 1_000_000_000);
  if (!time) {
    return [sec, nano];
  }
  const [prevSec, prevNano] = time;
  return [sec - prevSec, nano - prevNano];
}

hrtime.bigint = function (): BigInt {
  const [sec, nano] = hrtime();
  return BigInt(sec) * 1_000_000_000n + BigInt(nano);
};

function memoryUsage(): {
  rss: number;
  heapTotal: number;
  heapUsed: number;
  external: number;
  arrayBuffers: number;
} {
  return {
    ...Deno.memoryUsage(),
    arrayBuffers: 0,
  };
}

memoryUsage.rss = function (): number {
  return memoryUsage().rss;
};

class Process extends EventEmitter {
  constructor() {
    super();

    globalThis.addEventListener("unload", () => {
      if (!process._exiting) {
        process._exiting = true;
        super.emit("exit", process.exitCode || 0);
      }
    });
  }

  /** https://nodejs.org/api/process.html#process_process_arch */
  arch = arch;

  /**
   * https://nodejs.org/api/process.html#process_process_argv
   * Read permissions are required in order to get the executable route
   */
  argv = argv;

  /** https://nodejs.org/api/process.html#process_process_chdir_directory */
  chdir = chdir;

  /** https://nodejs.org/api/process.html#processconfig */
  config = {
    target_defaults: {},
    variables: {},
  };

  /** https://nodejs.org/api/process.html#process_process_cwd */
  cwd = cwd;

  /**
   * https://nodejs.org/api/process.html#process_process_env
   * Requires env permissions
   */
  env = env;

  /** https://nodejs.org/api/process.html#process_process_execargv */
  execArgv: string[] = [];

  /** https://nodejs.org/api/process.html#process_process_exit_code */
  exit = exit;

  _exiting = _exiting;

  /** https://nodejs.org/api/process.html#processexitcode_1 */
  exitCode: undefined | number = undefined;

  // Typed as any to avoid importing "module" module for types
  // deno-lint-ignore no-explicit-any
  mainModule: any = undefined;

  /** https://nodejs.org/api/process.html#process_process_nexttick_callback_args */
  nextTick = _nextTick;

  /** https://nodejs.org/api/process.html#process_process_events */
  on(event: "exit", listener: (code: number) => void): this;
  // deno-lint-ignore no-explicit-any
  on(event: string, listener: (...args: any[]) => void): this;
  // deno-lint-ignore ban-types
  on(event: typeof notImplementedEvents[number], listener: Function): this;
  // deno-lint-ignore no-explicit-any
  on(event: string, listener: (...args: any[]) => void): this {
    if (notImplementedEvents.includes(event)) {
      warnNotImplemented(`process.on("${event}")`);
    } else if (event.startsWith("SIG")) {
      Deno.addSignalListener(event as Deno.Signal, listener);
    } else {
      super.on(event, listener);
    }

    return this;
  }

  off(event: "exit", listener: (code: number) => void): this;
  // deno-lint-ignore no-explicit-any
  off(event: string, listener: (...args: any[]) => void): this;
  // deno-lint-ignore ban-types
  off(event: typeof notImplementedEvents[number], listener: Function): this;
  // deno-lint-ignore no-explicit-any
  off(event: string, listener: (...args: any[]) => void): this {
    if (notImplementedEvents.includes(event)) {
      warnNotImplemented(`process.off("${event}")`);
    } else if (event.startsWith("SIG")) {
      Deno.removeSignalListener(event as Deno.Signal, listener);
    } else {
      super.off(event, listener);
    }

    return this;
  }

  /** https://nodejs.org/api/process.html#process_process_pid */
  pid = pid;

  /** https://nodejs.org/api/process.html#process_process_platform */
  platform = platform;

  // @ts-ignore `deno_std`'s types are scricter than types from DefinitelyTyped for Node.js thus causing problems
  removeAllListeners(eventName?: string | symbol): this {
    // @ts-ignore `deno_std`'s types are scricter than types from DefinitelyTyped for Node.js thus causing problems
    return super.removeAllListeners(eventName);
  }

  // @ts-ignore `deno_std`'s types are scricter than types from DefinitelyTyped for Node.js thus causing problems
  removeListener(
    event: typeof notImplementedEvents[number],
    //deno-lint-ignore ban-types
    listener: Function,
  ): this;
  // @ts-ignore `deno_std`'s types are scricter than types from DefinitelyTyped for Node.js thus causing problems
  removeListener(event: "exit", listener: (code: number) => void): this;
  // @ts-ignore `deno_std`'s types are scricter than types from DefinitelyTyped for Node.js thus causing problems
  //deno-lint-ignore no-explicit-any
  removeListener(event: string, listener: (...args: any[]) => void): this {
    if (notImplementedEvents.includes(event)) {
      warnNotImplemented(`process.removeListener("${event}")`);
      return this;
    }

    super.removeListener("exit", listener);

    return this;
  }

  /**
   * Returns the current high-resolution real time in a [seconds, nanoseconds]
   * tuple.
   *
   * Note: You need to give --allow-hrtime permission to Deno to actually get
   * nanoseconds precision values. If you don't give 'hrtime' permission, the returned
   * values only have milliseconds precision.
   *
   * `time` is an optional parameter that must be the result of a previous process.hrtime() call to diff with the current time.
   *
   * These times are relative to an arbitrary time in the past, and not related to the time of day and therefore not subject to clock drift. The primary use is for measuring performance between intervals.
   * https://nodejs.org/api/process.html#process_process_hrtime_time
   */
  hrtime = hrtime;

  memoryUsage = memoryUsage;

  /** https://nodejs.org/api/process.html#process_process_stderr */
  stderr = stderr;

  /** https://nodejs.org/api/process.html#process_process_stdin */
  stdin = stdin;

  /** https://nodejs.org/api/process.html#process_process_stdout */
  stdout = stdout;

  /** https://nodejs.org/api/process.html#process_process_version */
  version = version;

  /** https://nodejs.org/api/process.html#process_process_versions */
  versions = versions;

  /** https://nodejs.org/api/process.html#process_process_emitwarning_warning_options */
  emitWarning = emitWarning;

  binding(name: BindingName) {
    return getBinding(name);
  }

  /** https://nodejs.org/api/process.html#processumaskmask */
  umask = Deno.umask;

  /** https://nodejs.org/api/process.html#processgetuid */
  getuid(): number {
    // TODO(kt3k): return user id in mac and linux
    return NaN;
  }

  // TODO(kt3k): Implement this when we added -e option to node compat mode
  _eval: string | undefined = undefined;

  /** https://nodejs.org/api/process.html#processexecpath */
  get execPath() {
    return argv[0];
  }
}

/** https://nodejs.org/api/process.html#process_process */
const process = new Process();

Object.defineProperty(process, Symbol.toStringTag, {
  enumerable: false,
  writable: true,
  configurable: false,
  value: "process",
});

addReadOnlyProcessAlias("noDeprecation", "--no-deprecation");
addReadOnlyProcessAlias("throwDeprecation", "--throw-deprecation");

export const removeListener = process.removeListener;
export const removeAllListeners = process.removeAllListeners;

export default process;

//TODO(Soremwar)
//Remove on 1.0
//Kept for backwards compatibility with std
export { process };
