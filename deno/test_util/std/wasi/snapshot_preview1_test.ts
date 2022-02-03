// Copyright 2018-2021 the Deno authors. All rights reserved. MIT license.
import Context from "./snapshot_preview1.ts";
import { assertEquals, assertThrows } from "../testing/asserts.ts";
import { copy } from "../fs/copy.ts";
import * as path from "../path/mod.ts";
import { readAll, writeAll } from "../streams/conversion.ts";
import { isWindows } from "../_util/os.ts";

const tests = [
  "testdata/std_env_args.wasm",
  "testdata/std_env_vars.wasm",
  "testdata/std_fs_create_dir.wasm",
  "testdata/std_fs_file_create.wasm",
  "testdata/std_fs_file_metadata.wasm",
  "testdata/std_fs_file_seek.wasm",
  "testdata/std_fs_file_set_len.wasm",
  "testdata/std_fs_file_sync_all.wasm",
  "testdata/std_fs_file_sync_data.wasm",
  "testdata/std_fs_hard_link.wasm",
  "testdata/std_fs_metadata.wasm",
  "testdata/std_fs_read.wasm",
  "testdata/std_fs_read_dir.wasm",
  "testdata/std_fs_remove_dir_all.wasm",
  "testdata/std_fs_rename.wasm",
  "testdata/std_fs_symlink_metadata.wasm",
  "testdata/std_fs_write.wasm",
  "testdata/std_io_stderr.wasm",
  "testdata/std_io_stdin.wasm",
  "testdata/std_io_stdout.wasm",
  "testdata/std_process_exit.wasm",
  "testdata/wasi_clock_res_get.wasm",
  "testdata/wasi_clock_time_get.wasm",
  "testdata/wasi_fd_fdstat_get.wasm",
  "testdata/wasi_fd_fdstat_get.wasm",
  "testdata/wasi_fd_fdstat_get.wasm",
  "testdata/wasi_fd_renumber.wasm",
  "testdata/wasi_fd_tell_file.wasm",
  "testdata/wasi_fd_write_file.wasm",
  "testdata/wasi_fd_write_stderr.wasm",
  "testdata/wasi_fd_write_stdout.wasm",
  "testdata/wasi_path_open.wasm",
  "testdata/wasi_proc_exit.wasm",
  "testdata/wasi_random_get.wasm",
  "testdata/wasi_sched_yield.wasm",
];

const ignore = [];

// TODO(caspervonb) investigate why these tests are failing on windows and fix
// them.
// The failing tests all involve symlinks in some way, my best guess so far is
// that there's something going wrong with copying the symlinks over to the
// temporary working directory, but only in some cases.
if (isWindows) {
  ignore.push("testdata/std_fs_metadata.wasm");
  ignore.push("testdata/std_fs_read_dir.wasm");
  ignore.push("testdata/wasi_path_open.wasm");
}

const rootdir = path.dirname(path.fromFileUrl(import.meta.url));
const testdir = path.join(rootdir, "testdata");

for (const pathname of tests) {
  Deno.test({
    name: path.basename(pathname),
    ignore: ignore.includes(pathname),
    fn: async function () {
      const prelude = await Deno.readTextFile(
        path.resolve(rootdir, pathname.replace(/\.wasm$/, ".json")),
      );
      const options = JSON.parse(prelude);

      // TODO(caspervonb) investigate more.
      // On Windows creating a tempdir in the default directory breaks nearly
      // all the tests, possibly due to symlinks pointing to the original file
      // which crosses drive boundaries.
      const workdir = await Deno.makeTempDir({
        dir: testdir,
      });

      await copy(
        path.join(testdir, "fixtures"),
        path.join(workdir, "fixtures"),
      );

      try {
        const process = await Deno.run({
          cwd: workdir,
          cmd: [
            `${Deno.execPath()}`,
            "run",
            "--quiet",
            "--unstable",
            "--allow-all",
            "--no-check",
            path.resolve(rootdir, "snapshot_preview1_test_runner.ts"),
            prelude,
            path.resolve(rootdir, pathname),
          ],
          stdin: "piped",
          stdout: "piped",
          stderr: "piped",
        });

        if (options.stdin) {
          const stdin = new TextEncoder().encode(options.stdin);
          await writeAll(process.stdin, stdin);
        }

        process.stdin.close();

        const stdout = await readAll(process.stdout);

        if (options.stdout) {
          assertEquals(new TextDecoder().decode(stdout), options.stdout);
        } else {
          await writeAll(Deno.stdout, stdout);
        }

        process.stdout.close();

        const stderr = await readAll(process.stderr);

        if (options.stderr) {
          assertEquals(new TextDecoder().decode(stderr), options.stderr);
        } else {
          await writeAll(Deno.stderr, stderr);
        }

        process.stderr.close();

        const status = await process.status();
        assertEquals(status.code, options.exitCode ? +options.exitCode : 0);

        process.close();
      } catch (err) {
        throw err;
      } finally {
        await Deno.remove(workdir, { recursive: true });
      }
    },
  });
}

