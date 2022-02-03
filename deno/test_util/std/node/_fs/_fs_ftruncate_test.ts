// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import { assertEquals, assertThrows, fail } from "../../testing/asserts.ts";
import { ftruncate, ftruncateSync } from "./_fs_ftruncate.ts";

Deno.test({
  name: "ASYNC: no callback function results in Error",
  fn() {
    assertThrows(
      () => {
        ftruncate(123, 0);
      },
      Error,
      "No callback function supplied",
    );
  },
});

Deno.test({
  name: "ASYNC: truncate entire file contents",
  async fn() {
    const file: string = Deno.makeTempFileSync();
    await Deno.writeFile(file, new TextEncoder().encode("hello world"));
    const { rid } = await Deno.open(file, {
      read: true,
      write: true,
      create: true,
    });

    await new Promise<void>((resolve, reject) => {
      ftruncate(rid, (err: Error | null) => {
        if (err !== null) reject();
        else resolve();
      });
    })
      .then(
        () => {
          const fileInfo: Deno.FileInfo = Deno.lstatSync(file);
          assertEquals(fileInfo.size, 0);
        },
        () => {
          fail("No error expected");
        },
      )
      .finally(() => {
        Deno.removeSync(file);
        Deno.close(rid);
      });
  },
});

Deno.test({
  name: "ASYNC: truncate file to a size of precisely len bytes",
  async fn() {
    const file: string = Deno.makeTempFileSync();
    await Deno.writeFile(file, new TextEncoder().encode("hello world"));
    const { rid } = await Deno.open(file, {
      read: true,
      write: true,
      create: true,
    });

    await new Promise<void>((resolve, reject) => {
      ftruncate(rid, 3, (err: Error | null) => {
        if (err !== null) reject();
        else resolve();
      });
    })
      .then(
        () => {
          const fileInfo: Deno.FileInfo = Deno.lstatSync(file);
          assertEquals(fileInfo.size, 3);
        },
        () => {
          fail("No error expected");
        },
      )
      .finally(() => {
        Deno.removeSync(file);
        Deno.close(rid);
      });
  },
});

Deno.test({
  name: "SYNC: truncate entire file contents",
  fn() {
    const file: string = Deno.makeTempFileSync();
    Deno.writeFileSync(file, new TextEncoder().encode("hello world"));
    const { rid } = Deno.openSync(file, {
      read: true,
      write: true,
      create: true,
    });

    try {
      ftruncateSync(rid);
      const fileInfo: Deno.FileInfo = Deno.lstatSync(file);
      assertEquals(fileInfo.size, 0);
    } finally {
      Deno.removeSync(file);
      Deno.close(rid);
    }
  },
});

Deno.test({
  name: "SYNC: truncate file to a size of precisely len bytes",
  fn() {
    const file: string = Deno.makeTempFileSync();
    Deno.writeFileSync(file, new TextEncoder().encode("hello world"));
    const { rid } = Deno.openSync(file, {
      read: true,
      write: true,
      create: true,
    });

    try {
      ftruncateSync(rid, 3);
      const fileInfo: Deno.FileInfo = Deno.lstatSync(file);
      assertEquals(fileInfo.size, 3);
    } finally {
      Deno.removeSync(file);
      Deno.close(rid);
    }
  },
});
