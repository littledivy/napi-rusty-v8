// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
"use strict";

((window) => {
  const core = window.Deno.core;
  const { File } = window.__bootstrap.files;
  const { readAll } = window.__bootstrap.io;
  const { assert, pathFromURL } = window.__bootstrap.util;
  const {
    ArrayPrototypeMap,
    TypeError,
    isNaN,
    ObjectEntries,
    String,
  } = window.__bootstrap.primordials;

  function opKill(pid, signo) {
    core.opSync("op_kill", pid, signo);
  }

  function opRunStatus(rid) {
    return core.opAsync("op_run_status", rid);
  }

  function opRun(request) {
    assert(request.cmd.length > 0);
    return core.opSync("op_run", request);
  }

  async function runStatus(rid) {
    const res = await opRunStatus(rid);

    if (res.gotSignal) {
      const signal = res.exitSignal;
      return { success: false, code: 128 + signal, signal };
    } else if (res.exitCode != 0) {
      return { success: false, code: res.exitCode };
    } else {
      return { success: true, code: 0 };
    }
  }

  class Process {
    constructor(res) {
      this.rid = res.rid;
      this.pid = res.pid;

      if (res.stdinRid && res.stdinRid > 0) {
        this.stdin = new File(res.stdinRid);
      }

      if (res.stdoutRid && res.stdoutRid > 0) {
        this.stdout = new File(res.stdoutRid);
      }

      if (res.stderrRid && res.stderrRid > 0) {
        this.stderr = new File(res.stderrRid);
      }
    }

    status() {
      return runStatus(this.rid);
    }

    async output() {
      if (!this.stdout) {
        throw new TypeError("stdout was not piped");
      }
      try {
        return await readAll(this.stdout);
      } finally {
        this.stdout.close();
      }
    }

    async stderrOutput() {
      if (!this.stderr) {
        throw new TypeError("stderr was not piped");
      }
      try {
        return await readAll(this.stderr);
      } finally {
        this.stderr.close();
      }
    }

    close() {
      core.close(this.rid);
    }

    kill(signo) {
      opKill(this.pid, signo);
    }
  }

  function isRid(arg) {
    return !isNaN(arg);
  }

  function run({
    cmd,
    cwd = undefined,
    clearEnv = false,
    env = {},
    gid = undefined,
    uid = undefined,
    stdout = "inherit",
    stderr = "inherit",
    stdin = "inherit",
  }) {
    if (cmd[0] != null) {
      cmd[0] = pathFromURL(cmd[0]);
    }
    const res = opRun({
      cmd: ArrayPrototypeMap(cmd, String),
      cwd,
      clearEnv,
      env: ObjectEntries(env),
      gid,
      uid,
      stdin: isRid(stdin) ? "" : stdin,
      stdout: isRid(stdout) ? "" : stdout,
      stderr: isRid(stderr) ? "" : stderr,
      stdinRid: isRid(stdin) ? stdin : 0,
      stdoutRid: isRid(stdout) ? stdout : 0,
      stderrRid: isRid(stderr) ? stderr : 0,
    });
    return new Process(res);
  }

  window.__bootstrap.process = {
    run,
    Process,
    kill: opKill,
  };
})(this);
