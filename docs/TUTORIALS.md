# OtterLang Tutorial Series

## Getting Started

Welcome to OtterLang! This tutorial series will guide you through learning the language.

## Table of Contents

1. [Installation](#installation)
2. [Your First Program](#your-first-program)
3. [Variables and Types](#variables-and-types)
4. [Functions](#functions)
5. [Control Flow](#control-flow)
6. [Collections](#collections)
7. [Structs and Types](#structs-and-types)
8. [Concurrency](#concurrency)
9. [Error Handling](#error-handling)
10. [Advanced Topics](#advanced-topics)

## Installation

See the main [README](../README.md) for installation instructions.

## Your First Program

Create a file `hello.otter`:

```otter
fn main:
    print("Hello, World!")
```

Run it:

```bash
otter run hello.otter
```

## Variables and Types

### Basic Types

```otter
fn main:
    # Integers
    x = 42
    
    # Floats
    pi = 3.14
    
    # Strings
    name = "Otter"
    
    # Booleans
    is_active = true
    
    # Arrays
    numbers = [1, 2, 3, 4, 5]
    
    # Dictionaries
    person = {"name": "Otter", "age": 42}
```

### Type Annotations

```otter
fn main:
    x: int = 42
    name: string = "Otter"
```

## Functions

### Basic Functions

```otter
fn greet(name: string) -> string:
    return f"Hello, {name}!"

fn main:
    message = greet("World")
    println(message)
```

### Multiple Parameters

```otter
fn add(x: int, y: int) -> int:
    return x + y

fn main:
    result = add(10, 20)
    println(f"Result: {result}")
```

## Control Flow

### If Statements

```otter
fn check_age(age: int):
    if age >= 18:
        println("Adult")
    elif age >= 13:
        println("Teenager")
    else:
        println("Child")
```

### Match Expressions

```otter
fn day_name(day: int) -> string:
    return match day:
        1 => "Monday"
        2 => "Tuesday"
        3 => "Wednesday"
        _ => "Unknown"
```

### Loops

```otter
fn main:
    # While loop
    mut i = 0
    while i < 10:
        println(f"Count: {i}")
        i = i + 1
    
    # For loop
    for num in [1, 2, 3, 4, 5]:
        println(f"Number: {num}")
```

## Collections

### Arrays

```otter
fn main:
    numbers = [1, 2, 3, 4, 5]
    
    # Access elements
    first = numbers[0]
    
    # Iterate
    for num in numbers:
        println(f"{num}")
```

### Dictionaries

```otter
fn main:
    person = {"name": "Otter", "age": 42}
    
    # Access values
    name = person["name"]
    age = person["age"]
```

## Structs and Types

### Defining Structs

```otter
struct Point:
    x: float
    y: float

fn main:
    p = Point{x: 1.0, y: 2.0}
    println(f"Point: ({p.x}, {p.y})")
```

### Type Aliases

```otter
type ID = int
type Name = string

fn create_user(id: ID, name: Name):
    # ...
```

## Concurrency

### Spawning Tasks

```otter
fn main:
    task1 = spawn:
        return compute_something()
    
    task2 = spawn:
        return compute_something_else()
    
    result1 = await task1
    result2 = await task2
    
    println(f"Results: {result1}, {result2}")
```

## Error Handling

OtterLang uses `nil` for error handling:

```otter
fn safe_divide(a: float, b: float) -> float | nil:
    if b == 0:
        return nil
    return a / b

fn main:
    result = safe_divide(10, 2)
    if result != nil:
        println(f"Result: {result}")
    else:
        println("Division by zero!")
```

## Advanced Topics

### Generics

```otter
fn first<T>(items: [T]) -> T:
    return items[0]

fn main:
    numbers = [1, 2, 3]
    first_num = first(numbers)
    
    strings = ["a", "b", "c"]
    first_str = first(strings)
```

### F-Strings

```otter
fn main:
    name = "Otter"
    age = 42
    message = f"Hello, {name}! You are {age} years old."
    println(message)
```

For more information, see the [Language Specification](./LANGUAGE_SPEC.md) and [API Reference](./API_REFERENCE.md).

