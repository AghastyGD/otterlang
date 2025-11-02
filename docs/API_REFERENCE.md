# OtterLang API Reference

Complete API reference for OtterLang standard library.

## Built-in Functions

### `print(message: string) -> unit`

Prints a message to standard output.

**Parameters:**
- `message`: The string to print

**Example:**
```otter
print("Hello, World!")
```

### `println(message: string) -> unit`

Prints a message to standard output followed by a newline.

**Parameters:**
- `message`: The string to print

**Example:**
```otter
println("Hello, World!")
```

### `len(collection: array | string) -> int`

Returns the length of an array or string.

**Parameters:**
- `collection`: An array or string

**Returns:** The length as an integer

**Example:**
```otter
length = len([1, 2, 3])  # Returns 3
str_len = len("hello")   # Returns 5
```

### `cap(array: array) -> int`

Returns the capacity of an array.

**Parameters:**
- `array`: An array

**Returns:** The capacity as an integer

## Module: `io`

### `read_line() -> string | nil`

Reads a line from standard input.

**Returns:** The line as a string, or `nil` on EOF

**Example:**
```otter
line = read_line()
if line != nil:
    println(f"You entered: {line}")
```

## Module: `math`

### `sqrt(x: float) -> float`

Returns the square root of x.

**Parameters:**
- `x`: A non-negative number

**Returns:** The square root

### `sin(x: float) -> float`

Returns the sine of x (in radians).

### `cos(x: float) -> float`

Returns the cosine of x (in radians).

### `tan(x: float) -> float`

Returns the tangent of x (in radians).

### `pow(base: float, exponent: float) -> float`

Returns base raised to the power of exponent.

### `abs(x: float) -> float`

Returns the absolute value of x.

### `max(a: float, b: float) -> float`

Returns the maximum of two values.

### `min(a: float, b: float) -> float`

Returns the minimum of two values.

## Module: `time`

### `now_ms() -> int`

Returns the current time in milliseconds since Unix epoch.

**Returns:** Milliseconds since epoch

**Example:**
```otter
timestamp = now_ms()
```

### `sleep_ms(ms: int) -> unit`

Sleeps for the specified number of milliseconds.

**Parameters:**
- `ms`: Milliseconds to sleep

**Example:**
```otter
sleep_ms(1000)  # Sleep for 1 second
```

## Module: `json`

### `parse(json_str: string) -> dict | array | nil`

Parses a JSON string into a dictionary or array.

**Parameters:**
- `json_str`: A valid JSON string

**Returns:** Parsed value or `nil` on error

**Example:**
```otter
data = json.parse('{"name": "Otter", "age": 42}')
if data != nil:
    name = data["name"]
```

### `stringify(value: dict | array) -> string`

Converts a dictionary or array to a JSON string.

**Parameters:**
- `value`: A dictionary or array

**Returns:** JSON string representation

**Example:**
```otter
json_str = json.stringify({"key": "value"})
```

## Module: `runtime`

### `collect_garbage() -> int`

Manually triggers garbage collection.

**Returns:** Bytes freed

**Example:**
```otter
freed = runtime.collect_garbage()
println(f"Freed {freed} bytes")
```

### `memory_profiler_start() -> unit`

Starts memory profiling.

### `memory_profiler_stop() -> unit`

Stops memory profiling.

### `memory_profiler_stats() -> string`

Returns memory profiling statistics as JSON.

**Returns:** JSON string with profiling statistics

### `memory_profiler_leaks() -> string`

Detects and returns memory leaks as JSON.

**Returns:** JSON string with leak information

### `set_gc_strategy(strategy: string) -> unit`

Sets the garbage collection strategy.

**Parameters:**
- `strategy`: One of "rc", "marksweep", "hybrid", "none"

## Module: `task`

### `spawn(block: () -> T) -> Task<T>`

Spawns a concurrent task.

**Parameters:**
- `block`: A function block to execute concurrently

**Returns:** A Task handle

**Example:**
```otter
task = spawn:
    return compute_result()
```

### `await(task: Task<T>) -> T`

Waits for a task to complete and returns its result.

**Parameters:**
- `task`: A Task handle

**Returns:** The task's result

**Example:**
```otter
task = spawn:
    return 42
result = await task
```

## Type Definitions

### `Task<T>`

Represents a concurrent task that returns type T.

### Built-in Types

- `int`: 64-bit signed integer
- `float`: 64-bit floating point
- `bool`: Boolean (`true` or `false`)
- `string`: UTF-8 string
- `unit`: Unit type (void)
- `array<T>`: Dynamic array of type T
- `dict<K, V>`: Dictionary mapping K to V

See [Language Specification](./LANGUAGE_SPEC.md) for more details.

