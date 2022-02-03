// Copyright 2018-2022 the Deno authors. All rights reserved. MIT license.

use std::collections::HashMap;
use std::future::Future;

use deno_ast::swc::common::BytePos;
use deno_ast::swc::common::Span;
use deno_ast::LineAndColumnIndex;
use deno_ast::ModuleSpecifier;
use deno_ast::SourceTextInfo;
use deno_core::anyhow::anyhow;
use deno_core::error::AnyError;
use deno_core::serde_json;
use lspower::lsp::ClientCapabilities;
use lspower::lsp::ClientInfo;
use lspower::lsp::CompletionContext;
use lspower::lsp::CompletionParams;
use lspower::lsp::CompletionResponse;
use lspower::lsp::CompletionTextEdit;
use lspower::lsp::CompletionTriggerKind;
use lspower::lsp::DidChangeTextDocumentParams;
use lspower::lsp::DidCloseTextDocumentParams;
use lspower::lsp::DidOpenTextDocumentParams;
use lspower::lsp::InitializeParams;
use lspower::lsp::InitializedParams;
use lspower::lsp::PartialResultParams;
use lspower::lsp::Position;
use lspower::lsp::Range;
use lspower::lsp::TextDocumentContentChangeEvent;
use lspower::lsp::TextDocumentIdentifier;
use lspower::lsp::TextDocumentItem;
use lspower::lsp::TextDocumentPositionParams;
use lspower::lsp::VersionedTextDocumentIdentifier;
use lspower::lsp::WorkDoneProgressParams;
use lspower::LanguageServer;

use crate::logger;

use super::client::Client;
use super::config::CompletionSettings;
use super::config::ImportCompletionSettings;
use super::config::WorkspaceSettings;

#[derive(Debug)]
pub struct ReplCompletionItem {
  pub new_text: String,
  pub span: Span,
}

pub struct ReplLanguageServer {
  language_server: super::language_server::LanguageServer,
  document_version: i32,
  document_text: String,
  pending_text: String,
  cwd_uri: ModuleSpecifier,
}

impl ReplLanguageServer {
  pub async fn new_initialized() -> Result<ReplLanguageServer, AnyError> {
    super::logging::set_lsp_log_level(log::Level::Debug);
    let language_server =
      super::language_server::LanguageServer::new(Client::new_for_repl());

    let cwd_uri = get_cwd_uri()?;

    #[allow(deprecated)]
    language_server
      .initialize(InitializeParams {
        process_id: None,
        root_path: None,
        root_uri: Some(cwd_uri.clone()),
        initialization_options: Some(
          serde_json::to_value(get_repl_workspace_settings()).unwrap(),
        ),
        capabilities: ClientCapabilities {
          workspace: None,
          text_document: None,
          window: None,
          general: None,
          experimental: None,
        },
        trace: None,
        workspace_folders: None,
        client_info: Some(ClientInfo {
          name: "Deno REPL".to_string(),
          version: None,
        }),
        locale: None,
      })
      .await?;

    language_server.initialized(InitializedParams {}).await;

    let server = ReplLanguageServer {
      language_server,
      document_version: 0,
      document_text: String::new(),
      pending_text: String::new(),
      cwd_uri,
    };
    server.open_current_document().await;

    Ok(server)
  }

  pub async fn commit_text(&mut self, line_text: &str) {
    self.did_change(line_text).await;
    self.document_text.push_str(&self.pending_text);
    self.pending_text = String::new();
  }

