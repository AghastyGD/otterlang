# OtterLang TextMate Grammar

This directory contains syntax highlighting definitions for OtterLang.

## Installation

### VS Code
Copy `otterlang.tmLanguage.json` to:
- **macOS**: `~/Library/Application Support/Code/User/snippets/`
- **Windows**: `%APPDATA%\Code\User\snippets\`
- **Linux**: `~/.config/Code/User/snippets/`

Or install via extension marketplace (when available).

### Vim/Neovim
Use with a plugin that supports TextMate grammars like `vim-polyglot` or `nvim-treesitter`.

### Sublime Text
Copy to `Packages/User/` directory.

### Atom
Copy to `~/.atom/packages/language-otterlang/grammars/`

## Features

- Syntax highlighting for all OtterLang keywords
- String literal highlighting with escape sequence support
- F-string interpolation highlighting
- Number literal highlighting (integers and floats)
- Comment highlighting
- Operator and punctuation highlighting
- Type highlighting

