// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

/**
 * Returns all elements in the given array that produce a distinct value using the given selector, preserving order by first occurrence
 *
 * Example:
 *
 * ```ts
 * import { distinctBy } from "https://deno.land/std@$STD_VERSION/collections/mod.ts";
 * import { assertEquals } from "https://deno.land/std@$STD_VERSION/testing/asserts.ts";
 *
 * const names = [ 'Anna', 'Kim', 'Arnold', 'Kate' ]
 * const exampleNamesByFirstLetter = distinctBy(names, it => it.charAt(0))
 *
 * assertEquals(exampleNamesByFirstLetter, [ 'Anna', 'Kim' ])
 * ```
 */
export function distinctBy<T, D>(
  array: readonly T[],
  selector: (el: T) => D,
): T[] {
  const selectedValues = new Set<D>();
  const ret: T[] = [];

  for (const element of array) {
    const currentSelectedValue = selector(element);

    if (!selectedValues.has(currentSelectedValue)) {
      selectedValues.add(currentSelectedValue);
      ret.push(element);
    }
  }

  return ret;
}
