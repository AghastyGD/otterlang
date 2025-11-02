# Installation Complete! 🎉

The `otter` command is now installed and ready to use!

## Quick Start

```bash
# Run a program
otter run examples/hello.otter

# Build an executable
otter build examples/hello.otter

# Profile memory usage
otter profile memory examples/hello.otter

# List benchmarks
otter bench list examples

# Format code
otter fmt

# Start REPL
otter repl
```

## Installation Location

The `otter` binary is installed at:
```
~/.cargo/bin/otter
```

Make sure `~/.cargo/bin` is in your PATH. If you're using zsh/bash, add this to your `~/.zshrc` or `~/.bashrc`:
```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

## Usage Examples

```bash
# Run with debug output
otter run examples/hello.otter --debug

# Run with timing information
otter run examples/hello.otter --time

# Build optimized release binary
otter build examples/hello.otter --release

# Dump tokens during compilation
otter run examples/hello.otter --dump-tokens

# Dump AST during compilation
otter run examples/hello.otter --dump-ast

# Dump LLVM IR
otter run examples/hello.otter --dump-ir
```

## Updating

To update the installed binary:
```bash
cargo install --path . --bin otter --force
```

