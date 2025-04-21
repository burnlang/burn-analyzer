use log::{error, info};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::ast::Ast;
use crate::parser::{self};
use crate::typechecker::BurnTypeChecker;
use crate::utils;

#[derive(Clone)]
pub struct Document {
    pub uri: String,
    pub content: String,
    pub ast: Option<Ast>,
}

pub struct BurnAnalyzer {
    documents: Mutex<HashMap<String, Document>>,

    type_checker: Arc<BurnTypeChecker>,

    workspace_root: Mutex<Option<PathBuf>>,
}

impl BurnAnalyzer {
    pub fn new(type_checker: Arc<BurnTypeChecker>) -> Self {
        BurnAnalyzer {
            documents: Mutex::new(HashMap::new()),
            type_checker,
            workspace_root: Mutex::new(None),
        }
    }

    pub fn set_workspace_root<P: AsRef<Path>>(&self, path: P) {
        let mut root = self.workspace_root.lock().unwrap();
        *root = Some(path.as_ref().to_path_buf());

        self.type_checker.set_workspace_root(path);
    }

    pub fn open_document(&self, uri: &str, content: String) {
        info!("Opening document: {}", uri);

        let ast = match parser::parse(&content) {
            Ok(ast) => Some(ast),
            Err(errors) => {
                for err in &errors {
                    error!("Parse error in {}: {}", uri, err);
                }
                None
            }
        };

        let document = Document {
            uri: uri.to_string(),
            content,
            ast,
        };

        let mut documents = self.documents.lock().unwrap();
        documents.insert(uri.to_string(), document);
    }

    pub fn close_document(&self, uri: &str) {
        info!("Closing document: {}", uri);
        let mut documents = self.documents.lock().unwrap();
        documents.remove(uri);
    }

    pub fn analyze_document(&self, uri: &str) -> Vec<AnalysisError> {
        let documents = self.documents.lock().unwrap();
        let document = match documents.get(uri) {
            Some(doc) => doc,
            None => {
                error!("Document not found for analysis: {}", uri);
                return vec![];
            }
        };

        let mut errors = Vec::new();

        match &document.ast {
            Some(ast) => {
                self.type_checker.set_current_file(uri);

                match self.type_checker.check_types(ast, uri) {
                    Ok(_) => {}
                    Err(type_errors) => {
                        for err in type_errors {
                            errors.push(AnalysisError {
                                message: err.message,
                                error_type: ErrorType::TypeError,
                                line: err.line,
                                column: err.column,
                                length: err.length,
                            });
                        }
                    }
                }
            }
            None => match parser::parse(&document.content) {
                Ok(_) => {}
                Err(parse_errors) => {
                    for err in parse_errors {
                        errors.push(AnalysisError {
                            message: err.message,
                            error_type: ErrorType::ParseError,
                            line: err.line,
                            column: err.column,
                            length: 1,
                        });
                    }
                }
            },
        }

        errors
    }

    pub fn analyze_all_documents(&self) -> HashMap<String, Vec<AnalysisError>> {
        let documents = self.documents.lock().unwrap();
        let mut results = HashMap::new();

        for (uri, _) in documents.iter() {
            let errors = self.analyze_document(uri);
            results.insert(uri.clone(), errors);
        }

        results
    }

    pub fn get_document(&self, uri: &str) -> Option<Document> {
        let documents = self.documents.lock().unwrap();
        documents.get(uri).cloned()
    }

    pub fn is_burn_file(&self, path: &Path) -> bool {
        path.extension().map_or(false, |ext| ext == "bn")
    }

    pub fn get_workspace_root(&self) -> Option<PathBuf> {
        let root = self.workspace_root.lock().unwrap();
        root.clone()
    }

    pub fn get_all_burn_files(&self) -> Vec<PathBuf> {
        if let Some(root) = self.get_workspace_root() {
            utils::get_burn_files(root)
        } else {
            Vec::new()
        }
    }

    pub fn find_definition(
        &self,
        uri: &str,
        line: usize,
        character: usize,
    ) -> Option<DefinitionLocation> {
        let documents = self.documents.lock().unwrap();

        if let Some(document) = documents.get(uri) {
            if let Ok(offset) = utils::position_to_offset(
                &document.content,
                tower_lsp::lsp_types::Position::new(line as u32, character as u32),
            ) {
                if let Some((start, end)) = utils::find_word_at_offset(&document.content, offset) {
                    let word = &document.content[start..end];

                    for (doc_uri, doc) in documents.iter() {
                        if let Some(ast) = &doc.ast {
                            for node in &ast.nodes {
                                match node {
                                    crate::ast::Node::FunctionDeclaration {
                                        name,
                                        line,
                                        column,
                                        ..
                                    } if name == word => {
                                        return Some(DefinitionLocation {
                                            uri: doc_uri.clone(),
                                            line: *line,
                                            character: *column,
                                        });
                                    }
                                    crate::ast::Node::VariableDeclaration {
                                        name,
                                        line,
                                        column,
                                        ..
                                    } if name == word => {
                                        return Some(DefinitionLocation {
                                            uri: doc_uri.clone(),
                                            line: *line,
                                            character: *column,
                                        });
                                    }
                                    crate::ast::Node::StructDeclaration {
                                        name,
                                        line,
                                        column,
                                        ..
                                    } if name == word => {
                                        return Some(DefinitionLocation {
                                            uri: doc_uri.clone(),
                                            line: *line,
                                            character: *column,
                                        });
                                    }
                                    crate::ast::Node::ClassDeclaration {
                                        name,
                                        line,
                                        column,
                                        ..
                                    } if name == word => {
                                        return Some(DefinitionLocation {
                                            uri: doc_uri.clone(),
                                            line: *line,
                                            character: *column,
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    pub fn get_document_symbols(&self, uri: &str) -> Vec<DocumentSymbol> {
        let documents = self.documents.lock().unwrap();
        let mut symbols = Vec::new();

        if let Some(document) = documents.get(uri) {
            if let Some(ast) = &document.ast {
                for node in &ast.nodes {
                    match node {
                        crate::ast::Node::FunctionDeclaration {
                            name, line, column, ..
                        } => {
                            symbols.push(DocumentSymbol {
                                name: name.clone(),
                                symbol_type: SymbolType::Function,
                                line: *line,
                                character: *column,
                            });
                        }
                        crate::ast::Node::VariableDeclaration {
                            name, line, column, ..
                        } => {
                            symbols.push(DocumentSymbol {
                                name: name.clone(),
                                symbol_type: SymbolType::Variable,
                                line: *line,
                                character: *column,
                            });
                        }
                        crate::ast::Node::StructDeclaration {
                            name, line, column, ..
                        } => {
                            symbols.push(DocumentSymbol {
                                name: name.clone(),
                                symbol_type: SymbolType::Struct,
                                line: *line,
                                character: *column,
                            });
                        }
                        crate::ast::Node::ClassDeclaration {
                            name, line, column, ..
                        } => {
                            symbols.push(DocumentSymbol {
                                name: name.clone(),
                                symbol_type: SymbolType::Class,
                                line: *line,
                                character: *column,
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        symbols
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    ParseError,
    TypeError,
    SemanticError,
}

#[derive(Debug, Clone)]
pub struct AnalysisError {
    pub message: String,
    pub error_type: ErrorType,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

#[derive(Debug, Clone)]
pub struct DefinitionLocation {
    pub uri: String,
    pub line: usize,
    pub character: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolType {
    Function,
    Variable,
    Struct,
    Class,
    Method,
    Property,
}

#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    pub name: String,
    pub symbol_type: SymbolType,
    pub line: usize,
    pub character: usize,
}
