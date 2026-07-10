mod convert;
mod vfs_ext;

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use dashmap::DashMap;
use itertools::Itertools as _;
use lopa_core::{analysis::Analysis, ide::Root, vfs::Vfs};
use notify_rust::{Notification, NotificationId};
use salsa::Setter;
use tokio::task::{self, AbortHandle};
use tower_lsp_server::{Client, LanguageServer, jsonrpc::Result, ls_types::*};

use crate::vfs_ext::VfsExt as _;

pub struct Backend {
    pub client: Client,
    pub opened_files: DashMap<Uri, FileData>,
    pub vfs: Arc<RwLock<Vfs>>,
    pub analysis: Arc<Mutex<Analysis>>,
}

#[derive(Default)]
pub struct FileData {
    diagnostics_task: Option<AbortHandle>,
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

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "lopa-ls".to_string(),
                version: Some("0.0.1".to_string()),
            }),
            offset_encoding: None,
            capabilities: ServerCapabilities {
                document_formatting_provider: Some(OneOf::Left(true)),
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
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                position_encoding: Some(PositionEncodingKind::UTF8),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {}
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = &params.text_document.uri;
        {
            let mut vfs = self.vfs.write().unwrap();
            let mut analysis = self.analysis.lock().unwrap();

            let Some(file) = vfs.file_by_uri(uri) else {
                return;
            };

            for change in params.content_changes.iter() {
                let range = change.range.map(|r| convert::from_range(&vfs, file, r));
                vfs.change_content(&mut analysis.db, file, &change.text, range);
            }
        }

        // let uris = self
        //     .opened_files
        //     .iter()
        //     .map(|o| o.key().clone())
        //     .collect_vec();
        self.spawn_update_diagnostics(uri.clone());
    }
    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let mut vfs = self.vfs.write().unwrap();
        let uri = params.text_document.uri;
        let path = uri.to_file_path().unwrap();

        self.opened_files.insert(uri.clone(), FileData::default());
        if let Some(root_dir) = Self::find_root(&path) {
            if vfs.root().is_none() {
                let root = Root::new(
                    &self.analysis.lock().unwrap().db,
                    Default::default(),
                    root_dir.clone(),
                );
                vfs.set_root(root);
            }
            vfs.insert_file(
                path.to_path_buf(),
                params.text_document.text,
                &mut self.analysis.lock().unwrap().db,
            );
            self.scan_files(root_dir.as_path(), &mut vfs);
        }
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {}
}

impl Backend {
    fn scan_files(&self, root: &Path, vfs: &mut Vfs) {
        let mut analysis = self.analysis.lock().unwrap();
        let db = &mut analysis.db;
        let mut files = vec![];
        for entry in walkdir::WalkDir::new(root.join("src"))
            .into_iter()
            .flatten()
            .filter(|e| e.path().is_file())
            .filter(|e| e.path().extension().map(|e| e == "lopa").unwrap_or(false))
        {
            let path = entry.path().to_path_buf();
            let uri = Uri::from_file_path(&path).unwrap();
            if self.opened_files.contains_key(&uri) {
                let file = vfs.file_by_path(&path).unwrap();
                files.push(file);
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(entry.path()) else {
                continue;
            };
            let file = if let Some(file) = vfs.file_by_path(&path) {
                vfs.change_content(db, file, &contents, None);
                file
            } else {
                if let Some(file) = vfs.insert_file(path, contents, db) {
                    file
                } else {
                    panic!("no source root"); //FIXME: handle this better
                }
            };
            files.push(file);
        }
        vfs.root().unwrap().set_files(db).to(files);
    }

    fn find_root(path: &Path) -> Option<PathBuf> {
        let mut current = path.to_path_buf();
        while let Some(root) = current.parent() {
            if !&root.join("lopa.toml").is_file() {
                _ = current.pop();
                continue;
            }

            if !current.ends_with("src") {
                _ = current.pop();
                continue;
            }

            return Some(root.to_path_buf());
        }
        None
    }
    fn spawn_update_diagnostics(&self, uri: Uri) {
        let (analysis, vfs) = (self.analysis.clone(), self.vfs.clone());
        let uri_clone = uri.clone();
        let task = task::spawn_blocking(move || {
            salsa::Cancelled::catch(|| {
                let analysis = analysis.lock().unwrap();
                let vfs = vfs.read().unwrap();
                let (file, content) = {
                    let file = vfs.file_by_uri(&uri_clone).expect("TODO: handle");
                    (file, vfs.content_by_file(file))
                };
                let diagnostics = analysis.diagnostics(file);
                convert::to_lsp_diagnostics(content, diagnostics)
            })
            .unwrap_or_default()
        });
        let Some(mut file) = self.opened_files.get_mut(&uri) else {
            task.abort();
            return;
        };
        if let Some(prev_task) = file.diagnostics_task.replace(task.abort_handle()) {
            prev_task.abort();
        }
        drop(file);

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
