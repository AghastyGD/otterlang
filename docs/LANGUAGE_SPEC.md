# OtterLang Language Specification

## Version 0.1.0

This document specifies the OtterLang programming language syntax and semantics.

## Table of Contents

1. [Introduction](#introduction)
2. [Lexical Structure](#lexical-structure)
3. [Syntax](#syntax)
4. [Type System](#type-system)
5. [Semantics](#semantics)
6. [Standard Library](#standard-library)

## Introduction

OtterLang is an indentation-sensitive programming language with an LLVM backend. It emphasizes simplicity, performance, and developer ergonomics.

### Design Principles

- **Indentation-based syntax**: Uses whitespace for block structure (similar to Python)
- **Type inference**: Types are inferred where possible
- **Static typing**: Type checking happens at compile time
- **Performance**: Compiles to native code via LLVM

## Lexical Structure

### Keywords

The following are reserved keywords in OtterLang:

```
fn, if, elif, else, match, for, while, return, break, continue,
true, false, nil, let, mut, struct, type, use, import, spawn, await
```

### Identifiers

Identifiers start with a letter or underscore, followed by letters, digits, or underscores.

### Literals

- **Integers**: `42`, `-10`, `0xFF`
- **Floats**: `3.14`, `-0.5`, `1e10`
- **Strings**: `"hello"`, `'world'`, `f"Hello, {name}!"`
- **Booleans**: `true`, `false`
- **Arrays**: `[1, 2, 3]`
- **Dictionaries**: `{"key": "value"}`

## Syntax

### Functions

```otter
fn function_name(param: type) -> return_type:
    # function body
    return value
```

### Variables

```otter
# Immutable variable
x = 42

# Mutable variable
mut y = 10
y = 20
```

### Control Flow

#### If Statements

```otter
if condition:
    # block
elif other_condition:
    # block
else:
    # block
```

#### Match Expressions

```otter
result = match value:
    pattern1 => expression1
    pattern2 => expression2
    _ => default
```

#### Loops

```otter
# While loop
while condition:
    # block

# For loop
for item in collection:
    # block
```

### Structs

```otter
struct Point:
    x: float
    y: float

# Usage
p = Point{x: 1.0, y: 2.0}
```

### Type Aliases

```otter
type ID = int
type Name = string
```

### Generics

```otter
fn first<T>(items: [T]) -> T:
    return items[0]
```

## Type System

### Built-in Types

- `int`: 64-bit signed integer
- `float`: 64-bit floating point
- `bool`: Boolean
- `string`: UTF-8 string
- `unit`: Unit type (void)
- `array<T>`: Dynamic array
- `dict<K, V>`: Dictionary/map

### Type Inference

OtterLang performs type inference for most expressions:

```otter
x = 42        # inferred as int
y = 3.14      # inferred as float
name = "Otter" # inferred as string
```

### Type Annotations

Type annotations are optional but can be used for clarity:

```otter
fn add(x: int, y: int) -> int:
    return x + y
```

## Semantics

### Memory Management

OtterLang uses reference counting for memory management with optional garbage collection.

### Concurrency

OtterLang supports concurrent execution through tasks:

```otter
task = spawn:
    # concurrent code

result = await task
```

## Standard Library

See [API Reference](./API_REFERENCE.md) for complete standard library documentation.

Common modules:
- `io`: Input/output operations
- `math`: Mathematical functions
- `time`: Time and date operations
- `json`: JSON parsing and generation
- `runtime`: Runtime utilities

## Future Extensions

- Async/await syntax
- Pattern matching enhancements
- Trait system
- Module system improvements