  pub async fn completions(
    &mut self,
    line_text: &str,
    position: usize,
  ) -> Vec<ReplCompletionItem> {
    self.did_change(line_text).await;
    let before_line_len = BytePos(self.document_text.len() as u32);
    let position = before_line_len + BytePos(position as u32);
    let text_info = deno_ast::SourceTextInfo::from_string(format!(
      "{}{}",
      self.document_text, self.pending_text
    ));
    let line_and_column = text_info.line_and_column_index(position);
    let response = self
      .language_server
      .completion(CompletionParams {
        text_document_position: TextDocumentPositionParams {
          text_document: TextDocumentIdentifier {
            uri: self.get_document_specifier(),
          },
          position: Position {
            line: line_and_column.line_index as u32,
            character: line_and_column.column_index as u32,
          },
        },
        work_done_progress_params: WorkDoneProgressParams {
          work_done_token: None,
        },
        partial_result_params: PartialResultParams {
          partial_result_token: None,
        },
        context: Some(CompletionContext {
          trigger_kind: CompletionTriggerKind::INVOKED,
          trigger_character: None,
        }),
      })
      .await
      .ok()
      .unwrap_or_default();

    let items = match response {
      Some(CompletionResponse::Array(items)) => items,
      Some(CompletionResponse::List(list)) => list.items,
      None => Vec::new(),
    };
    items
      .into_iter()
      .filter_map(|item| {
        item.text_edit.and_then(|edit| match edit {
          CompletionTextEdit::Edit(edit) => Some(ReplCompletionItem {
            new_text: edit.new_text,
            span: lsp_range_to_span(&text_info, &edit.range),
          }),
          CompletionTextEdit::InsertAndReplace(_) => None,
        })
      })
      .filter(|item| {
        // filter the results to only exact matches
        let text = &text_info.text_str()
          [item.span.lo.0 as usize..item.span.hi.0 as usize];
        item.new_text.starts_with(text)
      })
      .map(|mut item| {
        // convert back to a line position
        item.span = Span::new(
          item.span.lo - before_line_len,
          item.span.hi - before_line_len,
          Default::default(),
        );
        item
      })
      .collect()
  }

  async fn did_change(&mut self, new_text: &str) {
    self.check_cwd_change().await;
    let new_text = if new_text.ends_with('\n') {
      new_text.to_string()
    } else {
      format!("{}\n", new_text)
    };
    self.document_version += 1;
    let current_line_count =
      self.document_text.chars().filter(|c| *c == '\n').count() as u32;
    let pending_line_count =
      self.pending_text.chars().filter(|c| *c == '\n').count() as u32;
    self
      .language_server
      .did_change(DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
          uri: self.get_document_specifier(),
          version: self.document_version,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
          range: Some(Range {
            start: Position::new(current_line_count, 0),
            end: Position::new(current_line_count + pending_line_count, 0),
          }),
          range_length: None,
          text: new_text.to_string(),
        }],
      })
      .await;
    self.pending_text = new_text;
  }

  async fn check_cwd_change(&mut self) {
    // handle if the cwd changes, if the cwd is deleted in the case of
    // get_cwd_uri() erroring, then keep using it as the base
    let cwd_uri = get_cwd_uri().unwrap_or_else(|_| self.cwd_uri.clone());
    if self.cwd_uri != cwd_uri {
      self
        .language_server
        .did_close(DidCloseTextDocumentParams {
          text_document: TextDocumentIdentifier {
            uri: self.get_document_specifier(),
          },
        })
        .await;
      self.cwd_uri = cwd_uri;
      self.document_version = 0;
      self.open_current_document().await;
    }
  }

  async fn open_current_document(&self) {
    self
      .language_server
      .did_open(DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
          uri: self.get_document_specifier(),
          language_id: "typescript".to_string(),
          version: self.document_version,
          text: format!("{}{}", self.document_text, self.pending_text),
        },
      })
      .await;
  }

  fn get_document_specifier(&self) -> ModuleSpecifier {
    self.cwd_uri.join("$deno$repl.ts").unwrap()
  }
}

fn lsp_range_to_span(text_info: &SourceTextInfo, range: &Range) -> Span {
  Span::new(
    text_info.byte_index(LineAndColumnIndex {
      line_index: range.start.line as usize,
      column_index: range.start.character as usize,
    }),
    text_info.byte_index(LineAndColumnIndex {
      line_index: range.end.line as usize,
      column_index: range.end.character as usize,
    }),
    Default::default(),
  )
}

fn get_cwd_uri() -> Result<ModuleSpecifier, AnyError> {
  let cwd = std::env::current_dir()?;
  ModuleSpecifier::from_directory_path(&cwd)
    .map_err(|_| anyhow!("Could not get URI from {}", cwd.display()))
}

pub fn get_repl_workspace_settings() -> WorkspaceSettings {
  WorkspaceSettings {
    enable: true,
    config: None,
    certificate_stores: None,
    cache: None,
    import_map: None,
    code_lens: Default::default(),
    internal_debug: false,
    lint: false,
    tls_certificate: None,
    unsafely_ignore_certificate_errors: None,
    unstable: false,
    suggest: CompletionSettings {
      complete_function_calls: false,
      names: false,
      paths: false,
      auto_imports: false,
      imports: ImportCompletionSettings {
        auto_discover: false,
        hosts: HashMap::from([("https://deno.land".to_string(), true)]),
      },
    },
  }
}
