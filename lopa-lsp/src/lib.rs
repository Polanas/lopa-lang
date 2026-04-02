pub mod base;
pub mod convert;
pub mod uri_ext;
pub mod vfs;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use dashmap::DashMap;
use notify_rust::Notification;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

use crate::uri_ext::UrlExt as _;
use crate::vfs::Vfs;

pub struct Settings {}

pub struct Backend {
    pub client: Client,
    pub vfs: Arc<RwLock<Vfs>>,
    pub opened_files: DashMap<Uri, FileData>,
}

pub struct FileData {}

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "lopa-ls".to_string(),
                version: Some("0.0.1".to_string()),
            }),
            offset_encoding: None,
            capabilities: ServerCapabilities {
                // document_formatting_provider: Some(OneOf::Left(true)),
                // inlay_hint_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: None,
                        })),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                // execute_command_provider: Some(ExecuteCommandOptions {
                //     commands: vec!["dummy.do_something".to_string()],
                //     work_done_progress_options: Default::default(),
                // }),
                //
                // workspace: Some(WorkspaceServerCapabilities {
                //     workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                //         supported: Some(true),
                //         change_notifications: Some(OneOf::Left(true)),
                //     }),
                //     file_operations: None,
                // }),
                // semantic_tokens_provider: Some(
                //     SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                //         SemanticTokensRegistrationOptions {
                //             text_document_registration_options: {
                //                 TextDocumentRegistrationOptions {
                //                     document_selector: Some(vec![DocumentFilter {
                //                         language: Some("lopa".to_string()),
                //                         scheme: Some("file".to_string()),
                //                         pattern: None,
                //                     }]),
                //                 }
                //             },
                //             semantic_tokens_options: SemanticTokensOptions {
                //                 work_done_progress_options: WorkDoneProgressOptions::default(),
                //                 legend: SemanticTokensLegend {
                //                     token_types: vec![
                //                         SemanticTokenType::FUNCTION,
                //                         SemanticTokenType::VARIABLE,
                //                         SemanticTokenType::PARAMETER,
                //                         SemanticTokenType::STRUCT,
                //                         SemanticTokenType::PROPERTY,
                //                     ],
                //                     token_modifiers: vec![],
                //                 },
                //                 range: Some(true),
                //                 full: Some(SemanticTokensFullOptions::Bool(true)),
                //             },
                //             static_registration_options: StaticRegistrationOptions::default(),
                //         },
                //     ),
                // ),
                // definition_provider: Some(OneOf::Left(true)),
                // references_provider: Some(OneOf::Left(true)),
                // rename_provider: Some(OneOf::Left(true)),
                position_encoding: Some(PositionEncodingKind::UTF8),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_save(&self, _params: DidSaveTextDocumentParams) {}

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let mut vfs = self.vfs.write().unwrap();
        let uri = params.text_document.uri;
        vfs.set_path_content(uri.to_vfs_path().unwrap(), params.text_document.text);
        self.opened_files.insert(uri, FileData {});
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let (diagnostics, uri) = {
            let mut vfs = self.vfs.write().unwrap();
            let uri = params.text_document.uri;
            let Some(file) = vfs.file_by_url(&uri) else {
                return;
            };
            for change in params.content_changes.iter() {
                let range = change.range.map(|r| convert::from_range(&vfs, file, r));
                vfs.change_file_content(file, &change.text, range);
            }

            let content = vfs.content_by_file(file);
            let len = {
                let content = content.read().unwrap();
                content.len()
            };
            let range = convert::to_range(&vfs, file, 0..(len-1));
            let diagnostics = vec![Diagnostic {
                range,
                message: content.read().unwrap().as_str().to_string().replace("\n", "\\n"),
                ..Default::default()
            }];
            (diagnostics, uri)
        };

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {}

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        for file_event in &params.changes {
            if self.opened_files.contains_key(&file_event.uri) {
                continue;
            }
            let Some(path) = file_event.uri.to_file_path() else {
                continue;
            };

            if matches!(
                file_event.typ,
                FileChangeType::CREATED | FileChangeType::CHANGED
            ) {
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        self.vfs
                            .write()
                            .unwrap()
                            .set_path_content(file_event.uri.to_vfs_path().unwrap(), content);
                    }
                    Err(e) => {
                        panic!("{e}");
                        //TODO: add proper logging
                    }
                }
            }
            if file_event.typ == FileChangeType::DELETED {
                self.vfs.write().unwrap().remove_uri(&file_event.uri);
            }
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(String::from("hover1"))),
            range: None,
        }))
    }
}
