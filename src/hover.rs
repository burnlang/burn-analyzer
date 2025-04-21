use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{Hover, MarkedString, Position, Range};

use crate::typechecker::BurnTypeChecker;
use crate::utils;

pub fn on_hover(
    document: &str,
    position: Position,
    type_checker: &Arc<BurnTypeChecker>,
) -> Result<Option<Hover>> {
    let offset = utils::position_to_offset(document, position)?;
    let text = document;

    if let Some((object_name, property_name)) = check_for_dot_access(text, offset) {
        return get_property_hover(object_name, property_name, type_checker);
    }

    if let Some(word_range) = get_word_range_at_position(text, offset) {
        let word = &text[word_range.0..word_range.1];

        if let Some(var_type) = type_checker.get_variable_type(word) {
            return Ok(Some(Hover {
                contents: tower_lsp::lsp_types::HoverContents::Markup(
                    tower_lsp::lsp_types::MarkupContent {
                        kind: tower_lsp::lsp_types::MarkupKind::Markdown,
                        value: format!("**{}**: {}", word, var_type),
                    },
                ),
                range: Some(utils::create_range(
                    utils::offset_to_position(text, word_range.0)?,
                    utils::offset_to_position(text, word_range.1)?,
                )),
            }));
        }

        if let Some(keyword_info) = get_keyword_info(word) {
            return Ok(Some(Hover {
                contents: tower_lsp::lsp_types::HoverContents::Markup(
                    tower_lsp::lsp_types::MarkupContent {
                        kind: tower_lsp::lsp_types::MarkupKind::Markdown,
                        value: format!("**{}**: {}", word, keyword_info),
                    },
                ),
                range: Some(utils::create_range(
                    utils::offset_to_position(text, word_range.0)?,
                    utils::offset_to_position(text, word_range.1)?,
                )),
            }));
        }

        if let Some(builtin_info) = get_builtin_info(word) {
            return Ok(Some(Hover {
                contents: tower_lsp::lsp_types::HoverContents::Markup(
                    tower_lsp::lsp_types::MarkupContent {
                        kind: tower_lsp::lsp_types::MarkupKind::Markdown,
                        value: builtin_info,
                    },
                ),
                range: Some(utils::create_range(
                    utils::offset_to_position(text, word_range.0)?,
                    utils::offset_to_position(text, word_range.1)?,
                )),
            }));
        }
    }

    Ok(None)
}

fn check_for_dot_access(text: &str, offset: usize) -> Option<(String, String)> {
    let text_before = &text[..offset];
    let text_after = &text[offset..];

    if let Some(property_start) = text_before.rfind('.') {
        let object_end = property_start;

        if let Some(object_start) = text_before[..object_end]
            .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
            .map(|pos| pos + 1)
            .or(Some(0))
        {
            let object_name = text_before[object_start..object_end].trim().to_string();

            let after_dot = property_start + 1;
            let property_end = after_dot
                + text_after
                    .find(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .unwrap_or_else(|| text_after.len());

            let property_name = text[after_dot..property_end].trim().to_string();

            if !object_name.is_empty() && !property_name.is_empty() {
                return Some((object_name, property_name));
            }
        }
    }

    None
}

fn get_word_range_at_position(text: &str, offset: usize) -> Option<(usize, usize)> {
    if offset >= text.len() {
        return None;
    }

    let text_before = &text[..offset];
    let start = text_before
        .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map(|pos| pos + 1)
        .unwrap_or(0);

    let text_after = &text[offset..];
    let end_offset = text_after
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or_else(|| text_after.len());
    let end = offset + end_offset;

    if end > start {
        Some((start, end))
    } else {
        None
    }
}

fn get_property_hover(
    object_name: String,
    property_name: String,
    type_checker: &Arc<BurnTypeChecker>,
) -> Result<Option<Hover>> {
    if let Some(object_type) = type_checker.get_variable_type(&object_name) {
        if let Some(property_info) = type_checker.get_property_type(&object_type, &property_name) {
            return Ok(Some(Hover {
                contents: tower_lsp::lsp_types::HoverContents::Markup(
                    tower_lsp::lsp_types::MarkupContent {
                        kind: tower_lsp::lsp_types::MarkupKind::Markdown,
                        value: format!("**{}**: {}", property_name, property_info),
                    },
                ),
                range: None,
            }));
        }
    }

    Ok(None)
}

fn get_keyword_info(keyword: &str) -> Option<String> {
    match keyword {
        "fn" => Some("Function declaration keyword".to_string()),
        "return" => Some("Return statement keyword".to_string()),
        "if" => Some("Conditional statement keyword".to_string()),
        "else" => Some("Conditional statement keyword".to_string()),
        "while" => Some("Loop keyword".to_string()),
        "for" => Some("Loop keyword".to_string()),
        "in" => Some("Loop/iterator keyword".to_string()),
        "var" => Some("Variable declaration keyword".to_string()),
        "const" => Some("Constant declaration keyword".to_string()),
        "let" => Some("Block scoped variable declaration keyword".to_string()),
        "import" => Some("Module import keyword".to_string()),
        "struct" => Some("Structure definition keyword".to_string()),
        "type" => Some("Type alias declaration keyword".to_string()),
        "true" | "false" => Some("Boolean literal".to_string()),
        "null" => Some("Null literal".to_string()),
        "class" => Some("Class definition keyword".to_string()),
        _ => None,
    }
}

fn get_builtin_info(function_name: &str) -> Option<String> {
    match function_name {
        "print" => Some(
            "```burn\nfn print(value: any) -> void\n```\n\nPrints a value to the console.".to_string()
        ),
        "println" => Some(
            "```burn\nfn println(value: any) -> void\n```\n\nPrints a value to the console with a newline.".to_string()
        ),
        "len" => Some(
            "```burn\nfn len(collection: any) -> number\n```\n\nReturns the length of an array, string, or collection.".to_string()
        ),
        "typeof" => Some(
            "```burn\nfn typeof(value: any) -> string\n```\n\nReturns the type of a value as a string.".to_string()
        ),
        "parseInt" => Some(
            "```burn\nfn parseInt(str: string) -> number\n```\n\nParses a string into an integer number.".to_string()
        ),
        "parseFloat" => Some(
            "```burn\nfn parseFloat(str: string) -> number\n```\n\nParses a string into a floating-point number.".to_string()
        ),
        _ => None,
    }
}
