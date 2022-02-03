// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import {
  assert,
  assertEquals,
  assertStringIncludes,
} from "../testing/asserts.ts";
import {
  fromFileUrl,
  join,
  joinGlobs,
  normalize,
  relative,
} from "../path/mod.ts";
import {
  expandGlob,
  ExpandGlobOptions,
  expandGlobSync,
} from "./expand_glob.ts";

async function expandGlobArray(
  globString: string,
  options: ExpandGlobOptions,
): Promise<string[]> {
  const paths: string[] = [];
  for await (const { path } of expandGlob(globString, options)) {
    paths.push(path);
  }
  paths.sort();
  const pathsSync = [...expandGlobSync(globString, options)].map(
    ({ path }): string => path,
  );
  pathsSync.sort();
  assertEquals(paths, pathsSync);
  const root = normalize(options.root || Deno.cwd());
  for (const path of paths) {
    assert(path.startsWith(root));
  }
  const relativePaths = paths.map(
    (path: string): string => relative(root, path) || ".",
  );
  relativePaths.sort();
  return relativePaths;
}

const EG_OPTIONS: ExpandGlobOptions = {
  root: fromFileUrl(new URL(join("testdata", "glob"), import.meta.url)),
  includeDirs: true,
  extended: false,
  globstar: false,
};

Deno.test("expandGlobWildcard", async function () {
  const options = EG_OPTIONS;
  assertEquals(await expandGlobArray("*", options), [
    "a[b]c",
    "abc",
    "abcdef",
    "abcdefghi",
    "subdir",
  ]);
});

Deno.test("expandGlobTrailingSeparator", async function () {
  const options = EG_OPTIONS;
  assertEquals(await expandGlobArray("*/", options), ["a[b]c", "subdir"]);
});

Deno.test("expandGlobParent", async function () {
  const options = EG_OPTIONS;
  assertEquals(await expandGlobArray("subdir/../*", options), [
    "a[b]c",
    "abc",
    "abcdef",
    "abcdefghi",
    "subdir",
  ]);
});

Deno.test("expandGlobExt", async function () {
  const options = { ...EG_OPTIONS, extended: true };
  assertEquals(await expandGlobArray("abc?(def|ghi)", options), [
    "abc",
    "abcdef",
  ]);
  assertEquals(await expandGlobArray("abc*(def|ghi)", options), [
    "abc",
    "abcdef",
    "abcdefghi",
  ]);
  assertEquals(await expandGlobArray("abc+(def|ghi)", options), [
    "abcdef",
    "abcdefghi",
  ]);
  assertEquals(await expandGlobArray("abc@(def|ghi)", options), ["abcdef"]);
  assertEquals(await expandGlobArray("abc{def,ghi}", options), ["abcdef"]);
  assertEquals(await expandGlobArray("abc!(def|ghi)", options), ["abc"]);
});

Deno.test("expandGlobGlobstar", async function () {
  const options = { ...EG_OPTIONS, globstar: true };
  assertEquals(
    await expandGlobArray(joinGlobs(["**", "abc"], options), options),
    ["abc", join("subdir", "abc")],
  );
});

Deno.test("expandGlobGlobstarParent", async function () {
  const options = { ...EG_OPTIONS, globstar: true };
  assertEquals(
    await expandGlobArray(joinGlobs(["subdir", "**", ".."], options), options),
    ["."],
  );
});

Deno.test("expandGlobIncludeDirs", async function () {
  const options = { ...EG_OPTIONS, includeDirs: false };
  assertEquals(await expandGlobArray("subdir", options), []);
});

Deno.test("expandGlobPermError", async function () {
  const exampleUrl = new URL("testdata/expand_wildcard.js", import.meta.url);
  const p = Deno.run({
    cmd: [
      Deno.execPath(),
      "run",
      "--quiet",
      "--unstable",
      exampleUrl.toString(),
    ],
    stdin: "null",
    stdout: "piped",
    stderr: "piped",
  });
  const decoder = new TextDecoder();
  assertEquals(await p.status(), { code: 1, success: false });
  assertEquals(decoder.decode(await p.output()), "");
  assertStringIncludes(
    decoder.decode(await p.stderrOutput()),
    "Uncaught PermissionDenied",
  );
  p.close();
});

Deno.test("expandGlobRootIsNotGlob", async function () {
  const options = { ...EG_OPTIONS, root: join(EG_OPTIONS.root!, "a[b]c") };
  assertEquals(await expandGlobArray("*", options), ["foo"]);
});
