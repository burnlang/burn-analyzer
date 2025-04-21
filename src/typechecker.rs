use log::{debug, error};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Position};

use crate::ast::{Ast, Expression, Type};
use crate::utils;

pub struct TypeErrorInfo {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

pub struct BurnTypeChecker {
    variables: Mutex<HashMap<String, HashMap<String, String>>>,

    workspace_root: Mutex<Option<PathBuf>>,

    current_file: Mutex<Option<String>>,
}

impl BurnTypeChecker {
    pub fn new() -> Self {
        BurnTypeChecker {
            variables: Mutex::new(HashMap::new()),
            workspace_root: Mutex::new(None),
            current_file: Mutex::new(None),
        }
    }

    pub fn set_workspace_root<P: AsRef<Path>>(&self, path: P) {
        let mut root = self.workspace_root.lock().unwrap();
        *root = Some(path.as_ref().to_path_buf());
    }

    pub fn set_current_file(&self, file_uri: &str) {
        let mut current = self.current_file.lock().unwrap();
        *current = Some(file_uri.to_string());
    }

    pub fn check_types(&self, ast: &Ast, file_path: &str) -> Result<(), Vec<TypeErrorInfo>> {
        self.set_current_file(file_path);

        let mut variable_types = HashMap::new();
        let mut errors = Vec::new();

        for node in &ast.nodes {
            match node {
                crate::ast::Node::VariableDeclaration {
                    name,
                    data_type,
                    line,
                    column,
                    ..
                } => {
                    let type_str = match data_type {
                        Some(t) => t.to_string(),
                        None => "any".to_string(),
                    };
                    variable_types.insert(name.clone(), type_str);
                }
                crate::ast::Node::FunctionDeclaration {
                    name,
                    params,
                    return_type,
                    line,
                    column,
                    ..
                } => {
                    let param_types: Vec<String> = params
                        .iter()
                        .map(|p| {
                            p.typ
                                .clone()
                                .map_or_else(|| "any".to_string(), |t| t.to_string())
                        })
                        .collect();

                    let return_type_str = match return_type {
                        Some(t) => t.to_string(),
                        None => "void".to_string(),
                    };

                    let fn_type = format!("fn({})->{}", param_types.join(", "), return_type_str);
                    variable_types.insert(name.clone(), fn_type);
                }
                crate::ast::Node::StructDeclaration {
                    name,
                    fields,
                    line,
                    column,
                    ..
                } => {
                    variable_types.insert(name.clone(), format!("struct {}", name));

                    for field in fields {
                        let field_type = match &field.typ {
                            Some(t) => t.to_string(),
                            None => "any".to_string(),
                        };
                    }
                }

                _ => {}
            }
        }

        for node in &ast.nodes {}

        if errors.is_empty() {
            let mut all_variables = self.variables.lock().unwrap();
            all_variables.insert(file_path.to_string(), variable_types);
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn get_variable_type(&self, variable_name: &str) -> Option<String> {
        let current_file = self.current_file.lock().unwrap();

        if let Some(file) = &*current_file {
            let variables = self.variables.lock().unwrap();

            if let Some(file_vars) = variables.get(file) {
                return file_vars.get(variable_name).cloned();
            }
        }

        match variable_name {
            "String" => Some("type".to_string()),
            "Number" => Some("type".to_string()),
            "Boolean" => Some("type".to_string()),
            "Array" => Some("type".to_string()),
            "Object" => Some("type".to_string()),
            "Date" => Some("class".to_string()),
            "Http" => Some("namespace".to_string()),
            "Time" => Some("namespace".to_string()),
            _ => None,
        }
    }

    pub fn get_property_type(&self, object_type: &str, property_name: &str) -> Option<String> {
        match object_type {
            "String" => match property_name {
                "length" => Some("number".to_string()),
                "toUpperCase" => Some("fn()->String".to_string()),
                "toLowerCase" => Some("fn()->String".to_string()),
                "substring" => Some("fn(number, number)->String".to_string()),
                _ => None,
            },
            "Array" => match property_name {
                "length" => Some("number".to_string()),
                "push" => Some("fn(any)->number".to_string()),
                "pop" => Some("fn()->any".to_string()),
                "join" => Some("fn(String)->String".to_string()),
                _ => None,
            },
            "Date" => match property_name {
                "getTime" => Some("fn()->number".to_string()),
                "getDay" => Some("fn()->number".to_string()),
                "getMonth" => Some("fn()->number".to_string()),
                "getFullYear" => Some("fn()->number".to_string()),
                _ => None,
            },
            "Http" => match property_name {
                "get" => Some("fn(String)->HttpResponse".to_string()),
                "post" => Some("fn(String, Object)->HttpResponse".to_string()),
                _ => None,
            },
            "Time" => match property_name {
                "now" => Some("fn()->number".to_string()),
                "sleep" => Some("fn(number)->void".to_string()),
                _ => None,
            },

            s if s.starts_with("struct ") => {
                let struct_name = s.trim_start_matches("struct ");
                let current_file = self.current_file.lock().unwrap();

                if let Some(file) = &*current_file {
                    Some("any".to_string())
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

pub fn get_completions(
    document: &str,
    position: Position,
    type_checker: &std::sync::Arc<BurnTypeChecker>,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    if let Ok(offset) = utils::position_to_offset(document, position) {
        let text_before = &document[..offset];

        if let Some(last_char) = text_before.chars().last() {
            if last_char == '.' {
                if let Some(property_start) = text_before.rfind('.') {
                    let object_end = property_start;

                    if let Some(object_start) = text_before[..object_end]
                        .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .map(|pos| pos + 1)
                        .or(Some(0))
                    {
                        let object_name = text_before[object_start..object_end].trim();

                        if let Some(object_type) = type_checker.get_variable_type(object_name) {
                            match object_type.as_str() {
                                "String" => add_string_completions(&mut items),
                                "Array" => add_array_completions(&mut items),
                                "Date" => add_date_completions(&mut items),
                                "Http" => add_http_completions(&mut items),
                                "Time" => add_time_completions(&mut items),

                                _ => {}
                            }

                            return items;
                        }
                    }
                }

                return default_property_completions();
            }
        }
    }

    add_keyword_completions(&mut items);
    add_type_completions(&mut items);
    add_builtin_function_completions(&mut items);

    if let Some(current_file) = &*type_checker.current_file.lock().unwrap() {
        if let Ok(variables) = type_checker.variables.try_lock() {
            if let Some(file_vars) = variables.get(current_file) {
                for (var_name, var_type) in file_vars {
                    items.push(CompletionItem {
                        label: var_name.clone(),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: Some(var_type.clone()),
                        ..Default::default()
                    });
                }
            }
        }
    }

    items
}

fn add_keyword_completions(items: &mut Vec<CompletionItem>) {
    let keywords = [
        "fn", "return", "if", "else", "while", "for", "in", "var", "const", "let", "import",
        "struct", "type", "true", "false", "null", "class", "break", "continue", "switch", "case",
        "default",
    ];

    for &keyword in &keywords {
        items.push(CompletionItem {
            label: keyword.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }
}

fn add_type_completions(items: &mut Vec<CompletionItem>) {
    let types = [
        "String", "Number", "Boolean", "Array", "Object", "Date", "Function", "any", "void",
    ];

    for &typ in &types {
        items.push(CompletionItem {
            label: typ.to_string(),
            kind: Some(CompletionItemKind::CLASS),
            ..Default::default()
        });
    }
}

fn add_builtin_function_completions(items: &mut Vec<CompletionItem>) {
    let builtins = [
        ("print", "fn(any)->void"),
        ("println", "fn(any)->void"),
        ("len", "fn(collection)->Number"),
        ("typeof", "fn(any)->String"),
        ("parseInt", "fn(String)->Number"),
        ("parseFloat", "fn(String)->Number"),
    ];

    for &(name, signature) in &builtins {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(signature.to_string()),
            ..Default::default()
        });
    }
}

fn add_string_completions(items: &mut Vec<CompletionItem>) {
    let methods = [
        ("length", "number", CompletionItemKind::PROPERTY),
        ("toUpperCase", "fn()->String", CompletionItemKind::METHOD),
        ("toLowerCase", "fn()->String", CompletionItemKind::METHOD),
        (
            "substring",
            "fn(number, number)->String",
            CompletionItemKind::METHOD,
        ),
        ("indexOf", "fn(String)->number", CompletionItemKind::METHOD),
        ("split", "fn(String)->Array", CompletionItemKind::METHOD),
    ];

    for &(name, detail, kind) in &methods {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }
}

fn add_array_completions(items: &mut Vec<CompletionItem>) {
    let methods = [
        ("length", "number", CompletionItemKind::PROPERTY),
        ("push", "fn(any)->number", CompletionItemKind::METHOD),
        ("pop", "fn()->any", CompletionItemKind::METHOD),
        ("shift", "fn()->any", CompletionItemKind::METHOD),
        ("unshift", "fn(any)->number", CompletionItemKind::METHOD),
        ("join", "fn(String)->String", CompletionItemKind::METHOD),
        ("map", "fn(fn(any)->any)->Array", CompletionItemKind::METHOD),
        (
            "filter",
            "fn(fn(any)->Boolean)->Array",
            CompletionItemKind::METHOD,
        ),
    ];

    for &(name, detail, kind) in &methods {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }
}

fn add_date_completions(items: &mut Vec<CompletionItem>) {
    let methods = [
        ("getTime", "fn()->number", CompletionItemKind::METHOD),
        ("getDay", "fn()->number", CompletionItemKind::METHOD),
        ("getMonth", "fn()->number", CompletionItemKind::METHOD),
        ("getFullYear", "fn()->number", CompletionItemKind::METHOD),
        ("getHours", "fn()->number", CompletionItemKind::METHOD),
        ("getMinutes", "fn()->number", CompletionItemKind::METHOD),
        ("getSeconds", "fn()->number", CompletionItemKind::METHOD),
    ];

    for &(name, detail, kind) in &methods {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }
}

fn add_http_completions(items: &mut Vec<CompletionItem>) {
    let methods = [
        (
            "get",
            "fn(String)->HttpResponse",
            CompletionItemKind::METHOD,
        ),
        (
            "post",
            "fn(String, Object)->HttpResponse",
            CompletionItemKind::METHOD,
        ),
        (
            "put",
            "fn(String, Object)->HttpResponse",
            CompletionItemKind::METHOD,
        ),
        (
            "delete",
            "fn(String)->HttpResponse",
            CompletionItemKind::METHOD,
        ),
    ];

    for &(name, detail, kind) in &methods {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }
}

fn add_time_completions(items: &mut Vec<CompletionItem>) {
    let methods = [
        ("now", "fn()->number", CompletionItemKind::METHOD),
        ("sleep", "fn(number)->void", CompletionItemKind::METHOD),
    ];

    for &(name, detail, kind) in &methods {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }
}

fn default_property_completions() -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let default_props = [
        ("length", "property", CompletionItemKind::PROPERTY),
        ("name", "property", CompletionItemKind::PROPERTY),
        ("toString", "fn()->String", CompletionItemKind::METHOD),
        ("valueOf", "fn()->any", CompletionItemKind::METHOD),
    ];

    for &(name, detail, kind) in &default_props {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(kind),
            detail: Some(detail.to_string()),
            ..Default::default()
        });
    }

    items
}
