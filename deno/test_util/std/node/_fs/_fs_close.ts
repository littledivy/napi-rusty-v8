// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import type { CallbackWithError } from "./_fs_common.ts";

export function close(fd: number, callback: CallbackWithError): void {
  setTimeout(() => {
    let error = null;
    try {
      Deno.close(fd);
    } catch (err) {
      error = err instanceof Error ? err : new Error("[non-error thrown]");
    }
    callback(error);
  }, 0);
}

export function closeSync(fd: number): void {
  Deno.close(fd);
}
