use crate::ast::{Ast, Expression, LiteralValue, Node, Parameter, StructField, Type};
use log::error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

pub fn parse(source: &str) -> Result<Ast, Vec<ParseError>> {
    let mut nodes = Vec::new();
    let mut errors = Vec::new();

    let lines: Vec<&str> = source.lines().collect();

    for (line_idx, line) in lines.iter().enumerate() {
        let line_num = line_idx + 1;
        let trimmed = line.trim();

        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if let Some(var_decl) = parse_variable_declaration(trimmed, line_num) {
            nodes.push(var_decl);
        } else if let Some(fn_decl) =
            parse_function_declaration(trimmed, line_num, &lines[line_idx..])
        {
            nodes.push(fn_decl);
        } else if let Some(struct_decl) =
            parse_struct_declaration(trimmed, line_num, &lines[line_idx..])
        {
            nodes.push(struct_decl);
        } else if let Some(import_decl) = parse_import_declaration(trimmed, line_num) {
            nodes.push(import_decl);
        } else {
            if !trimmed.starts_with('}') && !trimmed.starts_with(')') && !trimmed.starts_with(']') {
                match parse_expression(trimmed, line_num, 0) {
                    Ok(expr) => {
                        nodes.push(Node::ExpressionStatement {
                            expression: Box::new(expr),
                            line: line_num,
                            column: 0,
                        });
                    }
                    Err(err) => {
                        if !trimmed
                            .chars()
                            .all(|c| c.is_whitespace() || c == '{' || c == '}')
                        {
                            errors.push(err);
                        }
                    }
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(Ast { nodes })
    } else {
        Err(errors)
    }
}

fn parse_variable_declaration(line: &str, line_num: usize) -> Option<Node> {
    let mut parts = line.split_whitespace();

    let keyword = parts.next()?;
    if keyword != "var" && keyword != "let" && keyword != "const" {
        return None;
    }

    let name = parts.next()?.trim_end_matches(':');

    let mut data_type = None;
    let mut current = parts.next()?;

    if current == ":" {
        let type_name = parts.next()?;
        data_type = Some(Type::Basic(type_name.to_string()));
        current = parts.next()?;
    }

    let initializer = if current == "=" {
        let value_str = parts.collect::<Vec<&str>>().join(" ");
        let value_str = value_str.trim_end_matches(';');

        match parse_expression(value_str, line_num, line.find('=').unwrap_or(0) + 1) {
            Ok(expr) => Some(Box::new(expr)),
            Err(_) => None,
        }
    } else {
        None
    };

    Some(Node::VariableDeclaration {
        name: name.to_string(),
        initializer,
        data_type,
        is_mutable: keyword != "const",
        line: line_num,
        column: 0,
    })
}

fn parse_function_declaration(line: &str, line_num: usize, all_lines: &[&str]) -> Option<Node> {
    if !line.trim().starts_with("fn ") {
        return None;
    }

    let fn_decl_pattern = regex::Regex::new(
        r"fn\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\((.*?)\)(?:\s*:\s*([a-zA-Z_][a-zA-Z0-9_]*))?\s*\{",
    )
    .ok()?;

    if let Some(captures) = fn_decl_pattern.captures(line) {
        let name = captures.get(1)?.as_str().to_string();
        let params_str = captures.get(2)?.as_str();

        let params = parse_parameters(params_str);

        let return_type = captures
            .get(3)
            .map(|rt| Type::Basic(rt.as_str().to_string()));

        Some(Node::FunctionDeclaration {
            name,
            params,
            return_type,
            body: Vec::new(),
            line: line_num,
            column: line.find("fn")? + 1,
        })
    } else {
        None
    }
}

fn parse_parameters(params_str: &str) -> Vec<Parameter> {
    let mut params = Vec::new();

    for param in params_str.split(',') {
        let param = param.trim();
        if param.is_empty() {
            continue;
        }

        let parts: Vec<&str> = param.split(':').collect();
        let name = parts[0].trim().to_string();

        let typ = if parts.len() > 1 {
            let type_name = parts[1].trim();
            Some(Type::Basic(type_name.to_string()))
        } else {
            None
        };

        params.push(Parameter { name, typ });
    }

    params
}

fn parse_struct_declaration(line: &str, line_num: usize, all_lines: &[&str]) -> Option<Node> {
    if !line.trim().starts_with("struct ") {
        return None;
    }

    let struct_decl_pattern = regex::Regex::new(r"struct\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*\{").ok()?;

    if let Some(captures) = struct_decl_pattern.captures(line) {
        let name = captures.get(1)?.as_str().to_string();

        Some(Node::StructDeclaration {
            name,
            fields: Vec::new(),
            line: line_num,
            column: line.find("struct")? + 1,
        })
    } else {
        None
    }
}

fn parse_import_declaration(line: &str, line_num: usize) -> Option<Node> {
    if !line.trim().starts_with("import ") {
        return None;
    }

    let path_pattern = regex::Regex::new(r#"import\s+(?:\{(.*?)\}\s+from\s+)?"(.+?)""#).ok()?;

    if let Some(captures) = path_pattern.captures(line) {
        let path = captures.get(2)?.as_str().to_string();

        let imported_items = if let Some(items_match) = captures.get(1) {
            items_match
                .as_str()
                .split(',')
                .map(|s| s.trim().to_string())
                .collect()
        } else {
            Vec::new()
        };

        Some(Node::ImportDeclaration {
            path,
            imported_items,
            line: line_num,
            column: line.find("import")? + 1,
        })
    } else {
        None
    }
}

fn parse_expression(
    expr_str: &str,
    line_num: usize,
    column_offset: usize,
) -> Result<Expression, ParseError> {
    let trimmed = expr_str.trim();
    if trimmed.is_empty() {
        return Err(ParseError {
            message: "Empty expression".to_string(),
            line: line_num,
            column: column_offset,
        });
    }

    if let Some(value) = parse_literal(trimmed) {
        return Ok(Expression::Literal {
            value,
            line: line_num,
            column: column_offset,
        });
    }

    if let Some(dot_idx) = trimmed.find('.') {
        let object_str = &trimmed[..dot_idx].trim();
        let property = &trimmed[dot_idx + 1..].trim();

        if !property.contains(' ') && !property.contains('.') && !property.contains('(') {
            if let Ok(object) = parse_expression(object_str, line_num, column_offset) {
                return Ok(Expression::PropertyAccess {
                    object: Box::new(object),
                    property: property.to_string(),
                    line: line_num,
                    column: column_offset + dot_idx + 1,
                });
            }
        }
    }

    if let Some(paren_idx) = trimmed.find('(') {
        let callee_str = &trimmed[..paren_idx].trim();

        let args_str = if let Some(end_paren_idx) = find_matching_paren(trimmed, paren_idx) {
            &trimmed[paren_idx + 1..end_paren_idx]
        } else {
            return Err(ParseError {
                message: "Unmatched parenthesis in function call".to_string(),
                line: line_num,
                column: column_offset + paren_idx,
            });
        };

        let callee = parse_expression(callee_str, line_num, column_offset)?;

        let mut arguments = Vec::new();

        for (i, arg_str) in args_str.split(',').enumerate() {
            let arg_offset = column_offset
                + paren_idx
                + 1
                + args_str[..args_str.find(arg_str).unwrap_or(0)].len();
            if let Ok(arg) = parse_expression(arg_str, line_num, arg_offset) {
                arguments.push(arg);
            }
        }

        return Ok(Expression::Call {
            callee: Box::new(callee),
            arguments,
            line: line_num,
            column: column_offset,
        });
    }

    if is_valid_identifier(trimmed) {
        return Ok(Expression::Variable {
            name: trimmed.to_string(),
            line: line_num,
            column: column_offset,
        });
    }

    Err(ParseError {
        message: format!("Failed to parse expression: {}", trimmed),
        line: line_num,
        column: column_offset,
    })
}

fn parse_literal(text: &str) -> Option<LiteralValue> {
    if (text.starts_with('"') && text.ends_with('"'))
        || (text.starts_with('\'') && text.ends_with('\''))
    {
        let content = &text[1..text.len() - 1];
        return Some(LiteralValue::String(content.to_string()));
    }

    if text == "true" {
        return Some(LiteralValue::Boolean(true));
    } else if text == "false" {
        return Some(LiteralValue::Boolean(false));
    }

    if text == "null" {
        return Some(LiteralValue::Null);
    }

    if let Ok(int_val) = text.parse::<i64>() {
        return Some(LiteralValue::Integer(int_val));
    }

    if let Ok(float_val) = text.parse::<f64>() {
        return Some(LiteralValue::Number(float_val));
    }

    None
}

fn find_matching_paren(text: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0;
    let chars: Vec<char> = text.chars().collect();

    for (i, &c) in chars.iter().enumerate().skip(open_idx) {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }

    None
}

fn is_valid_identifier(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    let mut chars = text.chars();

    let first = chars.next().unwrap();
    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    for c in chars {
        if !c.is_alphanumeric() && c != '_' {
            return false;
        }
    }

    true
}
