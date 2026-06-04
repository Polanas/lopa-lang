pub mod base;
pub mod convert;
pub mod handler;
pub mod uri_ext;
pub mod vfs;

use std::sync::{Arc, Mutex, RwLock};

use dashmap::DashMap;
use lopa_core::ide::{Analysis, File};
use tokio::task;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer};

use crate::uri_ext::UrlExt as _;
use crate::vfs::Vfs;

pub struct Settings {}

pub struct Backend {
    pub analysis: Arc<Mutex<Analysis>>,
    pub client: Client,
    pub vfs: Arc<RwLock<Vfs>>,
    pub opened_files: DashMap<Uri, FileData>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            analysis: Arc::new(Analysis::default().into()),
            vfs: Arc::new(RwLock::new(Vfs::new())),
            opened_files: Default::default(),
        }
    }
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
        let file = vfs.insert_file(
            uri.to_vfs_path().unwrap(),
            params.text_document.text,
            &self.analysis.lock().unwrap().db,
        );
        self.analysis.lock().unwrap().insert_file(file);
        self.opened_files.insert(uri.clone(), FileData {});
        self.spawn_update_diagnostics(uri);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        {
            let mut vfs = self.vfs.write().unwrap();
            let uri = &params.text_document.uri;
            let Some(file) = vfs.file_by_uri(uri) else {
                return;
            };
            for change in params.content_changes.iter() {
                let range = change.range.map(|r| convert::from_range(&vfs, file, r));
                vfs.change_file_content(file, &change.text, range);
            }
            self.analysis.lock().unwrap().apply_change(file);
        }
        self.spawn_update_diagnostics(params.text_document.uri);
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.opened_files.remove(&params.text_document.uri);
    }

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
                        let file = self
                            .vfs
                            .write()
                            .unwrap()
                            .set_path_content(file_event.uri.to_vfs_path().unwrap(), content);
                        self.analysis.lock().unwrap().insert_file(file);
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

impl Backend {
    fn spawn_update_diagnostics(&self, uri: Uri) {
        let (analysis, vfs) = (self.analysis.clone(), self.vfs.clone());
        let uri_clone = uri.clone();
        let task = task::spawn_blocking(move || {
            let state = State { analysis, vfs };
            handler::diagnostics(state, &uri_clone)
        });
        //TODO: termiante previous diagnostics if present (see opened_files)
        task::spawn({
            let client = self.client.clone();
            async move {
                if let Ok(diagnostics) = task.await {
                    client.publish_diagnostics(uri, diagnostics, None).await;
                } else {
                    //task cancelled, do nothing
                }
            }
        });
    }
}

pub struct State {
    pub analysis: Arc<Mutex<Analysis>>,
    pub vfs: Arc<RwLock<Vfs>>,
}
