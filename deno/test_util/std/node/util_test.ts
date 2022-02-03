// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.

import {
  assert,
  assertEquals,
  assertStrictEquals,
  assertThrows,
} from "../testing/asserts.ts";
import { stripColor } from "../fmt/colors.ts";
import * as util from "./util.ts";

Deno.test({
  name: "[util] format",
  fn() {
    assertEquals(util.format("%o", [10, 11]), "[ 10, 11, [length]: 2 ]");
  },
});

Deno.test({
  name: "[util] inspect.custom",
  fn() {
    assertEquals(util.inspect.custom, Symbol.for("nodejs.util.inspect.custom"));
  },
});

Deno.test({
  name: "[util] inspect",
  fn() {
    assertEquals(stripColor(util.inspect({ foo: 123 })), "{ foo: 123 }");
    assertEquals(stripColor(util.inspect("foo")), "'foo'");
    assertEquals(
      stripColor(util.inspect("Deno's logo is so cute.")),
      `"Deno's logo is so cute."`,
    );
  },
});

Deno.test({
  name: "[util] isBoolean",
  fn() {
    assert(util.isBoolean(true));
    assert(util.isBoolean(new Boolean()));
    assert(util.isBoolean(new Boolean(true)));
    assert(util.isBoolean(false));
    assert(!util.isBoolean("deno"));
    assert(!util.isBoolean("true"));
  },
});

Deno.test({
  name: "[util] isNull",
  fn() {
    let n;
    assert(util.isNull(null));
    assert(!util.isNull(n));
    assert(!util.isNull(0));
    assert(!util.isNull({}));
  },
});

Deno.test({
  name: "[util] isNullOrUndefined",
  fn() {
    let n;
    assert(util.isNullOrUndefined(null));
    assert(util.isNullOrUndefined(n));
    assert(!util.isNullOrUndefined({}));
    assert(!util.isNullOrUndefined("undefined"));
  },
});

Deno.test({
  name: "[util] isNumber",
  fn() {
    assert(util.isNumber(666));
    assert(util.isNumber(new Number(666)));
    assert(!util.isNumber("999"));
    assert(!util.isNumber(null));
  },
});

Deno.test({
  name: "[util] isString",
  fn() {
    assert(util.isString("deno"));
    assert(util.isString(new String("DIO")));
    assert(!util.isString(1337));
  },
});

Deno.test({
  name: "[util] isSymbol",
  fn() {
    assert(util.isSymbol(Symbol()));
    assert(!util.isSymbol(123));
    assert(!util.isSymbol("string"));
  },
});

Deno.test({
  name: "[util] isUndefined",
  fn() {
    let t;
    assert(util.isUndefined(t));
    assert(!util.isUndefined("undefined"));
    assert(!util.isUndefined({}));
  },
});

Deno.test({
  name: "[util] isObject",
  fn() {
    const dio = { stand: "Za Warudo" };
    assert(util.isObject(dio));
    assert(util.isObject(new RegExp(/Toki Wo Tomare/)));
    assert(!util.isObject("Jotaro"));
  },
});

Deno.test({
  name: "[util] isError",
  fn() {
    const java = new Error();
    const nodejs = new TypeError();
    const deno = "Future";
    assert(util.isError(java));
    assert(util.isError(nodejs));
    assert(!util.isError(deno));
  },
});

Deno.test({
  name: "[util] isFunction",
  fn() {
    const f = function (): void {};
    assert(util.isFunction(f));
    assert(!util.isFunction({}));
    assert(!util.isFunction(new RegExp(/f/)));
  },
});

Deno.test({
  name: "[util] isRegExp",
  fn() {
    assert(util.isRegExp(new RegExp(/f/)));
    assert(util.isRegExp(/fuManchu/));
    assert(!util.isRegExp({ evil: "eye" }));
    assert(!util.isRegExp(null));
  },
});

Deno.test({
  name: "[util] isArray",
  fn() {
    assert(util.isArray([]));
    assert(!util.isArray({ yaNo: "array" }));
    assert(!util.isArray(null));
  },
});

Deno.test({
  name: "[util] isPrimitive",
  fn() {
    const stringType = "hasti";
    const booleanType = true;
    const integerType = 2;
    const symbolType = Symbol("anything");

    const functionType = function doBest(): void {};
    const objectType = { name: "ali" };
    const arrayType = [1, 2, 3];

    assert(util.isPrimitive(stringType));
    assert(util.isPrimitive(booleanType));
    assert(util.isPrimitive(integerType));
    assert(util.isPrimitive(symbolType));
    assert(util.isPrimitive(null));
    assert(util.isPrimitive(undefined));
    assert(!util.isPrimitive(functionType));
    assert(!util.isPrimitive(arrayType));
    assert(!util.isPrimitive(objectType));
  },
});

Deno.test({
  name: "[util] TextDecoder",
  fn() {
    assert(util.TextDecoder === TextDecoder);
    const td: util.TextDecoder = new util.TextDecoder();
    assert(td instanceof TextDecoder);
  },
});

Deno.test({
  name: "[util] TextEncoder",
  fn() {
    assert(util.TextEncoder === TextEncoder);
    const te: util.TextEncoder = new util.TextEncoder();
    assert(te instanceof TextEncoder);
  },
});

Deno.test({
  name: "[util] isDate",
  fn() {
    // Test verifies the method is exposed. See _util/_util_types_test for details
    assert(util.types.isDate(new Date()));
  },
});

Deno.test({
  name: "[util] getSystemErrorName()",
  fn() {
    type FnTestInvalidArg = (code?: unknown) => void;

    assertThrows(
      () => (util.getSystemErrorName as FnTestInvalidArg)(),
      TypeError,
    );
    assertThrows(
      () => (util.getSystemErrorName as FnTestInvalidArg)(1),
      RangeError,
    );

    assertStrictEquals(util.getSystemErrorName(-424242), undefined);

    switch (Deno.build.os) {
      case "windows":
        assertStrictEquals(util.getSystemErrorName(-4091), "EADDRINUSE");
        break;

      case "darwin":
        assertStrictEquals(util.getSystemErrorName(-48), "EADDRINUSE");
        break;

      case "linux":
        assertStrictEquals(util.getSystemErrorName(-98), "EADDRINUSE");
        break;
    }
  },
});
