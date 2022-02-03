// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
"use strict";

((window) => {
  const core = window.Deno.core;
  const __bootstrap = window.__bootstrap;
  const {
    ArrayBufferPrototype,
    Uint8Array,
    BigInt,
    Number,
    ObjectPrototypeIsPrototypeOf,
    TypeError,
  } = window.__bootstrap.primordials;

  function unpackU64([hi, lo]) {
    return BigInt(hi) << 32n | BigInt(lo);
  }

  function packU64(value) {
    return [Number(value >> 32n), Number(value & 0xFFFFFFFFn)];
  }

  function unpackI64([hi, lo]) {
    const u64 = unpackU64([hi, lo]);
    return u64 >> 63n ? u64 - 0x10000000000000000n : u64;
  }

  class UnsafePointerView {
    pointer;

    constructor(pointer) {
      this.pointer = pointer;
    }

    getUint8(offset = 0) {
      return core.opSync(
        "op_ffi_read_u8",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getInt8(offset = 0) {
      return core.opSync(
        "op_ffi_read_i8",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getUint16(offset = 0) {
      return core.opSync(
        "op_ffi_read_u16",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getInt16(offset = 0) {
      return core.opSync(
        "op_ffi_read_i16",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getUint32(offset = 0) {
      return core.opSync(
        "op_ffi_read_u32",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getInt32(offset = 0) {
      return core.opSync(
        "op_ffi_read_i32",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getBigUint64(offset = 0) {
      return unpackU64(core.opSync(
        "op_ffi_read_u64",
        packU64(this.pointer.value + BigInt(offset)),
      ));
    }

    getBigInt64(offset = 0) {
      return unpackI64(core.opSync(
        "op_ffi_read_u64",
        packU64(this.pointer.value + BigInt(offset)),
      ));
    }

    getFloat32(offset = 0) {
      return core.opSync(
        "op_ffi_read_f32",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getFloat64(offset = 0) {
      return core.opSync(
        "op_ffi_read_f64",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getCString(offset = 0) {
      return core.opSync(
        "op_ffi_cstr_read",
        packU64(this.pointer.value + BigInt(offset)),
      );
    }

    getArrayBuffer(byteLength, offset = 0) {
      const uint8array = new Uint8Array(byteLength);
      this.copyInto(uint8array, offset);
      return uint8array.buffer;
    }

    copyInto(destination, offset = 0) {
      core.opSync("op_ffi_buf_copy_into", [
        packU64(this.pointer.value + BigInt(offset)),
        destination,
        destination.byteLength,
      ]);
    }
  }

  class UnsafePointer {
    value;

    constructor(value) {
      this.value = value;
    }

    static of(typedArray) {
      return new UnsafePointer(
        unpackU64(core.opSync("op_ffi_ptr_of", typedArray)),
      );
    }

    valueOf() {
      return this.value;
    }
  }
  const UnsafePointerPrototype = UnsafePointer.prototype;

  function prepareArgs(types, args) {
    const parameters = [];
    const buffers = [];

    for (let i = 0; i < types.length; i++) {
      const type = types[i];
      const arg = args[i];

      if (type === "pointer") {
        if (
          ObjectPrototypeIsPrototypeOf(ArrayBufferPrototype, arg?.buffer) &&
          arg.byteLength !== undefined
        ) {
          parameters.push(buffers.length);
          buffers.push(arg);
        } else if (ObjectPrototypeIsPrototypeOf(UnsafePointerPrototype, arg)) {
          parameters.push(packU64(arg.value));
          buffers.push(undefined);
        } else if (arg === null) {
          parameters.push(null);
          buffers.push(undefined);
        } else {
          throw new TypeError(
            "Invalid ffi arg value, expected TypedArray, UnsafePointer or null",
          );
        }
      } else {
        parameters.push(arg);
      }
    }

    return { parameters, buffers };
  }

  class UnsafeFnPointer {
    pointer;
    definition;

    constructor(pointer, definition) {
      this.pointer = pointer;
      this.definition = definition;
    }

    call(...args) {
      const { parameters, buffers } = prepareArgs(
        this.definition.parameters,
        args,
      );
      if (this.definition.nonblocking) {
        const promise = core.opAsync("op_ffi_call_ptr_nonblocking", {
          pointer: packU64(this.pointer.value),
          def: this.definition,
          parameters,
          buffers,
        });

        if (this.definition.result === "pointer") {
          return promise.then((value) => new UnsafePointer(unpackU64(value)));
        }

        return promise;
      } else {
        const result = core.opSync("op_ffi_call_ptr", {
          pointer: packU64(this.pointer.value),
          def: this.definition,
          parameters,
          buffers,
        });

        if (this.definition.result === "pointer") {
          return new UnsafePointer(unpackU64(result));
        }

        return result;
      }
    }
  }

  class DynamicLibrary {
    #rid;
    symbols = {};

    constructor(path, symbols) {
      this.#rid = core.opSync("op_ffi_load", { path, symbols });

      for (const symbol in symbols) {
        const isNonBlocking = symbols[symbol].nonblocking;
        const types = symbols[symbol].parameters;

        this.symbols[symbol] = (...args) => {
          const { parameters, buffers } = prepareArgs(types, args);

          if (isNonBlocking) {
            const promise = core.opAsync("op_ffi_call_nonblocking", {
              rid: this.#rid,
              symbol,
              parameters,
              buffers,
            });

            if (symbols[symbol].result === "pointer") {
              return promise.then((value) =>
                new UnsafePointer(unpackU64(value))
              );
            }

            return promise;
          } else {
            const result = core.opSync("op_ffi_call", {
              rid: this.#rid,
              symbol,
              parameters,
              buffers,
            });

            if (symbols[symbol].result === "pointer") {
              return new UnsafePointer(unpackU64(result));
            }

            return result;
          }
        };
      }
    }

    close() {
      core.close(this.#rid);
    }
  }

  function dlopen(path, symbols) {
    // URL support is progressively enhanced by util in `runtime/js`.
    const pathFromURL = __bootstrap.util.pathFromURL ?? ((p) => p);
    return new DynamicLibrary(pathFromURL(path), symbols);
  }

  window.__bootstrap.ffi = {
    dlopen,
    UnsafePointer,
    UnsafePointerView,
    UnsafeFnPointer,
  };
})(this);
