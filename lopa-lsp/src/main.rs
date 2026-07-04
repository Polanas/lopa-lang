use lopa_lsp::Backend;
use notify_rust::Notification;
use tower_lsp_server::{LspService, Server};

#[tokio::main]
async fn main() {
    unsafe {
        std::env::set_var("RUST_BACKTRACE", "1");
    }
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
