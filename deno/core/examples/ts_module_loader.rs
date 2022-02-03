// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.
//! This example shows how to use swc to transpile TypeScript and JSX/TSX
//! modules.
//!
//! It will only transpile, not typecheck (like Deno's `--no-check` flag).

use std::pin::Pin;
use std::rc::Rc;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Error;
use deno_ast::MediaType;
use deno_ast::ParseParams;
use deno_ast::SourceTextInfo;
use deno_core::resolve_import;
use deno_core::resolve_path;
use deno_core::JsRuntime;
use deno_core::ModuleLoader;
use deno_core::ModuleSource;
use deno_core::ModuleSourceFuture;
use deno_core::ModuleSpecifier;
use deno_core::ModuleType;
use deno_core::RuntimeOptions;
use futures::FutureExt;

struct TypescriptModuleLoader;

impl ModuleLoader for TypescriptModuleLoader {
  fn resolve(
    &self,
    specifier: &str,
    referrer: &str,
    _is_main: bool,
  ) -> Result<ModuleSpecifier, Error> {
    Ok(resolve_import(specifier, referrer)?)
  }

  fn load(
    &self,
    module_specifier: &ModuleSpecifier,
    _maybe_referrer: Option<ModuleSpecifier>,
    _is_dyn_import: bool,
  ) -> Pin<Box<ModuleSourceFuture>> {
    let module_specifier = module_specifier.clone();
    async move {
      let path = module_specifier
        .to_file_path()
        .map_err(|_| anyhow!("Only file: URLs are supported."))?;

      let media_type = MediaType::from(&path);
      let (module_type, should_transpile) = match MediaType::from(&path) {
        MediaType::JavaScript | MediaType::Mjs | MediaType::Cjs => {
          (ModuleType::JavaScript, false)
        }
        MediaType::Jsx => (ModuleType::JavaScript, true),
        MediaType::TypeScript
        | MediaType::Mts
        | MediaType::Cts
        | MediaType::Dts
        | MediaType::Dmts
        | MediaType::Dcts
        | MediaType::Tsx => (ModuleType::JavaScript, true),
        MediaType::Json => (ModuleType::Json, false),
        _ => bail!("Unknown extension {:?}", path.extension()),
      };

      let code = std::fs::read_to_string(&path)?;
      let code = if should_transpile {
        let parsed = deno_ast::parse_module(ParseParams {
          specifier: module_specifier.to_string(),
          source: SourceTextInfo::from_string(code),
          media_type,
          capture_tokens: false,
          scope_analysis: false,
          maybe_syntax: None,
        })?;
        parsed.transpile(&Default::default())?.text
      } else {
        code
      };
      let module = ModuleSource {
        code,
        module_type,
        module_url_specified: module_specifier.to_string(),
        module_url_found: module_specifier.to_string(),
      };
      Ok(module)
    }
    .boxed_local()
  }
}

fn main() -> Result<(), Error> {
  let args: Vec<String> = std::env::args().collect();
  if args.len() < 2 {
    println!("Usage: target/examples/debug/ts_module_loader <path_to_module>");
    std::process::exit(1);
  }
  let main_url = args[1].clone();
  println!("Run {}", main_url);

  let mut js_runtime = JsRuntime::new(RuntimeOptions {
    module_loader: Some(Rc::new(TypescriptModuleLoader)),
    ..Default::default()
  });

  let main_module = resolve_path(&main_url)?;

  let future = async move {
    let mod_id = js_runtime.load_main_module(&main_module, None).await?;
    let _ = js_runtime.mod_evaluate(mod_id);
    js_runtime.run_event_loop(false).await?;
    Ok(())
  };

  tokio::runtime::Builder::new_current_thread()
    .enable_all()
    .build()
    .unwrap()
    .block_on(future)
}
