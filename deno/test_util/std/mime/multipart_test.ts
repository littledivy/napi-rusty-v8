// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import {
  assert,
  assertEquals,
  assertRejects,
  assertThrows,
} from "../testing/asserts.ts";
import * as path from "../path/mod.ts";
import {
  isFormFile,
  matchAfterPrefix,
  MultipartReader,
  MultipartWriter,
  scanUntilBoundary,
} from "./multipart.ts";
import { StringWriter } from "../io/writers.ts";
import { Buffer } from "../io/buffer.ts";

const e = new TextEncoder();
const boundary = "--abcde";
const dashBoundary = e.encode("--" + boundary);
const nlDashBoundary = e.encode("\r\n--" + boundary);

const moduleDir = path.dirname(path.fromFileUrl(import.meta.url));
const testdataDir = path.resolve(moduleDir, "testdata");

Deno.test("multipartScanUntilBoundary1", function (): void {
  const data = `--${boundary}`;
  const n = scanUntilBoundary(
    e.encode(data),
    dashBoundary,
    nlDashBoundary,
    0,
    true,
  );
  assertEquals(n, null);
});

Deno.test("multipartScanUntilBoundary2", function (): void {
  const data = `foo\r\n--${boundary}`;
  const n = scanUntilBoundary(
    e.encode(data),
    dashBoundary,
    nlDashBoundary,
    0,
    true,
  );
  assertEquals(n, 3);
});

Deno.test("multipartScanUntilBoundary3", function (): void {
  const data = `foobar`;
  const n = scanUntilBoundary(
    e.encode(data),
    dashBoundary,
    nlDashBoundary,
    0,
    false,
  );
  assertEquals(n, data.length);
});

Deno.test("multipartScanUntilBoundary4", function (): void {
  const data = `foo\r\n--`;
  const n = scanUntilBoundary(
    e.encode(data),
    dashBoundary,
    nlDashBoundary,
    0,
    false,
  );
  assertEquals(n, 3);
});

Deno.test("multipartMatchAfterPrefix1", function (): void {
  const data = `${boundary}\r`;
  const v = matchAfterPrefix(e.encode(data), e.encode(boundary), false);
  assertEquals(v, 1);
});

Deno.test("multipartMatchAfterPrefix2", function (): void {
  const data = `${boundary}hoge`;
  const v = matchAfterPrefix(e.encode(data), e.encode(boundary), false);
  assertEquals(v, -1);
});

Deno.test("multipartMatchAfterPrefix3", function (): void {
  const data = `${boundary}`;
  const v = matchAfterPrefix(e.encode(data), e.encode(boundary), false);
  assertEquals(v, 0);
});

Deno.test("multipartMultipartWriter", async function () {
  const buf = new Buffer();
  const mw = new MultipartWriter(buf);
  await mw.writeField("foo", "foo");
  await mw.writeField("bar", "bar");
  const f = await Deno.open(path.join(testdataDir, "sample.txt"), {
    read: true,
  });
  await mw.writeFile("file", "sample.txt", f);
  await mw.close();
  f.close();
});

Deno.test("multipartMultipartWriter2", function (): void {
  const w = new StringWriter();
  assertThrows(
    (): MultipartWriter => new MultipartWriter(w, ""),
    Error,
    "invalid boundary length",
  );
  assertThrows(
    (): MultipartWriter =>
      new MultipartWriter(
        w,
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" +
          "aaaaaaaa",
      ),
    Error,
    "invalid boundary length",
  );
  assertThrows(
    (): MultipartWriter => new MultipartWriter(w, "aaa aaa"),
    Error,
    "invalid boundary character",
  );
  assertThrows(
    (): MultipartWriter => new MultipartWriter(w, "boundary¥¥"),
    Error,
    "invalid boundary character",
  );
});

