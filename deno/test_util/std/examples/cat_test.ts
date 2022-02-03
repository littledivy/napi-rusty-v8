// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import { assertStrictEquals } from "../testing/asserts.ts";
import { dirname, fromFileUrl } from "../path/mod.ts";

const moduleDir = dirname(fromFileUrl(import.meta.url));

Deno.test("[examples/cat] print multiple files", async () => {
  const decoder = new TextDecoder();
  const process = Deno.run({
    cmd: [
      Deno.execPath(),
      "run",
      "--quiet",
      "--allow-read",
      "cat.ts",
      "testdata/cat/hello.txt",
      "testdata/cat/world.txt",
    ],
    cwd: moduleDir,
    stdout: "piped",
  });

  try {
    const output = await process.output();
    const actual = decoder.decode(output).trim();
    assertStrictEquals(actual, "Hello\nWorld");
  } finally {
    process.close();
  }
});
