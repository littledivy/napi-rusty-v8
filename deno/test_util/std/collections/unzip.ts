// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

/**
 * Builds two separate arrays from the given array of 2-tuples, with the first returned array holding all first
 * tuple elements and the second one holding all the second elements
 *
 * Example:
 *
 * ```ts
 * import { unzip } from "https://deno.land/std@$STD_VERSION/collections/mod.ts";
 * import { assertEquals } from "https://deno.land/std@$STD_VERSION/testing/asserts.ts";
 *
 * const parents = [
 *     [ 'Maria', 'Jeff' ],
 *     [ 'Anna', 'Kim' ],
 *     [ 'John', 'Leroy' ],
 * ] as [string, string][];
 *
 * const [ moms, dads ] = unzip(parents);
 *
 * assertEquals(moms, [ 'Maria', 'Anna', 'John' ]);
 * assertEquals(dads, [ 'Jeff', 'Kim', 'Leroy' ]);
 * ```
 */
export function unzip<T, U>(pairs: readonly [T, U][]): [T[], U[]] {
  const { length } = pairs;
  const ret: [T[], U[]] = [
    Array.from({ length }),
    Array.from({ length }),
  ];

  pairs.forEach(([first, second], index) => {
    ret[0][index] = first;
    ret[1][index] = second;
  });

  return ret;
}