Deno.test("context_start", function () {
  assertThrows(
    () => {
      const context = new Context({});
      context.start({
        exports: {
          _start() {},
        },
      });
    },
    TypeError,
    "must provide a memory export",
  );

  assertThrows(
    () => {
      const context = new Context({});
      context.start({
        exports: {
          _initialize() {},
          memory: new WebAssembly.Memory({ initial: 1 }),
        },
      });
    },
    TypeError,
    "export _initialize must not be a function",
  );

  assertThrows(
    () => {
      const context = new Context({});
      context.start({
        exports: {
          memory: new WebAssembly.Memory({ initial: 1 }),
        },
      });
    },
    TypeError,
    "export _start must be a function",
  );

  {
    const context = new Context({
      exitOnReturn: false,
    });
    const exitCode = context.start({
      exports: {
        _start() {
        },
        memory: new WebAssembly.Memory({ initial: 1 }),
      },
    });
    assertEquals(exitCode, null);
  }

  {
    const context = new Context({
      exitOnReturn: false,
    });
    const exitCode = context.start({
      exports: {
        _start() {
          const exit = context.exports["proc_exit"] as CallableFunction;
          exit(0);
        },
        memory: new WebAssembly.Memory({ initial: 1 }),
      },
    });
    assertEquals(exitCode, 0);
  }

  assertThrows(
    () => {
      const context = new Context({});
      context.start({
        exports: {
          memory: new WebAssembly.Memory({ initial: 1 }),
          _start() {},
        },
      });
      context.start({
        exports: {},
      });
    },
    Error,
    "WebAssembly.Instance has already started",
  );
});

Deno.test("context_initialize", function () {
  assertThrows(
    () => {
      const context = new Context({});
      context.initialize({
        exports: {
          _initialize() {},
        },
      });
    },
    TypeError,
    "must provide a memory export",
  );

  assertThrows(
    () => {
      const context = new Context({});
      context.initialize({
        exports: {
          _start() {},
          memory: new WebAssembly.Memory({ initial: 1 }),
        },
      });
    },
    TypeError,
    "export _start must not be a function",
  );

  assertThrows(
    () => {
      const context = new Context({});
      context.initialize({
        exports: {
          memory: new WebAssembly.Memory({ initial: 1 }),
        },
      });
    },
    TypeError,
    "export _initialize must be a function",
  );
  assertThrows(
    () => {
      const context = new Context({});
      context.initialize({
        exports: {
          memory: new WebAssembly.Memory({ initial: 1 }),
          _initialize() {},
        },
      });
      context.initialize({
        exports: {},
      });
    },
    Error,
    "WebAssembly.Instance has already started",
  );
});

Deno.test("std_io_stdin.wasm with stdin as file", function () {
  const stdinPath = Deno.makeTempFileSync();
  Deno.writeTextFileSync(stdinPath, "Hello, stdin!");

  const stdinFile = Deno.openSync(stdinPath);

  const context = new Context({
    exitOnReturn: false,
    stdin: stdinFile.rid,
  });

  const binary = Deno.readFileSync(path.join(testdir, "std_io_stdin.wasm"));
  const module = new WebAssembly.Module(binary);
  const instance = new WebAssembly.Instance(module, {
    wasi_snapshot_preview1: context.exports,
  });

  context.start(instance);

  stdinFile.close();
});

Deno.test("std_io_stdout.wasm with stdout as file", function () {
  const stdoutPath = Deno.makeTempFileSync();
  const stdoutFile = Deno.openSync(stdoutPath, { create: true, write: true });

  const context = new Context({
    exitOnReturn: false,
    stdout: stdoutFile.rid,
  });

  const binary = Deno.readFileSync(path.join(testdir, "std_io_stdout.wasm"));
  const module = new WebAssembly.Module(binary);
  const instance = new WebAssembly.Instance(module, {
    wasi_snapshot_preview1: context.exports,
  });

  context.start(instance);

  stdoutFile.close();

  assertEquals(Deno.readTextFileSync(stdoutPath), "Hello, stdout!");
});

Deno.test("std_io_stderr.wasm with stderr as file", function () {
  const stderrPath = Deno.makeTempFileSync();
  const stderrFile = Deno.openSync(stderrPath, { create: true, write: true });

  const context = new Context({
    exitOnReturn: false,
    stderr: stderrFile.rid,
  });

  const binary = Deno.readFileSync(path.join(testdir, "std_io_stderr.wasm"));
  const module = new WebAssembly.Module(binary);
  const instance = new WebAssembly.Instance(module, {
    wasi_snapshot_preview1: context.exports,
  });

  context.start(instance);

  stderrFile.close();

  assertEquals(Deno.readTextFileSync(stderrPath), "Hello, stderr!");
});
