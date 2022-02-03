// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use super::Result;
use std::{collections::HashMap, path::Path, process::Command, time::Duration};
pub use test_util::{parse_wrk_output, WrkOutput as HttpBenchmarkResult};

// Some of the benchmarks in this file have been renamed. In case the history
// somehow gets messed up:
//   "node_http" was once called "node"
//   "deno_tcp" was once called "deno"
//   "deno_http" was once called "deno_net_http"

const DURATION: &str = "20s";

pub(crate) fn benchmark(
  target_path: &Path,
) -> Result<HashMap<String, HttpBenchmarkResult>> {
  let deno_exe = test_util::deno_exe_path();
  let deno_exe = deno_exe.to_str().unwrap();

  let hyper_hello_exe = target_path.join("test_server");
  let hyper_hello_exe = hyper_hello_exe.to_str().unwrap();

  let core_http_json_ops_exe = target_path.join("examples/http_bench_json_ops");
  let core_http_json_ops_exe = core_http_json_ops_exe.to_str().unwrap();

  let mut res = HashMap::new();

  // "deno_tcp" was once called "deno"
  res.insert("deno_tcp".to_string(), deno_tcp(deno_exe)?);
  // res.insert("deno_udp".to_string(), deno_udp(deno_exe)?);
  res.insert("deno_http".to_string(), deno_http(deno_exe)?);
  res.insert("deno_http_native".to_string(), deno_http_native(deno_exe)?);
  // "core_http_json_ops" previously had a "bin op" counterpart called "core_http_bin_ops",
  // which was previously also called "deno_core_http_bench", "deno_core_single"
  res.insert(
    "core_http_json_ops".to_string(),
    core_http_json_ops(core_http_json_ops_exe)?,
  );
  // "node_http" was once called "node"
  res.insert("node_http".to_string(), node_http()?);
  res.insert("node_tcp".to_string(), node_tcp()?);
  res.insert("hyper".to_string(), hyper_http(hyper_hello_exe)?);

  Ok(res)
}

fn run(
  server_cmd: &[&str],
  port: u16,
  env: Option<Vec<(String, String)>>,
  origin_cmd: Option<&[&str]>,
) -> Result<HttpBenchmarkResult> {
  // Wait for port 4544 to become available.
  // TODO Need to use SO_REUSEPORT with tokio::net::TcpListener.
  std::thread::sleep(Duration::from_secs(5));

  let mut origin = None;
  if let Some(cmd) = origin_cmd {
    let mut com = Command::new(cmd[0]);
    com.args(&cmd[1..]);
    if let Some(env) = env.clone() {
      com.envs(env);
    }
    origin = Some(com.spawn()?);
  };

  println!("{}", server_cmd.join(" "));
  let mut server = {
    let mut com = Command::new(server_cmd[0]);
    com.args(&server_cmd[1..]);
    if let Some(env) = env {
      com.envs(env);
    }
    com.spawn()?
  };

  std::thread::sleep(Duration::from_secs(5)); // wait for server to wake up. TODO racy.

  let wrk = test_util::prebuilt_tool_path("wrk");
  assert!(wrk.is_file());

  let wrk_cmd = &[
    wrk.to_str().unwrap(),
    "-d",
    DURATION,
    "--latency",
    &format!("http://127.0.0.1:{}/", port),
  ];
  println!("{}", wrk_cmd.join(" "));
  let output = test_util::run_collect(wrk_cmd, None, None, None, true).0;

  std::thread::sleep(Duration::from_secs(1)); // wait to capture failure. TODO racy.

  println!("{}", output);
  assert!(
    server.try_wait()?.map_or(true, |s| s.success()),
    "server ended with error"
  );

  server.kill()?;
  if let Some(mut origin) = origin {
    origin.kill()?;
  }

  Ok(parse_wrk_output(&output))
}

fn get_port() -> u16 {
  static mut NEXT_PORT: u16 = 4544;

  let port = unsafe { NEXT_PORT };

  unsafe {
    NEXT_PORT += 1;
  }

  port
}

fn server_addr(port: u16) -> String {
  format!("0.0.0.0:{}", port)
}

fn deno_tcp(deno_exe: &str) -> Result<HttpBenchmarkResult> {
  let port = get_port();
  println!("http_benchmark testing DENO tcp.");
  run(
    &[
      deno_exe,
      "run",
      "--allow-net",
      "cli/bench/deno_tcp.ts",
      &server_addr(port),
    ],
    port,
    None,
    None,
  )
}

fn deno_http(deno_exe: &str) -> Result<HttpBenchmarkResult> {
  let port = get_port();
  println!("http_benchmark testing DENO using net/http.");
  run(
    &[
      deno_exe,
      "run",
      "--allow-net",
      "--reload",
      "--unstable",
      "test_util/std/http/bench.ts",
      &server_addr(port),
    ],
    port,
    None,
    None,
  )
}

fn deno_http_native(deno_exe: &str) -> Result<HttpBenchmarkResult> {
  let port = get_port();
  println!("http_benchmark testing DENO using native bindings.");
  run(
    &[
      deno_exe,
      "run",
      "--allow-net",
      "--reload",
      "cli/bench/deno_http_native.js",
      &server_addr(port),
    ],
    port,
    None,
    None,
  )
}

fn core_http_json_ops(exe: &str) -> Result<HttpBenchmarkResult> {
  println!("http_benchmark testing CORE http_bench_json_ops");
  run(&[exe], 4544, None, None)
}

fn node_http() -> Result<HttpBenchmarkResult> {
  let port = get_port();
  println!("http_benchmark testing NODE.");
  run(
    &["node", "cli/bench/node_http.js", &port.to_string()],
    port,
    None,
    None,
  )
}

fn node_tcp() -> Result<HttpBenchmarkResult> {
  let port = get_port();
  println!("http_benchmark testing node_tcp.js");
  run(
    &["node", "cli/bench/node_tcp.js", &port.to_string()],
    port,
    None,
    None,
  )
}

fn hyper_http(exe: &str) -> Result<HttpBenchmarkResult> {
  let port = get_port();
  println!("http_benchmark testing RUST hyper");
  run(&[exe, &port.to_string()], port, None, None)
}
