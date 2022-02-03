// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use crate::cache;
use crate::cache::CacherLoader;
use crate::colors;
use crate::compat;
use crate::create_main_worker;
use crate::emit;
use crate::file_fetcher::File;
use crate::file_watcher;
use crate::file_watcher::ResolutionResult;
use crate::flags::CheckFlag;
use crate::flags::Flags;
use crate::flags::TestFlags;
use crate::fs_util::collect_specifiers;
use crate::fs_util::is_supported_test_ext;
use crate::fs_util::is_supported_test_path;
use crate::graph_util::contains_specifier;
use crate::graph_util::graph_valid;
use crate::located_script_name;
use crate::lockfile;
use crate::ops;
use crate::proc_state::ProcState;
use crate::resolver::ImportMapResolver;
use crate::resolver::JsxResolver;
use crate::tools::coverage::CoverageCollector;

use deno_ast::swc::common::comments::CommentKind;
use deno_ast::MediaType;
use deno_core::error::generic_error;
use deno_core::error::AnyError;
use deno_core::futures::future;
use deno_core::futures::stream;
use deno_core::futures::FutureExt;
use deno_core::futures::StreamExt;
use deno_core::serde_json::json;
use deno_core::ModuleSpecifier;
use deno_graph::ModuleKind;
use deno_runtime::permissions::Permissions;
use deno_runtime::tokio_util::run_basic;
use log::Level;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::collections::HashSet;
use std::io::Write;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

