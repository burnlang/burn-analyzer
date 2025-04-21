use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::analyzer::{BurnAnalyzer, DocumentSymbol as BurnDocumentSymbol, SymbolType};
use crate::typechecker;
use crate::utils;
use log::{error, info};
use std::sync::Arc;

pub struct BurnLanguageServer {
    client: Client,
    document_map: DashMap<String, String>,
    type_checker: Arc<typechecker::BurnTypeChecker>,
    analyzer: Arc<BurnAnalyzer>,
}

impl BurnLanguageServer {
    pub fn new(client: Client) -> Self {
        let type_checker = Arc::new(typechecker::BurnTypeChecker::new());
        let analyzer = Arc::new(BurnAnalyzer::new(Arc::clone(&type_checker)));

        BurnLanguageServer {
            client,
            document_map: DashMap::new(),
            type_checker,
            analyzer,
        }
    }

    async fn validate_document(&self, uri: &Url) -> Result<()> {
        let uri_str = uri.to_string();

        let diagnostics = match self.document_map.get(&uri_str) {
            Some(document) => {
                // Use analyzer to get diagnostics
                let errors = self.analyzer.analyze_document(&uri_str);

                // Convert analyzer errors to LSP diagnostics
                errors
                    .iter()
                    .map(|err| Diagnostic {
                        range: Range {
                            start: Position {
                                line: err.line as u32,
                                character: err.column as u32,
                            },
                            end: Position {
                                line: err.line as u32,
                                character: (err.column + err.length) as u32,
                            },
                        },
                        severity: Some(match err.error_type {
                            crate::analyzer::ErrorType::ParseError => DiagnosticSeverity::ERROR,
                            crate::analyzer::ErrorType::TypeError => DiagnosticSeverity::ERROR,
                            crate::analyzer::ErrorType::SemanticError => {
                                DiagnosticSeverity::WARNING
                            }
                        }),
                        message: err.message.clone(),
                        source: Some("burn-analyzer".to_string()),
                        ..Diagnostic::default()
                    })
                    .collect()
            }
            None => vec![],
        };

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;

        Ok(())
    }

    fn convert_symbol_type(&self, symbol_type: SymbolType) -> SymbolKind {
        match symbol_type {
            SymbolType::Function => SymbolKind::FUNCTION,
            SymbolType::Variable => SymbolKind::VARIABLE,
            SymbolType::Struct => SymbolKind::STRUCT,
            SymbolType::Class => SymbolKind::CLASS,
            SymbolType::Method => SymbolKind::METHOD,
            SymbolType::Property => SymbolKind::PROPERTY,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BurnLanguageServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        info!("Burn language server initialized");

        // Set workspace folder if available
        if let Some(folders) = params.workspace_folders {
            if let Some(folder) = folders.first() {
                let path = utils::get_path_from_uri(&folder.uri);
                self.analyzer.set_workspace_root(path);
            }
        }

        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(true),
                trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                ..CompletionOptions::default()
            }),
            definition_provider: Some(OneOf::Left(true)),
            document_symbol_provider: Some(OneOf::Left(true)),
            code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
            document_formatting_provider: Some(OneOf::Left(true)),
            ..ServerCapabilities::default()
        };

        Ok(InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: "Burn Language Server".to_string(),
                version: Some(utils::get_burn_version()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Burn language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Burn language server shutting down");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let text = params.text_document.text;

        self.document_map.insert(uri.clone(), text.clone());
        self.analyzer.open_document(&uri, text);

        if let Err(e) = self.validate_document(&params.text_document.uri).await {
            error!("Error validating document: {:?}", e);
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.last() {
            let uri = params.text_document.uri.to_string();
            self.document_map.insert(uri.clone(), change.text.clone());
            self.analyzer.open_document(&uri, change.text.clone());

            if let Err(e) = self.validate_document(&params.text_document.uri).await {
                error!("Error validating document: {:?}", e);
            }
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.document_map.remove(&uri);
        self.analyzer.close_document(&uri);

        // Clear diagnostics when a file is closed
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let position = params.text_document_position_params.position;

        if let Some(document) = self.analyzer.get_document(&uri) {
            return crate::hover::on_hover(&document.content, position, &self.type_checker);
        }

        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let position = params.text_document_position.position;

        if let Some(document) = self.analyzer.get_document(&uri) {
            // Call completion resolver here
            return Ok(Some(CompletionResponse::Array(
                crate::typechecker::get_completions(
                    &document.content,
                    position,
                    &self.type_checker,
                ),
            )));
        }

        Ok(None)
    }

    async fn completion_resolve(&self, item: CompletionItem) -> Result<CompletionItem> {
        // Add more details to completion items if needed
        Ok(item)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .to_string();
        let position = params.text_document_position_params.position;

        if let Some(definition) =
            self.analyzer
                .find_definition(&uri, position.line as usize, position.character as usize)
        {
            let location = Location {
                uri: Url::parse(&definition.uri).unwrap_or_else(|_| {
                    params
                        .text_document_position_params
                        .text_document
                        .uri
                        .clone()
                }),
                range: Range {
                    start: Position {
                        line: definition.line as u32,
                        character: definition.character as u32,
                    },
                    end: Position {
                        line: definition.line as u32,
                        character: definition.character as u32 + 1,
                    },
                },
            };

            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri.to_string();
        let burn_symbols = self.analyzer.get_document_symbols(&uri);

        if burn_symbols.is_empty() {
            return Ok(None);
        }

        // Convert BurnDocumentSymbol to LSP SymbolInformation
        let mut symbols = Vec::new();

        for symbol in burn_symbols {
            let location = Location {
                uri: params.text_document.uri.clone(),
                range: Range {
                    start: Position {
                        line: symbol.line as u32,
                        character: symbol.character as u32,
                    },
                    end: Position {
                        line: symbol.line as u32,
                        character: symbol.character as u32 + symbol.name.len() as u32,
                    },
                },
            };

            symbols.push(SymbolInformation {
                name: symbol.name,
                kind: self.convert_symbol_type(symbol.symbol_type),
                tags: None,
                deprecated: Some(false),
                location,
                container_name: None,
            });
        }

        Ok(Some(DocumentSymbolResponse::Flat(symbols)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        // Placeholder for formatting implementation
        // In the future, this would integrate with a Burn formatter

        Ok(None)
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        // Placeholder for code action implementation

        Ok(None)
    }
}
