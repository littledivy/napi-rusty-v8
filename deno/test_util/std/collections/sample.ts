// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { randomInteger } from "./_utils.ts";

/**
 * Returns a random element from the given array.
 *
 * Example:
 *
 * ```ts
 * import { sample } from "https://deno.land/std@$STD_VERSION/collections/mod.ts"
 * import { assert } from "https://deno.land/std@$STD_VERSION/testing/asserts.ts";
 *
 * const numbers = [1, 2, 3, 4];
 * const random = sample(numbers);
 *
 * assert(numbers.includes(random as number));
 * ```
 */
export function sample<T>(array: readonly T[]): T | undefined {
  const length = array.length;
  return length ? array[randomInteger(0, length - 1)] : undefined;
}
