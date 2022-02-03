// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import {
  assertEquals,
  assertMatch,
  assertStrictEquals,
} from "../../testing/asserts.ts";
import { read, readSync } from "./_fs_read.ts";
import { open, openSync } from "./_fs_open.ts";
import { Buffer } from "../buffer.ts";
import * as path from "../../path/mod.ts";
import { closeSync } from "./_fs_close.ts";

async function readTest(
  testData: string,
  buffer: Buffer | Uint8Array,
  offset: number,
  length: number,
  position: number | null = null,
  expected: (
    fd: number,
    bytesRead: number | null,
    data: Buffer | undefined,
  ) => void,
) {
  let fd1 = 0;
  await new Promise<{
    fd: number;
    bytesRead: number | null;
    data: Buffer | undefined;
  }>((resolve, reject) => {
    open(testData, "r", (err, fd) => {
      if (err) reject(err);
      read(fd, buffer, offset, length, position, (err, bytesRead, data) => {
        if (err) reject(err);
        resolve({ fd, bytesRead, data });
      });
    });
  })
    .then(({ fd, bytesRead, data }) => {
      fd1 = fd;
      expected(fd, bytesRead, data);
    })
    .finally(() => closeSync(fd1));
}

Deno.test({
  name: "readSuccess",
  async fn() {
    const moduleDir = path.dirname(path.fromFileUrl(import.meta.url));
    const testData = path.resolve(moduleDir, "testdata", "hello.txt");
    const buf = Buffer.alloc(1024);
    await readTest(
      testData,
      buf,
      buf.byteOffset,
      buf.byteLength,
      null,
      (_fd, bytesRead, data) => {
        assertStrictEquals(bytesRead, 11);
        assertEquals(data instanceof Buffer, true);
        assertMatch((data as Buffer).toString(), /hello world/);
      },
    );
  },
});

Deno.test({
  name:
    "[std/node/fs] Read only five bytes, so that the position moves to five",
  async fn() {
    const moduleDir = path.dirname(path.fromFileUrl(import.meta.url));
    const testData = path.resolve(moduleDir, "testdata", "hello.txt");
    const buf = Buffer.alloc(5);
    await readTest(
      testData,
      buf,
      buf.byteOffset,
      5,
      null,
      (_fd, bytesRead, data) => {
        assertStrictEquals(bytesRead, 5);
        assertEquals(data instanceof Buffer, true);
        assertEquals((data as Buffer).toString(), "hello");
      },
    );
  },
});

Deno.test({
  name: "[std/node/fs] Specifies where to begin reading from in the file",
  async fn() {
    const moduleDir = path.dirname(path.fromFileUrl(import.meta.url));
    const testData = path.resolve(moduleDir, "testdata", "hello.txt");
    const buf = Buffer.alloc(11);
    await readTest(
      testData,
      buf,
      buf.byteOffset,
      buf.byteLength,
      6,
      (_fd, bytesRead, data) => {
        assertStrictEquals(bytesRead, 5);
        assertEquals(
          data,
          Buffer.from([119, 111, 114, 108, 100, 0, 0, 0, 0, 0, 0]),
        );
      },
    );
  },
});

Deno.test({
  name: "[std/node/fs] Read fs.read(fd, options, cb) signature",
  async fn() {
    const file = Deno.makeTempFileSync();
    Deno.writeTextFileSync(file, "hi there");
    const fd = openSync(file, "r+");
    const buf = Buffer.alloc(11);
    await read(
      fd,
      {
        buffer: buf,
        offset: buf.byteOffset,
        length: buf.byteLength,
        position: null,
      },
      (err, bytesRead, data) => {
        assertEquals(err, null);
        assertStrictEquals(bytesRead, 8);
        assertEquals(
          data,
          Buffer.from([104, 105, 32, 116, 104, 101, 114, 101, 0, 0, 0]),
        );
      },
    );
    closeSync(fd);
  },
});

Deno.test({
  name: "[std/node/fs] Read fs.read(fd, cb) signature",
  async fn() {
    const file = Deno.makeTempFileSync();
    Deno.writeTextFileSync(file, "hi deno");
    const fd = openSync(file, "r+");
    await read(fd, (err, bytesRead, data) => {
      assertEquals(err, null);
      assertStrictEquals(bytesRead, 7);
      assertStrictEquals(data?.byteLength, 16384);
    });
    closeSync(fd);
  },
});

Deno.test({
  name: "SYNC: readSuccess",
  fn() {
    const moduleDir = path.dirname(path.fromFileUrl(import.meta.url));
    const testData = path.resolve(moduleDir, "testdata", "hello.txt");
    const buffer = Buffer.alloc(1024);
    const fd = openSync(testData);
    const bytesRead = readSync(
      fd,
      buffer,
      buffer.byteOffset,
      buffer.byteLength,
      null,
    );
    assertStrictEquals(bytesRead, 11);
    closeSync(fd);
  },
});

Deno.test({
  name: "[std/node/fs] Read only two bytes, so that the position moves to two",
  fn() {
    const moduleDir = path.dirname(path.fromFileUrl(import.meta.url));
    const testData = path.resolve(moduleDir, "testdata", "hello.txt");
    const buffer = Buffer.alloc(2);
    const fd = openSync(testData);
    const bytesRead = readSync(fd, buffer, buffer.byteOffset, 2, null);
    assertStrictEquals(bytesRead, 2);
    closeSync(fd);
  },
});

Deno.test({
  name: "[std/node/fs] Read fs.readSync(fd, buffer[, options]) signature",
  fn() {
    const file = Deno.makeTempFileSync();
    Deno.writeTextFileSync(file, "hello deno");
    const buffer = Buffer.alloc(1024);
    const fd = openSync(file, "r+");
    const bytesRead = readSync(fd, buffer, {
      length: buffer.byteLength,
      offset: buffer.byteOffset,
      position: null,
    });
    assertStrictEquals(bytesRead, 10);
    closeSync(fd);
  },
});
