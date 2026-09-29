# Burn Language Analyzer

Language server launcher for the [Burn](https://github.com/burnlang/burn) programming language.

Burn ships its language server inside the compiler (`burn lsp`), so it always understands exactly the same
language as the compiler. `burn-analyzer` is a tiny, dependency-free binary that starts `burn lsp` over stdio,
so editors that are configured to run `burn-analyzer` keep working.

## Features

Everything comes from `burn lsp`:

- **Error reporting**: syntax and type errors with exact line and column while you type
- **Code completion**: locals, globals, types, built-ins and members after `.`
- **Hover information**: inferred types and signatures
- **Go to definition**: including into imported files
- **Document outline**
- **Formatting**

## Usage

```bash
cargo install --path .
burn-analyzer
```

`burn-analyzer` looks for the `burn` executable in `$BURN_PATH`, next to its own binary, and on `$PATH`.
You can also point your editor at `burn lsp` directly.

### Neovim

```lua
vim.lsp.start({ name = "burn", cmd = { "burn-analyzer" }, root_dir = vim.fn.getcwd() })
```

### Helix

```toml
[language-server.burn]
command = "burn-analyzer"

[[language]]
name = "burn"
scope = "source.burn"
file-types = ["bn"]
language-servers = ["burn"]
```

`syntaxes/burn.tmLanguage.json` and `language-configuration.json` contain the Burn grammar for editors that
use TextMate grammars.

## Building

```bash
cargo build --release
```

## License

This project is licensed under the MIT License - see the LICENSE file for details.