Deno.test("multipartMultipartWriter3", async function () {
  const w = new StringWriter();
  const mw = new MultipartWriter(w);
  await mw.writeField("foo", "foo");
  await mw.close();
  await assertRejects(
    async () => {
      await mw.close();
    },
    Error,
    "closed",
  );
  await assertRejects(
    async () => {
      // deno-lint-ignore no-explicit-any
      await mw.writeFile("bar", "file", null as any);
    },
    Error,
    "closed",
  );
  await assertRejects(
    async () => {
      await mw.writeField("bar", "bar");
    },
    Error,
    "closed",
  );
  assertThrows(
    (): void => {
      mw.createFormField("bar");
    },
    Error,
    "closed",
  );
  assertThrows(
    (): void => {
      mw.createFormFile("bar", "file");
    },
    Error,
    "closed",
  );
});

Deno.test({
  name: "[mime/multipart] readForm() basic",
  async fn() {
    const o = await Deno.open(path.join(testdataDir, "sample.txt"));
    const mr = new MultipartReader(
      o,
      "--------------------------434049563556637648550474",
    );
    const form = await mr.readForm();
    assertEquals(form.values("foo"), ["foo"]);
    assertEquals(form.values("bar"), ["bar"]);
    assertEquals(form.values("baz"), ["str1", "str2"]);
    const [file] = form.files("file") || [];
    assert(isFormFile(file));
    assert(file.content !== void 0);
    const [file2] = form.files("file2") || [];
    assert(isFormFile(file2));
    assert(file2.filename === "中文.json");
    assert(file2.content !== void 0);
    o.close();
  },
});

Deno.test({
  name:
    "[mime/multipart] readForm() should store big file completely in temp file",
  async fn() {
    const multipartFile = path.join(testdataDir, "form-data.dat");
    const sampleFile = await Deno.makeTempFile();
    const writer = await Deno.open(multipartFile, {
      write: true,
      create: true,
    });

    const size = 1 << 24; // 16mb

    await Deno.truncate(sampleFile, size);
    const bigFile = await Deno.open(sampleFile, { read: true });

    const mw = new MultipartWriter(writer);
    await mw.writeField("deno", "land");
    await mw.writeField("bar", "bar");
    await mw.writeFile("file", "sample.bin", bigFile);

    await mw.close();
    writer.close();
    bigFile.close();

    const o = await Deno.open(multipartFile);
    const mr = new MultipartReader(o, mw.boundary);
    // use low-memory to write "file" into temp file.
    const form = await mr.readForm({ maxMemory: 20 });
    try {
      assertEquals(form.values("deno"), ["land"]);
      assertEquals(form.values("bar"), ["bar"]);
      const [file] = form.files("file") || [];
      assert(file != null);
      assert(file.tempfile != null);
      assertEquals(file.size, size);
      assertEquals(file.type, "application/octet-stream");
      // TODO(bartlomieju): checksum of tmp & sampleFile
    } finally {
      await Deno.remove(multipartFile);
      await Deno.remove(sampleFile);
      await form.removeAll();
      o.close();
    }
  },
});

Deno.test({
  name: "[mime/multipart] removeAll() should remove all tempfiles",
  async fn() {
    const o = await Deno.open(path.join(testdataDir, "sample.txt"));
    const mr = new MultipartReader(
      o,
      "--------------------------434049563556637648550474",
    );
    const form = await mr.readForm({ maxMemory: 20 });
    const file = form.files("file")![0];
    assert(file != null);
    const { tempfile, content } = file;
    assert(tempfile != null);
    assert(content == null);
    const stat = await Deno.stat(tempfile);
    assertEquals(stat.size, file.size);
    await form.removeAll();
    await assertRejects(async () => {
      await Deno.stat(tempfile);
    }, Deno.errors.NotFound);
    o.close();
  },
});

Deno.test({
  name: "[mime/multipart] entries()",
  async fn() {
    const o = await Deno.open(path.join(testdataDir, "sample.txt"));
    const mr = new MultipartReader(
      o,
      "--------------------------434049563556637648550474",
    );
    const form = await mr.readForm();
    const map = new Map(form.entries());
    assertEquals(map.get("foo"), ["foo"]);
    assertEquals(map.get("bar"), ["bar"]);
    assertEquals(map.get("baz"), ["str1", "str2"]);
    const [file] = map.get("file") || [];
    assert(isFormFile(file));
    assertEquals(file.filename, "tsconfig.json");
    o.close();
  },
});
