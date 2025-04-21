use burn_analyzer::server::BurnLanguageServer;
use std::env;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| BurnLanguageServer::new(client));

    log::info!("Starting Burn language server");
    Server::new(stdin, stdout, socket).serve(service).await;
}
