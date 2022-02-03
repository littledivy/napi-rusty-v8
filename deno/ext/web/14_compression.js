// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

// @ts-check
/// <reference path="../../core/lib.deno_core.d.ts" />
/// <reference path="./internal.d.ts" />
/// <reference path="./lib.deno_web.d.ts" />

"use strict";

((window) => {
  const core = window.Deno.core;
  const webidl = window.__bootstrap.webidl;
  const { TransformStream } = window.__bootstrap.streams;

  webidl.converters.CompressionFormat = webidl.createEnumConverter(
    "CompressionFormat",
    [
      "deflate",
      "gzip",
    ],
  );

  class CompressionStream {
    #transform;

    constructor(format) {
      const prefix = "Failed to construct 'CompressionStream'";
      webidl.requiredArguments(arguments.length, 1, { prefix });
      format = webidl.converters.CompressionFormat(format, {
        prefix,
        context: "Argument 1",
      });

      const rid = core.opSync("op_compression_new", format, false);

      this.#transform = new TransformStream({
        transform(chunk, controller) {
          // TODO(lucacasonato): convert chunk to BufferSource
          const output = core.opSync(
            "op_compression_write",
            rid,
            chunk,
          );
          maybeEnqueue(controller, output);
        },
        flush(controller) {
          const output = core.opSync("op_compression_finish", rid);
          maybeEnqueue(controller, output);
        },
      });

      this[webidl.brand] = webidl.brand;
    }

    get readable() {
      webidl.assertBranded(this, CompressionStreamPrototype);
      return this.#transform.readable;
    }

    get writable() {
      webidl.assertBranded(this, CompressionStreamPrototype);
      return this.#transform.writable;
    }
  }

  webidl.configurePrototype(CompressionStream);
  const CompressionStreamPrototype = CompressionStream.prototype;

  class DecompressionStream {
    #transform;

    constructor(format) {
      const prefix = "Failed to construct 'DecompressionStream'";
      webidl.requiredArguments(arguments.length, 1, { prefix });
      format = webidl.converters.CompressionFormat(format, {
        prefix,
        context: "Argument 1",
      });

      const rid = core.opSync("op_compression_new", format, true);

      this.#transform = new TransformStream({
        transform(chunk, controller) {
          // TODO(lucacasonato): convert chunk to BufferSource
          const output = core.opSync(
            "op_compression_write",
            rid,
            chunk,
          );
          maybeEnqueue(controller, output);
        },
        flush(controller) {
          const output = core.opSync("op_compression_finish", rid);
          maybeEnqueue(controller, output);
        },
      });

      this[webidl.brand] = webidl.brand;
    }

    get readable() {
      webidl.assertBranded(this, DecompressionStreamPrototype);
      return this.#transform.readable;
    }

    get writable() {
      webidl.assertBranded(this, DecompressionStreamPrototype);
      return this.#transform.writable;
    }
  }

  function maybeEnqueue(controller, output) {
    if (output && output.byteLength > 0) {
      controller.enqueue(output);
    }
  }

  webidl.configurePrototype(DecompressionStream);
  const DecompressionStreamPrototype = DecompressionStream.prototype;

  window.__bootstrap.compression = {
    CompressionStream,
    DecompressionStream,
  };
})(globalThis);
