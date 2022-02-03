// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import { bytesToUuid, uuidToBytes } from "./_common.ts";
import { concat } from "../bytes/mod.ts";
import { assert } from "../_util/assert.ts";

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

/**
 * Validate that the passed UUID is an RFC4122 v5 UUID.
 *
 * ```ts
 * import { generate as generateV5, validate } from "./v5.ts";
 *
 * validate(await generateV5("6ba7b810-9dad-11d1-80b4-00c04fd430c8", new Uint8Array())); // true
 * validate(crypto.randomUUID()); // false
 * validate("this-is-not-a-uuid"); // false
 * ```
 */
export function validate(id: string): boolean {
  return UUID_RE.test(id);
}

/**
 * Generate a RFC4122 v5 UUID (SHA-1 namespace).
 *
 * ```js
 * import { generate } from "./v5.ts";
 *
 * const NAMESPACE_URL = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
 *
 * const uuid = await generate(NAMESPACE_URL, new TextEncoder().encode("python.org"));
 * uuid === "886313e1-3b8a-5372-9b90-0c9aee199e5d" // true
 * ```
 *
 * @param namespace The namespace to use, encoded as a UUID.
 * @param data The data to hash to calculate the SHA-1 digest for the UUID.
 */
export async function generate(
  namespace: string,
  data: Uint8Array,
): Promise<string> {
  // TODO(lucacasonato): validate that `namespace` is a valid UUID.

  const space = uuidToBytes(namespace);
  assert(space.length === 16, "namespace must be a valid UUID");

  const toHash = concat(new Uint8Array(space), data);
  const buffer = await crypto.subtle.digest("sha-1", toHash);
  const bytes = new Uint8Array(buffer);

  bytes[6] = (bytes[6] & 0x0f) | 0x50;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;

  return bytesToUuid(bytes);
}
