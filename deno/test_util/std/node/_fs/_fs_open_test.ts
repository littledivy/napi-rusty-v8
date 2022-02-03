import {
  assert,
  assertEquals,
  assertThrows,
  fail,
} from "../../testing/asserts.ts";
import { assertCallbackErrorUncaught } from "../_utils.ts";
import { open, openSync } from "./_fs_open.ts";
import { join, parse } from "../../path/mod.ts";
import { existsSync } from "../../fs/exists.ts";
import { closeSync } from "./_fs_close.ts";

const tempDir = parse(Deno.makeTempFileSync()).dir;

Deno.test({
  name: "ASYNC: open file",
  async fn() {
    const file = Deno.makeTempFileSync();
    let fd1: number;
    await new Promise<number>((resolve, reject) => {
      open(file, (err, fd) => {
        if (err) reject(err);
        resolve(fd);
      });
    })
      .then((fd) => {
        fd1 = fd;
        assert(Deno.resources()[fd], `${fd}`);
      }, () => fail())
      .finally(() => closeSync(fd1));
  },
});

Deno.test({
  name: "SYNC: open file",
  fn() {
    const file = Deno.makeTempFileSync();
    const fd = openSync(file);
    assert(Deno.resources()[fd]);
    closeSync(fd);
  },
});

Deno.test({
  name: "open with flag 'a'",
  fn() {
    const file = join(tempDir, "some_random_file");
    const fd = openSync(file, "a");
    assertEquals(typeof fd, "number");
    assertEquals(existsSync(file), true);
    assert(Deno.resources()[fd]);
    closeSync(fd);
  },
});

Deno.test({
  name: "open with flag 'ax'",
  fn() {
    const file = Deno.makeTempFileSync();
    assertThrows(
      () => {
        openSync(file, "ax");
      },
      Error,
      `EEXIST: file already exists, open '${file}'`,
    );
    Deno.removeSync(file);
  },
});

Deno.test({
  name: "open with flag 'a+'",
  fn() {
    const file = join(tempDir, "some_random_file2");
    const fd = openSync(file, "a+");
    assertEquals(typeof fd, "number");
    assertEquals(existsSync(file), true);
    closeSync(fd);
  },
});

Deno.test({
  name: "open with flag 'ax+'",
  fn() {
    const file = Deno.makeTempFileSync();
    assertThrows(
      () => {
        openSync(file, "ax+");
      },
      Error,
      `EEXIST: file already exists, open '${file}'`,
    );
    Deno.removeSync(file);
  },
});

Deno.test({
  name: "open with flag 'as'",
  fn() {
    const file = join(tempDir, "some_random_file10");
    const fd = openSync(file, "as");
    assertEquals(existsSync(file), true);
    assertEquals(typeof fd, "number");
    closeSync(fd);
  },
});

Deno.test({
  name: "open with flag 'as+'",
  fn() {
    const file = join(tempDir, "some_random_file10");
    const fd = openSync(file, "as+");
    assertEquals(existsSync(file), true);
    assertEquals(typeof fd, "number");
    closeSync(fd);
  },
});

Deno.test({
  name: "open with flag 'r'",
  fn() {
    const file = join(tempDir, "some_random_file3");
    assertThrows(() => {
      openSync(file, "r");
    }, Error);
  },
});

Deno.test({
  name: "open with flag 'r+'",
  fn() {
    const file = join(tempDir, "some_random_file4");
    assertThrows(() => {
      openSync(file, "r+");
    }, Error);
  },
});

Deno.test({
  name: "open with flag 'w'",
  fn() {
    const file = Deno.makeTempFileSync();
    Deno.writeTextFileSync(file, "hi there");
    const fd = openSync(file, "w");
    assertEquals(typeof fd, "number");
    assertEquals(Deno.readTextFileSync(file), "");
    closeSync(fd);

    const file2 = join(tempDir, "some_random_file5");
    const fd2 = openSync(file2, "w");
    assertEquals(typeof fd2, "number");
    assertEquals(existsSync(file2), true);
    closeSync(fd2);
  },
});

Deno.test({
  name: "open with flag 'wx'",
  fn() {
    const file = Deno.makeTempFileSync();
    Deno.writeTextFileSync(file, "hi there");
    const fd = openSync(file, "w");
    assertEquals(typeof fd, "number");
    assertEquals(Deno.readTextFileSync(file), "");
    closeSync(fd);

    const file2 = Deno.makeTempFileSync();
    assertThrows(
      () => {
        openSync(file2, "wx");
      },
      Error,
      `EEXIST: file already exists, open '${file2}'`,
    );
  },
});

Deno.test({
  name: "open with flag 'w+'",
  fn() {
    const file = Deno.makeTempFileSync();
    Deno.writeTextFileSync(file, "hi there");
    const fd = openSync(file, "w+");
    assertEquals(typeof fd, "number");
    assertEquals(Deno.readTextFileSync(file), "");
    closeSync(fd);

    const file2 = join(tempDir, "some_random_file6");
    const fd2 = openSync(file2, "w+");
    assertEquals(typeof fd2, "number");
    assertEquals(existsSync(file2), true);
    closeSync(fd2);
  },
});

Deno.test({
  name: "open with flag 'wx+'",
  fn() {
    const file = Deno.makeTempFileSync();
    assertThrows(
      () => {
        openSync(file, "wx+");
      },
      Error,
      `EEXIST: file already exists, open '${file}'`,
    );
    Deno.removeSync(file);
  },
});

Deno.test("[std/node/fs] open callback isn't called twice if error is thrown", async () => {
  const tempFile = await Deno.makeTempFile();
  const importUrl = new URL("./_fs_open.ts", import.meta.url);
  await assertCallbackErrorUncaught({
    prelude: `import { open } from ${JSON.stringify(importUrl)}`,
    invocation: `open(${JSON.stringify(tempFile)}, `,
    async cleanup() {
      await Deno.remove(tempFile);
    },
  });
});
