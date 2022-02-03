// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
"use strict";

((window) => {
  const core = window.Deno.core;
  const { read, readSync, write, writeSync } = window.__bootstrap.io;
  const { ftruncate, ftruncateSync, fstat, fstatSync } = window.__bootstrap.fs;
  const { pathFromURL } = window.__bootstrap.util;
  const {
    Error,
    ObjectValues,
    ArrayPrototypeFilter,
  } = window.__bootstrap.primordials;

  function seekSync(
    rid,
    offset,
    whence,
  ) {
    return core.opSync("op_seek_sync", { rid, offset, whence });
  }

  function seek(
    rid,
    offset,
    whence,
  ) {
    return core.opAsync("op_seek_async", { rid, offset, whence });
  }

  function openSync(
    path,
    options = { read: true },
  ) {
    checkOpenOptions(options);
    const mode = options?.mode;
    const rid = core.opSync(
      "op_open_sync",
      { path: pathFromURL(path), options, mode },
    );

    return new File(rid);
  }

  async function open(
    path,
    options = { read: true },
  ) {
    checkOpenOptions(options);
    const mode = options?.mode;
    const rid = await core.opAsync(
      "op_open_async",
      { path: pathFromURL(path), options, mode },
    );

    return new File(rid);
  }

  function createSync(path) {
    return openSync(path, {
      read: true,
      write: true,
      truncate: true,
      create: true,
    });
  }

  function create(path) {
    return open(path, {
      read: true,
      write: true,
      truncate: true,
      create: true,
    });
  }

  class File {
    #rid = 0;

    constructor(rid) {
      this.#rid = rid;
    }

    get rid() {
      return this.#rid;
    }

    write(p) {
      return write(this.rid, p);
    }

    writeSync(p) {
      return writeSync(this.rid, p);
    }

    truncate(len) {
      return ftruncate(this.rid, len);
    }

    truncateSync(len) {
      return ftruncateSync(this.rid, len);
    }

    read(p) {
      return read(this.rid, p);
    }

    readSync(p) {
      return readSync(this.rid, p);
    }

    seek(offset, whence) {
      return seek(this.rid, offset, whence);
    }

    seekSync(offset, whence) {
      return seekSync(this.rid, offset, whence);
    }

    stat() {
      return fstat(this.rid);
    }

    statSync() {
      return fstatSync(this.rid);
    }

    close() {
      core.close(this.rid);
    }
  }

  class Stdin {
    constructor() {
    }

    get rid() {
      return 0;
    }

    read(p) {
      return read(this.rid, p);
    }

    readSync(p) {
      return readSync(this.rid, p);
    }

    close() {
      core.close(this.rid);
    }
  }

  class Stdout {
    constructor() {
    }

    get rid() {
      return 1;
    }

    write(p) {
      return write(this.rid, p);
    }

    writeSync(p) {
      return writeSync(this.rid, p);
    }

    close() {
      core.close(this.rid);
    }
  }

  class Stderr {
    constructor() {
    }

    get rid() {
      return 2;
    }

    write(p) {
      return write(this.rid, p);
    }

    writeSync(p) {
      return writeSync(this.rid, p);
    }

    close() {
      core.close(this.rid);
    }
  }

  const stdin = new Stdin();
  const stdout = new Stdout();
  const stderr = new Stderr();

  function checkOpenOptions(options) {
    if (
      ArrayPrototypeFilter(
        ObjectValues(options),
        (val) => val === true,
      ).length === 0
    ) {
      throw new Error("OpenOptions requires at least one option to be true");
    }

    if (options.truncate && !options.write) {
      throw new Error("'truncate' option requires 'write' option");
    }

    const createOrCreateNewWithoutWriteOrAppend =
      (options.create || options.createNew) &&
      !(options.write || options.append);

    if (createOrCreateNewWithoutWriteOrAppend) {
      throw new Error(
        "'create' or 'createNew' options require 'write' or 'append' option",
      );
    }
  }

  window.__bootstrap.files = {
    stdin,
    stdout,
    stderr,
    File,
    create,
    createSync,
    open,
    openSync,
    seek,
    seekSync,
  };
})(this);
