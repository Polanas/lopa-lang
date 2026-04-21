use std::sync::{Arc, RwLock};

use lopa_core::ide::Analysis;
use lopa_lsp::{
    Backend,
    vfs::{self, Vfs},
};
use tower_lsp_server::{LspService, Server};

#[tokio::main]
async fn main() {
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        vfs: Arc::new(RwLock::new(Vfs::new())),
        opened_files: Default::default(),
        analysis: Arc::new(Analysis::default().into()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
