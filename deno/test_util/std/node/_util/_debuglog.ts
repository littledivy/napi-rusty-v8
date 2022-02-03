import { sprintf } from "../../fmt/printf.ts";
import { inspect } from "../util.ts";

// `debugImpls` and `testEnabled` are deliberately not initialized so any call
// to `debuglog()` before `initializeDebugEnv()` is called will throw.
let debugImpls: Record<string, (...args: unknown[]) => void>;
let testEnabled: (str: string) => boolean;

// `debugEnv` is initial value of process.env.NODE_DEBUG
function initializeDebugEnv(debugEnv: string) {
  debugImpls = Object.create(null);
  if (debugEnv) {
    // This is run before any user code, it's OK not to use primordials.
    debugEnv = debugEnv.replace(/[|\\{}()[\]^$+?.]/g, "\\$&")
      .replaceAll("*", ".*")
      .replaceAll(",", "$|^");
    const debugEnvRegex = new RegExp(`^${debugEnv}$`, "i");
    testEnabled = (str) => debugEnvRegex.exec(str) !== null;
  } else {
    testEnabled = () => false;
  }
}

// Emits warning when user sets
// NODE_DEBUG=http or NODE_DEBUG=http2.
function emitWarningIfNeeded(set: string) {
  if ("HTTP" === set || "HTTP2" === set) {
    console.warn(
      "Setting the NODE_DEBUG environment variable " +
        "to '" + set.toLowerCase() + "' can expose sensitive " +
        "data (such as passwords, tokens and authentication headers) " +
        "in the resulting log.",
    );
  }
}

const noop = () => {};

function debuglogImpl(
  enabled: boolean,
  set: string,
): (...args: unknown[]) => void {
  if (debugImpls[set] === undefined) {
    if (enabled) {
      emitWarningIfNeeded(set);
      debugImpls[set] = function debug(...args: unknown[]) {
        const msg = args.map((arg) => inspect(arg)).join(" ");
        console.error(sprintf("%s %s: %s\n", set, String(Deno.pid), msg));
      };
    } else {
      debugImpls[set] = noop;
    }
  }

  return debugImpls[set];
}

// debuglogImpl depends on process.pid and process.env.NODE_DEBUG,
// so it needs to be called lazily in top scopes of internal modules
// that may be loaded before these run time states are allowed to
// be accessed.
function debuglog(
  set: string,
  cb: (debug: (...args: unknown[]) => void) => void,
) {
  function init() {
    set = set.toUpperCase();
    enabled = testEnabled(set);
  }

  let debug = (...args: unknown[]): void => {
    init();
    // Only invokes debuglogImpl() when the debug function is
    // called for the first time.
    debug = debuglogImpl(enabled, set);

    if (typeof cb === "function") {
      cb(debug);
    }

    return debug(...args);
  };

  let enabled: boolean;
  let test = () => {
    init();
    test = () => enabled;
    return enabled;
  };

  const logger = (...args: unknown[]) => debug(...args);

  Object.defineProperty(logger, "enabled", {
    get() {
      return test();
    },
    configurable: true,
    enumerable: true,
  });

  return logger;
}

let state = "";

if (Deno.permissions) {
  state = (await Deno.permissions.query({
    name: "env",
    variable: "NODE_DEBUG",
  })).state;
}

if (state === "granted") {
  initializeDebugEnv(Deno.env.get("NODE_DEBUG") ?? "");
} else {
  initializeDebugEnv("");
}

export default { debuglog };

export { debuglog };