/// The test mode is used to determine how a specifier is to be tested.
#[derive(Debug, Clone, PartialEq)]
enum TestMode {
  /// Test as documentation, type-checking fenced code blocks.
  Documentation,
  /// Test as an executable module, loading the module into the isolate and running each test it
  /// defines.
  Executable,
  /// Test as both documentation and an executable module.
  Both,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct TestDescription {
  pub origin: String,
  pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestOutput {
  // TODO(caspervonb): add stdout and stderr redirection.
  Console(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestResult {
  Ok,
  Ignored,
  Failed(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStepDescription {
  pub test: TestDescription,
  pub level: usize,
  pub name: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestStepResult {
  Ok,
  Ignored,
  Failed(Option<String>),
  Pending(Option<String>),
}

impl TestStepResult {
  fn error(&self) -> Option<&str> {
    match self {
      TestStepResult::Failed(Some(text)) => Some(text.as_str()),
      TestStepResult::Pending(Some(text)) => Some(text.as_str()),
      _ => None,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestPlan {
  pub origin: String,
  pub total: usize,
  pub filtered_out: usize,
  pub used_only: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TestEvent {
  Plan(TestPlan),
  Wait(TestDescription),
  Output(TestOutput),
  Result(TestDescription, TestResult, u64),
  StepWait(TestStepDescription),
  StepResult(TestStepDescription, TestStepResult, u64),
}

#[derive(Debug, Clone, Deserialize)]
pub struct TestSummary {
  pub total: usize,
  pub passed: usize,
  pub failed: usize,
  pub ignored: usize,
  pub passed_steps: usize,
  pub failed_steps: usize,
  pub pending_steps: usize,
  pub ignored_steps: usize,
  pub filtered_out: usize,
  pub measured: usize,
  pub failures: Vec<(TestDescription, String)>,
}

#[derive(Debug, Clone, Deserialize)]
struct TestSpecifierOptions {
  compat_mode: bool,
  concurrent_jobs: NonZeroUsize,
  fail_fast: Option<NonZeroUsize>,
  filter: Option<String>,
  shuffle: Option<u64>,
}

impl TestSummary {
  fn new() -> TestSummary {
    TestSummary {
      total: 0,
      passed: 0,
      failed: 0,
      ignored: 0,
      passed_steps: 0,
      failed_steps: 0,
      pending_steps: 0,
      ignored_steps: 0,
      filtered_out: 0,
      measured: 0,
      failures: Vec::new(),
    }
  }

  fn has_failed(&self) -> bool {
    self.failed > 0 || !self.failures.is_empty()
  }

  fn has_pending(&self) -> bool {
    self.total - self.passed - self.failed - self.ignored > 0
  }
}

trait TestReporter {
  fn report_plan(&mut self, plan: &TestPlan);
  fn report_wait(&mut self, description: &TestDescription);
  fn report_output(&mut self, output: &TestOutput);
  fn report_result(
    &mut self,
    description: &TestDescription,
    result: &TestResult,
    elapsed: u64,
  );
  fn report_step_wait(&mut self, description: &TestStepDescription);
  fn report_step_result(
    &mut self,
    description: &TestStepDescription,
    result: &TestStepResult,
    elapsed: u64,
  );
  fn report_summary(&mut self, summary: &TestSummary, elapsed: &Duration);
}

enum DeferredStepOutput {
  StepWait(TestStepDescription),
  StepResult(TestStepDescription, TestStepResult, u64),
}

struct PrettyTestReporter {
  concurrent: bool,
  echo_output: bool,
  deferred_step_output: HashMap<TestDescription, Vec<DeferredStepOutput>>,
  last_wait_output_level: usize,
}

impl PrettyTestReporter {
  fn new(concurrent: bool, echo_output: bool) -> PrettyTestReporter {
    PrettyTestReporter {
      concurrent,
      echo_output,
      deferred_step_output: HashMap::new(),
      last_wait_output_level: 0,
    }
  }

  fn force_report_wait(&mut self, description: &TestDescription) {
    print!("test {} ...", description.name);
    // flush for faster feedback when line buffered
    std::io::stdout().flush().unwrap();
    self.last_wait_output_level = 0;
  }

  fn force_report_step_wait(&mut self, description: &TestStepDescription) {
    if self.last_wait_output_level < description.level {
      println!();
    }
    print!(
      "{}test {} ...",
      "  ".repeat(description.level),
      description.name
    );
    // flush for faster feedback when line buffered
    std::io::stdout().flush().unwrap();
    self.last_wait_output_level = description.level;
  }

  fn force_report_step_result(
    &mut self,
    description: &TestStepDescription,
    result: &TestStepResult,
    elapsed: u64,
  ) {
    let status = match result {
      TestStepResult::Ok => colors::green("ok").to_string(),
      TestStepResult::Ignored => colors::yellow("ignored").to_string(),
      TestStepResult::Pending(_) => colors::gray("pending").to_string(),
      TestStepResult::Failed(_) => colors::red("FAILED").to_string(),
    };

    if self.last_wait_output_level == description.level {
      print!(" ");
    } else {
      print!("{}", "  ".repeat(description.level));
    }

    println!("{} {}", status, colors::gray(human_elapsed(elapsed.into())));

    if let Some(error_text) = result.error() {
      for line in error_text.lines() {
        println!("{}{}", "  ".repeat(description.level + 1), line);
      }
    }
  }
}

/// A function that converts a milisecond elapsed time to a string that
/// represents a human readable version of that time.
fn human_elapsed(elapsed: u128) -> String {
  if elapsed < 1_000 {
    return format!("({}ms)", elapsed);
  }
  if elapsed < 1_000 * 60 {
    return format!("({}s)", elapsed / 1000);
  }

  let seconds = elapsed / 1_000;
  let minutes = seconds / 60;
  let seconds_remainder = seconds % 60;
  format!("({}m{}s)", minutes, seconds_remainder)
}

impl TestReporter for PrettyTestReporter {
  fn report_plan(&mut self, plan: &TestPlan) {
    let inflection = if plan.total == 1 { "test" } else { "tests" };
    println!("running {} {} from {}", plan.total, inflection, plan.origin);
  }

  fn report_wait(&mut self, description: &TestDescription) {
    if !self.concurrent {
      self.force_report_wait(description);
    }
  }

  fn report_output(&mut self, output: &TestOutput) {
    if self.echo_output {
      match output {
        TestOutput::Console(line) => println!("{}", line),
      }
    }
  }

  fn report_result(
    &mut self,
    description: &TestDescription,
    result: &TestResult,
    elapsed: u64,
  ) {
    if self.concurrent {
      self.force_report_wait(description);

      if let Some(step_outputs) = self.deferred_step_output.remove(description)
      {
        for step_output in step_outputs {
          match step_output {
            DeferredStepOutput::StepWait(description) => {
              self.force_report_step_wait(&description)
            }
            DeferredStepOutput::StepResult(
              step_description,
              step_result,
              elapsed,
            ) => self.force_report_step_result(
              &step_description,
              &step_result,
              elapsed,
            ),
          }
        }
      }
    }

    let status = match result {
      TestResult::Ok => colors::green("ok").to_string(),
      TestResult::Ignored => colors::yellow("ignored").to_string(),
      TestResult::Failed(_) => colors::red("FAILED").to_string(),
    };

    if self.last_wait_output_level == 0 {
      print!(" ");
    }

    println!("{} {}", status, colors::gray(human_elapsed(elapsed.into())));
  }

  fn report_step_wait(&mut self, description: &TestStepDescription) {
    if self.concurrent {
      self
        .deferred_step_output
        .entry(description.test.to_owned())
        .or_insert_with(Vec::new)
        .push(DeferredStepOutput::StepWait(description.clone()));
    } else {
      self.force_report_step_wait(description);
    }
  }

  fn report_step_result(
    &mut self,
    description: &TestStepDescription,
    result: &TestStepResult,
    elapsed: u64,
  ) {
    if self.concurrent {
      self
        .deferred_step_output
        .entry(description.test.to_owned())
        .or_insert_with(Vec::new)
        .push(DeferredStepOutput::StepResult(
          description.clone(),
          result.clone(),
          elapsed,
        ));
    } else {
      self.force_report_step_result(description, result, elapsed);
    }
  }

  fn report_summary(&mut self, summary: &TestSummary, elapsed: &Duration) {
    if !summary.failures.is_empty() {
      println!("\nfailures:\n");
      for (description, error) in &summary.failures {
        println!("{}", description.name);
        println!("{}", error);
        println!();
      }

      println!("failures:\n");
      for (description, _) in &summary.failures {
        println!("\t{}", description.name);
      }
    }

    let status = if summary.has_failed() || summary.has_pending() {
      colors::red("FAILED").to_string()
    } else {
      colors::green("ok").to_string()
    };

    let get_steps_text = |count: usize| -> String {
      if count == 0 {
        String::new()
      } else if count == 1 {
        " (1 step)".to_string()
      } else {
        format!(" ({} steps)", count)
      }
    };
    println!(
      "\ntest result: {}. {} passed{}; {} failed{}; {} ignored{}; {} measured; {} filtered out {}\n",
      status,
      summary.passed,
      get_steps_text(summary.passed_steps),
      summary.failed,
      get_steps_text(summary.failed_steps + summary.pending_steps),
      summary.ignored,
      get_steps_text(summary.ignored_steps),
      summary.measured,
      summary.filtered_out,
      colors::gray(human_elapsed(elapsed.as_millis())),
    );
  }
}

fn create_reporter(
  concurrent: bool,
  echo_output: bool,
) -> Box<dyn TestReporter + Send> {
  Box::new(PrettyTestReporter::new(concurrent, echo_output))
}

/// Test a single specifier as documentation containing test programs, an executable test module or
/// both.
async fn test_specifier(
  ps: ProcState,
  permissions: Permissions,
  specifier: ModuleSpecifier,
  mode: TestMode,
  channel: Sender<TestEvent>,
  options: TestSpecifierOptions,
) -> Result<(), AnyError> {
  let mut worker = create_main_worker(
    &ps,
    specifier.clone(),
    permissions,
    vec![ops::testing::init(channel.clone())],
  );

  let mut maybe_coverage_collector = if let Some(ref coverage_dir) =
    ps.coverage_dir
  {
    let session = worker.create_inspector_session().await;
    let coverage_dir = PathBuf::from(coverage_dir);
    let mut coverage_collector = CoverageCollector::new(coverage_dir, session);
    worker
      .with_event_loop(coverage_collector.start_collecting().boxed_local())
      .await?;

    Some(coverage_collector)
  } else {
    None
  };

  // We only execute the specifier as a module if it is tagged with TestMode::Module or
  // TestMode::Both.
  if mode != TestMode::Documentation {
    if options.compat_mode {
      worker.execute_side_module(&compat::GLOBAL_URL).await?;
      worker.execute_side_module(&compat::MODULE_URL).await?;

      let use_esm_loader = compat::check_if_should_use_esm_loader(&specifier)?;

      if use_esm_loader {
        worker.execute_side_module(&specifier).await?;
      } else {
        compat::load_cjs_module(
          &mut worker.js_runtime,
          &specifier.to_file_path().unwrap().display().to_string(),
          false,
        )?;
        worker.run_event_loop(false).await?;
      }
    } else {
      // We execute the module module as a side module so that import.meta.main is not set.
      worker.execute_side_module(&specifier).await?;
    }
  }

  worker.dispatch_load_event(&located_script_name!())?;

  let test_result = worker.js_runtime.execute_script(
    &located_script_name!(),
    &format!(
      r#"Deno[Deno.internal].runTests({})"#,
      json!({
        "filter": options.filter,
        "shuffle": options.shuffle,
      }),
    ),
  )?;

  worker.js_runtime.resolve_value(test_result).await?;

  worker.dispatch_unload_event(&located_script_name!())?;

  if let Some(coverage_collector) = maybe_coverage_collector.as_mut() {
    worker
      .with_event_loop(coverage_collector.stop_collecting().boxed_local())
      .await?;
  }

  Ok(())
}

fn extract_files_from_regex_blocks(
  specifier: &ModuleSpecifier,
  source: &str,
  media_type: MediaType,
  file_line_index: usize,
  blocks_regex: &Regex,
  lines_regex: &Regex,
) -> Result<Vec<File>, AnyError> {
  let files = blocks_regex
    .captures_iter(source)
    .filter_map(|block| {
      let maybe_attributes: Option<Vec<_>> = block
        .get(1)
        .map(|attributes| attributes.as_str().split(' ').collect());

      let file_media_type = if let Some(attributes) = maybe_attributes {
        if attributes.contains(&"ignore") {
          return None;
        }

        match attributes.get(0) {
          Some(&"js") => MediaType::JavaScript,
          Some(&"mjs") => MediaType::Mjs,
          Some(&"cjs") => MediaType::Cjs,
          Some(&"jsx") => MediaType::Jsx,
          Some(&"ts") => MediaType::TypeScript,
          Some(&"mts") => MediaType::Mts,
          Some(&"cts") => MediaType::Cts,
          Some(&"tsx") => MediaType::Tsx,
          Some(&"") => media_type,
          _ => MediaType::Unknown,
        }
      } else {
        media_type
      };

      if file_media_type == MediaType::Unknown {
        return None;
      }

      let line_offset = source[0..block.get(0).unwrap().start()]
        .chars()
        .filter(|c| *c == '\n')
        .count();

      let line_count = block.get(0).unwrap().as_str().split('\n').count();

      let body = block.get(2).unwrap();
      let text = body.as_str();

      // TODO(caspervonb) generate an inline source map
      let mut file_source = String::new();
      for line in lines_regex.captures_iter(text) {
        let text = line.get(1).unwrap();
        file_source.push_str(&format!("{}\n", text.as_str()));
      }

      file_source.push_str("export {};");

      let file_specifier = deno_core::resolve_url_or_path(&format!(
        "{}${}-{}{}",
        specifier,
        file_line_index + line_offset + 1,
        file_line_index + line_offset + line_count + 1,
        file_media_type.as_ts_extension(),
      ))
      .unwrap();

      Some(File {
        local: file_specifier.to_file_path().unwrap(),
        maybe_types: None,
        media_type: file_media_type,
        source: Arc::new(file_source),
        specifier: file_specifier,
        maybe_headers: None,
      })
    })
    .collect();

  Ok(files)
}

fn extract_files_from_source_comments(
  specifier: &ModuleSpecifier,
  source: Arc<String>,
  media_type: MediaType,
) -> Result<Vec<File>, AnyError> {
  let parsed_source = deno_ast::parse_module(deno_ast::ParseParams {
    specifier: specifier.as_str().to_string(),
    source: deno_ast::SourceTextInfo::new(source),
    media_type,
    capture_tokens: false,
    maybe_syntax: None,
    scope_analysis: false,
  })?;
  let comments = parsed_source.comments().get_vec();
  let blocks_regex = Regex::new(r"```([^\r\n]*)\r?\n([\S\s]*?)```")?;
  let lines_regex = Regex::new(r"(?:\* ?)(?:\# ?)?(.*)")?;

  let files = comments
    .iter()
    .filter(|comment| {
      if comment.kind != CommentKind::Block || !comment.text.starts_with('*') {
        return false;
      }

      true
    })
    .flat_map(|comment| {
      extract_files_from_regex_blocks(
        specifier,
        &comment.text,
        media_type,
        parsed_source.source().line_index(comment.span.lo),
        &blocks_regex,
        &lines_regex,
      )
    })
    .flatten()
    .collect();

  Ok(files)
}

fn extract_files_from_fenced_blocks(
  specifier: &ModuleSpecifier,
  source: &str,
  media_type: MediaType,
) -> Result<Vec<File>, AnyError> {
  let blocks_regex = Regex::new(r"```([^\r\n]*)\r?\n([\S\s]*?)```")?;
  let lines_regex = Regex::new(r"(?:\# ?)?(.*)")?;

  extract_files_from_regex_blocks(
    specifier,
    source,
    media_type,
    /* file line index */ 0,
    &blocks_regex,
    &lines_regex,
  )
}

async fn fetch_inline_files(
  ps: ProcState,
  specifiers: Vec<ModuleSpecifier>,
) -> Result<Vec<File>, AnyError> {
  let mut files = Vec::new();
  for specifier in specifiers {
    let mut fetch_permissions = Permissions::allow_all();
    let file = ps
      .file_fetcher
      .fetch(&specifier, &mut fetch_permissions)
      .await?;

    let inline_files = if file.media_type == MediaType::Unknown {
      extract_files_from_fenced_blocks(
        &file.specifier,
        &file.source,
        file.media_type,
      )
    } else {
      extract_files_from_source_comments(
        &file.specifier,
        file.source.clone(),
        file.media_type,
      )
    };

    files.extend(inline_files?);
  }

  Ok(files)
}

/// Type check a collection of module and document specifiers.
async fn check_specifiers(
  ps: ProcState,
  permissions: Permissions,
  specifiers: Vec<(ModuleSpecifier, TestMode)>,
  lib: emit::TypeLib,
) -> Result<(), AnyError> {
  let inline_files = fetch_inline_files(
    ps.clone(),
    specifiers
      .iter()
      .filter_map(|(specifier, mode)| {
        if *mode != TestMode::Executable {
          Some(specifier.clone())
        } else {
          None
        }
      })
      .collect(),
  )
  .await?;

  if !inline_files.is_empty() {
    let specifiers = inline_files
      .iter()
      .map(|file| (file.specifier.clone(), ModuleKind::Esm))
      .collect();

    for file in inline_files {
      ps.file_fetcher.insert_cached(file);
    }

    ps.prepare_module_load(
      specifiers,
      false,
      lib.clone(),
      Permissions::allow_all(),
      permissions.clone(),
      false,
    )
    .await?;
  }

  let module_specifiers = specifiers
    .iter()
    .filter_map(|(specifier, mode)| {
      if *mode != TestMode::Documentation {
        Some((specifier.clone(), ModuleKind::Esm))
      } else {
        None
      }
    })
    .collect();

  ps.prepare_module_load(
    module_specifiers,
    false,
    lib,
    Permissions::allow_all(),
    permissions,
    true,
  )
  .await?;

  Ok(())
}

/// Test a collection of specifiers with test modes concurrently.
async fn test_specifiers(
  ps: ProcState,
  permissions: Permissions,
  specifiers_with_mode: Vec<(ModuleSpecifier, TestMode)>,
  options: TestSpecifierOptions,
) -> Result<(), AnyError> {
  let log_level = ps.flags.log_level;
  let specifiers_with_mode = if let Some(seed) = options.shuffle {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut specifiers_with_mode = specifiers_with_mode.clone();
    specifiers_with_mode.sort_by_key(|(specifier, _)| specifier.clone());
    specifiers_with_mode.shuffle(&mut rng);
    specifiers_with_mode
  } else {
    specifiers_with_mode
  };

  let (sender, receiver) = channel::<TestEvent>();
  let concurrent_jobs = options.concurrent_jobs;
  let fail_fast = options.fail_fast;

  let join_handles =
    specifiers_with_mode.iter().map(move |(specifier, mode)| {
      let ps = ps.clone();
      let permissions = permissions.clone();
      let specifier = specifier.clone();
      let mode = mode.clone();
      let sender = sender.clone();
      let options = options.clone();

      tokio::task::spawn_blocking(move || {
        let join_handle = std::thread::spawn(move || {
          let future =
            test_specifier(ps, permissions, specifier, mode, sender, options);

          run_basic(future)
        });

        join_handle.join().unwrap()
      })
    });

  let join_stream = stream::iter(join_handles)
    .buffer_unordered(concurrent_jobs.get())
    .collect::<Vec<Result<Result<(), AnyError>, tokio::task::JoinError>>>();

  let mut reporter =
    create_reporter(concurrent_jobs.get() > 1, log_level != Some(Level::Error));

  let handler = {
    tokio::task::spawn_blocking(move || {
      let earlier = Instant::now();
      let mut summary = TestSummary::new();
      let mut used_only = false;

      for event in receiver.iter() {
        match event {
          TestEvent::Plan(plan) => {
            summary.total += plan.total;
            summary.filtered_out += plan.filtered_out;

            if plan.used_only {
              used_only = true;
            }

            reporter.report_plan(&plan);
          }

          TestEvent::Wait(description) => {
            reporter.report_wait(&description);
          }

          TestEvent::Output(output) => {
            reporter.report_output(&output);
          }

          TestEvent::Result(description, result, elapsed) => {
            match &result {
              TestResult::Ok => {
                summary.passed += 1;
              }
              TestResult::Ignored => {
                summary.ignored += 1;
              }
              TestResult::Failed(error) => {
                summary.failed += 1;
                summary.failures.push((description.clone(), error.clone()));
              }
            }

            reporter.report_result(&description, &result, elapsed);
          }

          TestEvent::StepWait(description) => {
            reporter.report_step_wait(&description);
          }

          TestEvent::StepResult(description, result, duration) => {
            match &result {
              TestStepResult::Ok => {
                summary.passed_steps += 1;
              }
              TestStepResult::Ignored => {
                summary.ignored_steps += 1;
              }
              TestStepResult::Failed(_) => {
                summary.failed_steps += 1;
              }
              TestStepResult::Pending(_) => {
                summary.pending_steps += 1;
              }
            }

            reporter.report_step_result(&description, &result, duration);
          }
        }

        if let Some(x) = fail_fast {
          if summary.failed >= x.get() {
            break;
          }
        }
      }

      let elapsed = Instant::now().duration_since(earlier);
      reporter.report_summary(&summary, &elapsed);

      if used_only {
        return Err(generic_error(
          "Test failed because the \"only\" option was used",
        ));
      }

      if summary.failed > 0 {
        return Err(generic_error("Test failed"));
      }

      Ok(())
    })
  };

  let (join_results, result) = future::join(join_stream, handler).await;

  // propagate any errors
  for join_result in join_results {
    join_result??;
  }

  result??;

  Ok(())
}

/// Collects specifiers marking them with the appropriate test mode while maintaining the natural
/// input order.
///
/// - Specifiers matching the `is_supported_test_ext` predicate are marked as
/// `TestMode::Documentation`.
/// - Specifiers matching the `is_supported_test_path` are marked as `TestMode::Executable`.
/// - Specifiers matching both predicates are marked as `TestMode::Both`
fn collect_specifiers_with_test_mode(
  include: Vec<String>,
  ignore: Vec<PathBuf>,
  include_inline: bool,
) -> Result<Vec<(ModuleSpecifier, TestMode)>, AnyError> {
  let module_specifiers =
    collect_specifiers(include.clone(), &ignore, is_supported_test_path)?;

  if include_inline {
    return collect_specifiers(include, &ignore, is_supported_test_ext).map(
      |specifiers| {
        specifiers
          .into_iter()
          .map(|specifier| {
            let mode = if module_specifiers.contains(&specifier) {
              TestMode::Both
            } else {
              TestMode::Documentation
            };

            (specifier, mode)
          })
          .collect()
      },
    );
  }

  let specifiers_with_mode = module_specifiers
    .into_iter()
    .map(|specifier| (specifier, TestMode::Executable))
    .collect();

  Ok(specifiers_with_mode)
}

/// Collects module and document specifiers with test modes via
/// `collect_specifiers_with_test_mode` which are then pre-fetched and adjusted
/// based on the media type.
///
/// Specifiers that do not have a known media type that can be executed as a
/// module are marked as `TestMode::Documentation`. Type definition files
/// cannot be run, and therefore need to be marked as `TestMode::Documentation`
/// as well.
async fn fetch_specifiers_with_test_mode(
  ps: ProcState,
  include: Vec<String>,
  ignore: Vec<PathBuf>,
  include_inline: bool,
) -> Result<Vec<(ModuleSpecifier, TestMode)>, AnyError> {
  let mut specifiers_with_mode =
    collect_specifiers_with_test_mode(include, ignore, include_inline)?;
  for (specifier, mode) in &mut specifiers_with_mode {
    let file = ps
      .file_fetcher
      .fetch(specifier, &mut Permissions::allow_all())
      .await?;

    if file.media_type == MediaType::Unknown
      || file.media_type == MediaType::Dts
    {
      *mode = TestMode::Documentation
    }
  }

  Ok(specifiers_with_mode)
}

pub async fn run_tests(
  flags: Flags,
  test_flags: TestFlags,
) -> Result<(), AnyError> {
  let ps = ProcState::build(flags.clone()).await?;
  let permissions = Permissions::from_options(&flags.clone().into());
  let specifiers_with_mode = fetch_specifiers_with_test_mode(
    ps.clone(),
    test_flags.include.unwrap_or_else(|| vec![".".to_string()]),
    test_flags.ignore.clone(),
    test_flags.doc,
  )
  .await?;

  if !test_flags.allow_none && specifiers_with_mode.is_empty() {
    return Err(generic_error("No test modules found"));
  }

  let lib = if flags.unstable {
    emit::TypeLib::UnstableDenoWindow
  } else {
    emit::TypeLib::DenoWindow
  };

  check_specifiers(
    ps.clone(),
    permissions.clone(),
    specifiers_with_mode.clone(),
    lib,
  )
  .await?;

  if test_flags.no_run {
    return Ok(());
  }

  test_specifiers(
    ps,
    permissions,
    specifiers_with_mode,
    TestSpecifierOptions {
      compat_mode: flags.compat,
      concurrent_jobs: test_flags.concurrent_jobs,
      fail_fast: test_flags.fail_fast,
      filter: test_flags.filter,
      shuffle: test_flags.shuffle,
    },
  )
  .await?;

  Ok(())
}

pub async fn run_tests_with_watch(
  flags: Flags,
  test_flags: TestFlags,
) -> Result<(), AnyError> {
  let ps = ProcState::build(flags.clone()).await?;
  let permissions = Permissions::from_options(&flags.clone().into());

  let lib = if flags.unstable {
    emit::TypeLib::UnstableDenoWindow
  } else {
    emit::TypeLib::DenoWindow
  };

  let include = test_flags.include.unwrap_or_else(|| vec![".".to_string()]);
  let ignore = test_flags.ignore.clone();
  let paths_to_watch: Vec<_> = include.iter().map(PathBuf::from).collect();
  let no_check = ps.flags.check == CheckFlag::None;

  let resolver = |changed: Option<Vec<PathBuf>>| {
    let mut cache = cache::FetchCacher::new(
      ps.dir.gen_cache.clone(),
      ps.file_fetcher.clone(),
      Permissions::allow_all(),
      Permissions::allow_all(),
    );

    let paths_to_watch = paths_to_watch.clone();
    let paths_to_watch_clone = paths_to_watch.clone();

    let maybe_import_map_resolver =
      ps.maybe_import_map.clone().map(ImportMapResolver::new);
    let maybe_jsx_resolver = ps
      .maybe_config_file
      .as_ref()
      .map(|cf| {
        cf.to_maybe_jsx_import_source_module()
          .map(|im| JsxResolver::new(im, maybe_import_map_resolver.clone()))
      })
      .flatten();
    let maybe_locker = lockfile::as_maybe_locker(ps.lockfile.clone());
    let maybe_imports = ps
      .maybe_config_file
      .as_ref()
      .map(|cf| cf.to_maybe_imports());
    let files_changed = changed.is_some();
    let include = include.clone();
    let ignore = ignore.clone();
    let check_js = ps
      .maybe_config_file
      .as_ref()
      .map(|cf| cf.get_check_js())
      .unwrap_or(false);

    async move {
      let test_modules = if test_flags.doc {
        collect_specifiers(include.clone(), &ignore, is_supported_test_ext)
      } else {
        collect_specifiers(include.clone(), &ignore, is_supported_test_path)
      }?;

      let mut paths_to_watch = paths_to_watch_clone;
      let mut modules_to_reload = if files_changed {
        Vec::new()
      } else {
        test_modules
          .iter()
          .map(|url| (url.clone(), ModuleKind::Esm))
          .collect()
      };
      let maybe_imports = if let Some(result) = maybe_imports {
        result?
      } else {
        None
      };
      let maybe_resolver = if maybe_jsx_resolver.is_some() {
        maybe_jsx_resolver.as_ref().map(|jr| jr.as_resolver())
      } else {
        maybe_import_map_resolver
          .as_ref()
          .map(|im| im.as_resolver())
      };
      let graph = deno_graph::create_graph(
        test_modules
          .iter()
          .map(|s| (s.clone(), ModuleKind::Esm))
          .collect(),
        false,
        maybe_imports,
        cache.as_mut_loader(),
        maybe_resolver,
        maybe_locker,
        None,
        None,
      )
      .await;
      graph_valid(&graph, !no_check, check_js)?;

      // TODO(@kitsonk) - This should be totally derivable from the graph.
      for specifier in test_modules {
        fn get_dependencies<'a>(
          graph: &'a deno_graph::ModuleGraph,
          maybe_module: Option<&'a deno_graph::Module>,
          // This needs to be accessible to skip getting dependencies if they're already there,
          // otherwise this will cause a stack overflow with circular dependencies
          output: &mut HashSet<&'a ModuleSpecifier>,
          no_check: bool,
        ) {
          if let Some(module) = maybe_module {
            for dep in module.dependencies.values() {
              if let Some(specifier) = &dep.get_code() {
                if !output.contains(specifier) {
                  output.insert(specifier);
                  get_dependencies(
                    graph,
                    graph.get(specifier),
                    output,
                    no_check,
                  );
                }
              }
              if !no_check {
                if let Some(specifier) = &dep.get_type() {
                  if !output.contains(specifier) {
                    output.insert(specifier);
                    get_dependencies(
                      graph,
                      graph.get(specifier),
                      output,
                      no_check,
                    );
                  }
                }
              }
            }
          }
        }

        // This test module and all it's dependencies
        let mut modules = HashSet::new();
        modules.insert(&specifier);
        get_dependencies(&graph, graph.get(&specifier), &mut modules, no_check);

        paths_to_watch.extend(
          modules
            .iter()
            .filter_map(|specifier| specifier.to_file_path().ok()),
        );

        if let Some(changed) = &changed {
          for path in changed.iter().filter_map(|path| {
            deno_core::resolve_url_or_path(&path.to_string_lossy()).ok()
          }) {
            if modules.contains(&&path) {
              modules_to_reload.push((specifier, ModuleKind::Esm));
              break;
            }
          }
        }
      }

      Ok((paths_to_watch, modules_to_reload))
    }
    .map(move |result| {
      if files_changed
        && matches!(result, Ok((_, ref modules)) if modules.is_empty())
      {
        ResolutionResult::Ignore
      } else {
        match result {
          Ok((paths_to_watch, modules_to_reload)) => {
            ResolutionResult::Restart {
              paths_to_watch,
              result: Ok(modules_to_reload),
            }
          }
          Err(e) => ResolutionResult::Restart {
            paths_to_watch,
            result: Err(e),
          },
        }
      }
    })
  };

  let operation = |modules_to_reload: Vec<(ModuleSpecifier, ModuleKind)>| {
    let filter = test_flags.filter.clone();
    let include = include.clone();
    let ignore = ignore.clone();
    let lib = lib.clone();
    let permissions = permissions.clone();
    let ps = ps.clone();

    async move {
      let specifiers_with_mode = fetch_specifiers_with_test_mode(
        ps.clone(),
        include.clone(),
        ignore.clone(),
        test_flags.doc,
      )
      .await?
      .iter()
      .filter(|(specifier, _)| {
        contains_specifier(&modules_to_reload, specifier)
      })
      .cloned()
      .collect::<Vec<(ModuleSpecifier, TestMode)>>();

      check_specifiers(
        ps.clone(),
        permissions.clone(),
        specifiers_with_mode.clone(),
        lib,
      )
      .await?;

      if test_flags.no_run {
        return Ok(());
      }

      test_specifiers(
        ps.clone(),
        permissions.clone(),
        specifiers_with_mode,
        TestSpecifierOptions {
          compat_mode: flags.compat,
          concurrent_jobs: test_flags.concurrent_jobs,
          fail_fast: test_flags.fail_fast,
          filter: filter.clone(),
          shuffle: test_flags.shuffle,
        },
      )
      .await?;

      Ok(())
    }
  };

  file_watcher::watch_func(
    resolver,
    operation,
    file_watcher::PrintConfig {
      job_name: "Test".to_string(),
      clear_screen: !flags.no_clear_screen,
    },
  )
  .await?;

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_human_elapsed() {
    assert_eq!(human_elapsed(1), "(1ms)");
    assert_eq!(human_elapsed(256), "(256ms)");
    assert_eq!(human_elapsed(1000), "(1s)");
    assert_eq!(human_elapsed(1001), "(1s)");
    assert_eq!(human_elapsed(1020), "(1s)");
    assert_eq!(human_elapsed(70 * 1000), "(1m10s)");
    assert_eq!(human_elapsed(86 * 1000 + 100), "(1m26s)");
  }
}
