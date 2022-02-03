// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

/**
 * Returns all distinct elements in the given array, preserving order by first occurrence
 *
 * Example:
 *
 * ```ts
 * import { distinct } from "https://deno.land/std@$STD_VERSION/collections/mod.ts";
 * import { assertEquals } from "https://deno.land/std@$STD_VERSION/testing/asserts.ts";
 *
 * const numbers = [ 3, 2, 5, 2, 5 ]
 * const distinctNumbers = distinct(numbers)
 *
 * assertEquals(distinctNumbers, [ 3, 2, 5 ])
 * ```
 */
export function distinct<T>(array: readonly T[]): T[] {
  const set = new Set(array);

  return Array.from(set);
}
