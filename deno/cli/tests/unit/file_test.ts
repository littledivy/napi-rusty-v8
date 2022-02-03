// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
import { assert, assertEquals } from "./test_util.ts";

// deno-lint-ignore no-explicit-any
function testFirstArgument(arg1: any[], expectedSize: number) {
  const file = new File(arg1, "name");
  assert(file instanceof File);
  assertEquals(file.name, "name");
  assertEquals(file.size, expectedSize);
  assertEquals(file.type, "");
}

Deno.test(function fileEmptyFileBits() {
  testFirstArgument([], 0);
});

Deno.test(function fileStringFileBits() {
  testFirstArgument(["bits"], 4);
});

Deno.test(function fileUnicodeStringFileBits() {
  testFirstArgument(["𝓽𝓮𝔁𝓽"], 16);
});

Deno.test(function fileStringObjectFileBits() {
  testFirstArgument([new String("string object")], 13);
});

Deno.test(function fileEmptyBlobFileBits() {
  testFirstArgument([new Blob()], 0);
});

Deno.test(function fileBlobFileBits() {
  testFirstArgument([new Blob(["bits"])], 4);
});

Deno.test(function fileEmptyFileFileBits() {
  testFirstArgument([new File([], "world.txt")], 0);
});

Deno.test(function fileFileFileBits() {
  testFirstArgument([new File(["bits"], "world.txt")], 4);
});

Deno.test(function fileArrayBufferFileBits() {
  testFirstArgument([new ArrayBuffer(8)], 8);
});

Deno.test(function fileTypedArrayFileBits() {
  testFirstArgument([new Uint8Array([0x50, 0x41, 0x53, 0x53])], 4);
});

Deno.test(function fileVariousFileBits() {
  testFirstArgument(
    [
      "bits",
      new Blob(["bits"]),
      new Blob(),
      new Uint8Array([0x50, 0x41]),
      new Uint16Array([0x5353]),
      new Uint32Array([0x53534150]),
    ],
    16,
  );
});

Deno.test(function fileNumberInFileBits() {
  testFirstArgument([12], 2);
});

Deno.test(function fileArrayInFileBits() {
  testFirstArgument([[1, 2, 3]], 5);
});

Deno.test(function fileObjectInFileBits() {
  // "[object Object]"
  testFirstArgument([{}], 15);
});

// deno-lint-ignore no-explicit-any
function testSecondArgument(arg2: any, expectedFileName: string) {
  const file = new File(["bits"], arg2);
  assert(file instanceof File);
  assertEquals(file.name, expectedFileName);
}

Deno.test(function fileUsingFileName() {
  testSecondArgument("dummy", "dummy");
});

Deno.test(function fileUsingNullFileName() {
  testSecondArgument(null, "null");
});

Deno.test(function fileUsingNumberFileName() {
  testSecondArgument(1, "1");
});

Deno.test(function fileUsingEmptyStringFileName() {
  testSecondArgument("", "");
});

Deno.test(
  { permissions: { read: true, write: true } },
  function fileTruncateSyncSuccess() {
    const filename = Deno.makeTempDirSync() + "/test_fileTruncateSync.txt";
    const file = Deno.openSync(filename, {
      create: true,
      read: true,
      write: true,
    });

    file.truncateSync(20);
    assertEquals(Deno.readFileSync(filename).byteLength, 20);
    file.truncateSync(5);
    assertEquals(Deno.readFileSync(filename).byteLength, 5);
    file.truncateSync(-5);
    assertEquals(Deno.readFileSync(filename).byteLength, 0);

    file.close();
    Deno.removeSync(filename);
  },
);

Deno.test(
  { permissions: { read: true, write: true } },
  async function fileTruncateSuccess() {
    const filename = Deno.makeTempDirSync() + "/test_fileTruncate.txt";
    const file = await Deno.open(filename, {
      create: true,
      read: true,
      write: true,
    });

    await file.truncate(20);
    assertEquals((await Deno.readFile(filename)).byteLength, 20);
    await file.truncate(5);
    assertEquals((await Deno.readFile(filename)).byteLength, 5);
    await file.truncate(-5);
    assertEquals((await Deno.readFile(filename)).byteLength, 0);

    file.close();
    await Deno.remove(filename);
  },
);

Deno.test({ permissions: { read: true } }, function fileStatSyncSuccess() {
  const file = Deno.openSync("README.md");
  const fileInfo = file.statSync();
  assert(fileInfo.isFile);
  assert(!fileInfo.isSymlink);
  assert(!fileInfo.isDirectory);
  assert(fileInfo.size);
  assert(fileInfo.atime);
  assert(fileInfo.mtime);
  // The `birthtime` field is not available on Linux before kernel version 4.11.
  assert(fileInfo.birthtime || Deno.build.os === "linux");

  file.close();
});

Deno.test({ permissions: { read: true } }, async function fileStatSuccess() {
  const file = await Deno.open("README.md");
  const fileInfo = await file.stat();
  assert(fileInfo.isFile);
  assert(!fileInfo.isSymlink);
  assert(!fileInfo.isDirectory);
  assert(fileInfo.size);
  assert(fileInfo.atime);
  assert(fileInfo.mtime);
  // The `birthtime` field is not available on Linux before kernel version 4.11.
  assert(fileInfo.birthtime || Deno.build.os === "linux");

  file.close();
});
