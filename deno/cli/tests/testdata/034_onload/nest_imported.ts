// deno-lint-ignore-file no-window-prefix
import { assert } from "../../../../test_util/std/testing/asserts.ts";

const handler = (e: Event) => {
  assert(!e.cancelable);
  console.log(`got ${e.type} event in event handler (nest_imported)`);
};

window.addEventListener("load", handler);
window.addEventListener("unload", handler);
console.log("log from nest_imported script");
