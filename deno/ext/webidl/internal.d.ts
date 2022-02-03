// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
// deno-lint-ignore-file no-explicit-any ban-types

/// <reference no-default-lib="true" />
/// <reference lib="esnext" />

declare namespace globalThis {
  declare namespace __bootstrap {
    declare namespace webidl {
      declare interface ConverterOpts {
        /**
         * The prefix for error messages created by this converter.
         * Examples:
         *    - `Failed to construct 'Event'`
         *    - `Failed to execute 'removeEventListener' on 'EventTarget'`
         */
        prefix: string;
      }
      declare interface ValueConverterOpts extends ConverterOpts {
        /**
         * The context of this value error messages created by this converter.
         * Examples:
         *    - `Argument 1`
         *    - `Argument 3`
         */
        context: string;
      }
      declare function makeException(
        ErrorType: any,
        message: string,
        opts: ValueConverterOpts,
      ): any;
      declare interface IntConverterOpts extends ValueConverterOpts {
        /**
         * Wether to throw if the number is outside of the acceptable values for
         * this type.
         */
        enforceRange?: boolean;
        /**
         * Wether to clamp this number to the acceptable values for this type.
         */
        clamp?: boolean;
      }
      declare interface StringConverterOpts extends ValueConverterOpts {
        /**
         * Wether to treat `null` value as an empty string.
         */
        treatNullAsEmptyString?: boolean;
      }
      declare interface BufferConverterOpts extends ValueConverterOpts {
        /**
         * Wether to allow `SharedArrayBuffer` (not just `ArrayBuffer`).
         */
        allowShared?: boolean;
      }
      declare const converters: {
        any(v: any): any;
        /**
         * Convert a value into a `boolean` (bool).
         */
        boolean(v: any, opts?: IntConverterOpts): boolean;
        /**
         * Convert a value into a `byte` (int8).
         */
        byte(v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `octet` (uint8).
         */
        octet(v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `short` (int16).
         */
        short(v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `unsigned short` (uint16).
         */
        ["unsigned short"](v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `long` (int32).
         */
        long(v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `unsigned long` (uint32).
         */
        ["unsigned long"](v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `long long` (int64).
         * **Note this is truncated to a JS number (53 bit precision).**
         */
        ["long long"](v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `unsigned long long` (uint64).
         * **Note this is truncated to a JS number (53 bit precision).**
         */
        ["unsigned long long"](v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `float` (f32).
         */
        float(v: any, opts?: ValueConverterOpts): number;
        /**
         * Convert a value into a `unrestricted float` (f32, infinity, or NaN).
         */
        ["unrestricted float"](v: any, opts?: ValueConverterOpts): number;
        /**
         * Convert a value into a `double` (f64).
         */
        double(v: any, opts?: ValueConverterOpts): number;
        /**
         * Convert a value into a `unrestricted double` (f64, infinity, or NaN).
         */
        ["unrestricted double"](v: any, opts?: ValueConverterOpts): number;
        /**
         * Convert a value into a `DOMString` (string).
         */
        DOMString(v: any, opts?: StringConverterOpts): string;
        /**
         * Convert a value into a `ByteString` (string with only u8 codepoints).
         */
        ByteString(v: any, opts?: StringConverterOpts): string;
        /**
         * Convert a value into a `USVString` (string with only valid non
         * surrogate Unicode code points).
         */
        USVString(v: any, opts?: StringConverterOpts): string;
        /**
         * Convert a value into an `object` (object).
         */
        object(v: any, opts?: ValueConverterOpts): object;
        /**
         * Convert a value into an `ArrayBuffer` (ArrayBuffer).
         */
        ArrayBuffer(v: any, opts?: BufferConverterOpts): ArrayBuffer;
        /**
         * Convert a value into a `DataView` (ArrayBuffer).
         */
        DataView(v: any, opts?: BufferConverterOpts): DataView;
        /**
         * Convert a value into a `Int8Array` (Int8Array).
         */
        Int8Array(v: any, opts?: BufferConverterOpts): Int8Array;
        /**
         * Convert a value into a `Int16Array` (Int16Array).
         */
        Int16Array(v: any, opts?: BufferConverterOpts): Int16Array;
        /**
         * Convert a value into a `Int32Array` (Int32Array).
         */
        Int32Array(v: any, opts?: BufferConverterOpts): Int32Array;
        /**
         * Convert a value into a `Uint8Array` (Uint8Array).
         */
        Uint8Array(v: any, opts?: BufferConverterOpts): Uint8Array;
        /**
         * Convert a value into a `Uint16Array` (Uint16Array).
         */
        Uint16Array(v: any, opts?: BufferConverterOpts): Uint16Array;
        /**
         * Convert a value into a `Uint32Array` (Uint32Array).
         */
        Uint32Array(v: any, opts?: BufferConverterOpts): Uint32Array;
        /**
         * Convert a value into a `Uint8ClampedArray` (Uint8ClampedArray).
         */
        Uint8ClampedArray(
          v: any,
          opts?: BufferConverterOpts,
        ): Uint8ClampedArray;
        /**
         * Convert a value into a `Float32Array` (Float32Array).
         */
        Float32Array(v: any, opts?: BufferConverterOpts): Float32Array;
        /**
         * Convert a value into a `Float64Array` (Float64Array).
         */
        Float64Array(v: any, opts?: BufferConverterOpts): Float64Array;
        /**
         * Convert a value into an `ArrayBufferView` (ArrayBufferView).
         */
        ArrayBufferView(v: any, opts?: BufferConverterOpts): ArrayBufferView;
        /**
         * Convert a value into a `BufferSource` (ArrayBuffer or ArrayBufferView).
         */
        BufferSource(
          v: any,
          opts?: BufferConverterOpts,
        ): ArrayBuffer | ArrayBufferView;
        /**
         * Convert a value into a `DOMTimeStamp` (u64). Alias for unsigned long long
         */
        DOMTimeStamp(v: any, opts?: IntConverterOpts): number;
        /**
         * Convert a value into a `Function` ((...args: any[]) => any).
         */
        Function(v: any, opts?: ValueConverterOpts): (...args: any) => any;
        /**
         * Convert a value into a `VoidFunction` (() => void).
         */
        VoidFunction(v: any, opts?: ValueConverterOpts): () => void;
        ["UVString?"](v: any, opts?: ValueConverterOpts): string | null;
        ["sequence<double>"](v: any, opts?: ValueConverterOpts): number[];

        [type: string]: (v: any, opts: ValueConverterOpts) => any;
      };

      /**
       * Assert that the a function has at least a required amount of arguments.
       */
      declare function requiredArguments(
        length: number,
        required: number,
        opts: ConverterOpts,
      ): void;
      declare type Dictionary = DictionaryMember[];
      declare interface DictionaryMember {
        key: string;
        converter: (v: any, opts: ValueConverterOpts) => any;
        defaultValue?: any;
        required?: boolean;
      }

      /**
       * Create a converter for dictionaries.
       */
      declare function createDictionaryConverter<T>(
        name: string,
        ...dictionaries: Dictionary[]
      ): (v: any, opts: ValueConverterOpts) => T;

      /**
       * Create a converter for enums.
       */
      declare function createEnumConverter(
        name: string,
        values: string[],
      ): (v: any, opts: ValueConverterOpts) => string;

      /**
       * Create a converter that makes the contained type nullable.
       */
      declare function createNullableConverter<T>(
        converter: (v: any, opts: ValueConverterOpts) => T,
      ): (v: any, opts: ValueConverterOpts) => T | null;

      /**
       * Create a converter that converts a sequence of the inner type.
       */
      declare function createSequenceConverter<T>(
        converter: (v: any, opts: ValueConverterOpts) => T,
      ): (v: any, opts: ValueConverterOpts) => T[];

      /**
       * Create a converter that converts a Promise of the inner type.
       */
      declare function createPromiseConverter<T>(
        converter: (v: any, opts: ValueConverterOpts) => T,
      ): (v: any, opts: ValueConverterOpts) => Promise<T>;

      /**
       * Invoke a callback function.
       */
      declare function invokeCallbackFunction<T>(
        callable: (...args: any) => any,
        args: any[],
        thisArg: any,
        returnValueConverter: (v: any, opts: ValueConverterOpts) => T,
        opts: ConverterOpts & { returnsPromise?: boolean },
      ): T;

      /**
       * Throw an illegal constructor error.
       */
      declare function illegalConstructor(): never;

      /**
       * The branding symbol.
       */
      declare const brand: unique symbol;

      /**
       * Create a branded instance of an interface.
       */
      declare function createBranded(self: any): any;

      /**
       * Assert that self is branded.
       */
      declare function assertBranded(self: any, type: any): void;

      /**
       * Create a converter for interfaces.
       */
      declare function createInterfaceConverter(
        name: string,
        prototype: any,
      ): (v: any, opts: ValueConverterOpts) => any;

      declare function createRecordConverter<
        K extends string | number | symbol,
        V,
      >(
        keyConverter: (v: any, opts: ValueConverterOpts) => K,
        valueConverter: (v: any, opts: ValueConverterOpts) => V,
      ): (
        v: Record<K, V>,
        opts: ValueConverterOpts,
      ) => any;

      /**
       * Mix in the iterable declarations defined in WebIDL.
       * https://heycam.github.io/webidl/#es-iterable
       */
      declare function mixinPairIterable(
        name: string,
        prototype: any,
        dataSymbol: symbol,
        keyKey: string | number | symbol,
        valueKey: string | number | symbol,
      ): void;

      /**
       * Configure prototype properties enumerability / writability / configurability.
       */
      declare function configurePrototype(prototype: any);

      /**
       * Get the WebIDL / ES type of a value.
       */
      declare function type(
        v: any,
      ):
        | "Null"
        | "Undefined"
        | "Boolean"
        | "Number"
        | "String"
        | "Symbol"
        | "BigInt"
        | "Object";
    }
  }
}
