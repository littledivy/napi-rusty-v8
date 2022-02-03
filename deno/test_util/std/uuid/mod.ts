// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

// Based on https://github.com/kelektiv/node-uuid -> https://www.ietf.org/rfc/rfc4122.txt
// Supporting Support for RFC4122 version 1, 4, and 5 UUIDs

import * as v1 from "./v1.ts";
import * as v4 from "./v4.ts";
import * as v5 from "./v5.ts";

export const NIL_UUID = "00000000-0000-0000-0000-000000000000";

/**
 * Check if the passed UUID is the nil UUID.
 *
 * ```js
 * import { isNil } from "./mod.ts";
 *
 * isNil("00000000-0000-0000-0000-000000000000") // true
 * isNil(crypto.randomUUID()) // false
 * ```
 */
export function isNil(id: string): boolean {
  return id === NIL_UUID;
}

/**
 * Test a string to see if it is a valid UUID.
 *
 * ```js
 * import { validate } from "./mod.ts"
 *
 * validate("not a UUID") // false
 * validate("6ec0bd7f-11c0-43da-975e-2a8ad9ebae0b") // true
 * ```
 */
export function validate(uuid: string): boolean {
  return /^(?:[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}|00000000-0000-0000-0000-000000000000)$/i
    .test(
      uuid,
    );
}

/**
 * Detect RFC version of a UUID.
 *
 * ```js
 * import { version } from "./mod.ts"
 *
 * version("d9428888-122b-11e1-b85c-61cd3cbb3210") // 1
 * version("109156be-c4fb-41ea-b1b4-efe1671c5836") // 4
 * ```
 */
export function version(uuid: string): number {
  if (!validate(uuid)) {
    throw TypeError("Invalid UUID");
  }

  return parseInt(uuid[14], 16);
}

export { v1, v4, v5 };
