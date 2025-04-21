# Burn Language Analyzer

A Rust-based language server implementation for the Burn programming language.

## Features

- **Syntax Highlighting**: Provides syntax highlighting for Burn language files
- **Error Reporting**: Shows syntax and type errors as you type
- **Code Completion**: Offers context-aware code suggestions
- **Hover Information**: Displays type information and documentation when hovering over code
- **Go to Definition**: Jump to where variables, functions, and types are defined
- **Document Outline**: Provides a structural outline of your code


## Development

### Prerequisites

- Rust (latest stable)
- [tower-lsp](https://github.com/ebkalderon/tower-lsp) crate

### Building

```bash
cargo build --release
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.