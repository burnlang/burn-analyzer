use log::error;
use std::path::{Path, PathBuf};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{Position, Range};
use url::Url;

pub fn get_path_from_uri(uri: &Url) -> String {
    match uri.to_file_path() {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => uri.path().to_string(),
    }
}

pub fn get_burn_version() -> String {
    // the ./burn is temporary for developement should be replaced with burn soon
    match std::process::Command::new("./burn")
        .arg("--version")
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8_lossy(&output.stdout).trim().to_string()
            } else {
                error!(
                    "Failed to get burn version: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                "unknown".to_string()
            }
        }
        Err(e) => {
            error!("Failed to execute burn command: {}", e);
            "unknown".to_string()
        }
    }
}

pub fn position_to_offset(text: &str, position: Position) -> Result<usize> {
    let lines: Vec<&str> = text.lines().collect();

    if position.line as usize >= lines.len() {
        error!("Invalid line position: {}", position.line);
        return Err(tower_lsp::jsonrpc::Error {
            code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
            message: "Invalid position".into(),
            data: None,
        });
    }

    let mut offset = 0;
    for i in 0..position.line as usize {
        offset += lines[i].len() + 1;
    }

    let line = lines[position.line as usize];
    let column = position.character as usize;
    let column = column.min(line.len());

    Ok(offset + column)
}

pub fn offset_to_position(text: &str, offset: usize) -> Result<Position> {
    if offset > text.len() {
        error!("Offset {} exceeds document length {}", offset, text.len());
        return Err(tower_lsp::jsonrpc::Error {
            code: tower_lsp::jsonrpc::ErrorCode::InvalidParams,
            message: "Invalid offset".into(),
            data: None,
        });
    }

    let mut line = 0;
    let mut char_count = 0;

    for (i, c) in text.char_indices() {
        if i >= offset {
            break;
        }

        if c == '\n' {
            line += 1;
            char_count = 0;
        } else {
            char_count += 1;
        }
    }

    Ok(Position::new(line as u32, char_count as u32))
}

pub fn create_range(start: Position, end: Position) -> Range {
    Range { start, end }
}

pub fn get_burn_files<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                result.extend(get_burn_files(path));
            } else if let Some(extension) = path.extension() {
                if extension == "bn" {
                    result.push(path);
                }
            }
        }
    }

    result
}

pub fn find_word_at_offset(text: &str, offset: usize) -> Option<(usize, usize)> {
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
