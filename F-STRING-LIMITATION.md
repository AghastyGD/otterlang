# F-String Interpolation Limitation

## Current Status

F-string interpolation is **partially implemented** but has significant limitations:

### What Works:
- **Tokenization**: F-strings are correctly identified as `f"..."` tokens
- **Parsing**: Expressions inside `{...}` are extracted
- **Compilation**: F-strings compile successfully

### What Doesn't Work:
- **Runtime Value Interpolation**: Values are NOT inserted at runtime
- **String Formatting**: No numeric-to-string conversion at runtime
- **String Concatenation**: Parts are not properly concatenated

## The Problem

The original `pi_benchmark.otter` uses f-strings like:
```otter
print(f"π ≈ {result}")
print(f"Time: {duration} ms for {iterations} iterations")
```

Currently, these compile but output:
- Raw UTF-8 bytes for emoji characters
- Placeholder text instead of actual variable values
- Malformed output

## Why This Is Hard

F-string interpolation requires:

1. **Runtime sprintf/snprintf**: Need C runtime functions to format numbers as strings
2. **Dynamic Memory Allocation**: Must allocate buffers for concatenated strings
3. **Type-aware Formatting**: Different format specifiers for int vs float
4. **Memory Management**: Must free allocated string buffers

## Workaround

For now, use regular `print()` statements without interpolation:

```otter
fn main:
    print("Starting pi benchmark...")
    result = calculate_pi()
    print("Pi calculation complete!")
```

## Implementation Plan

To properly implement f-strings, we need to:

1. Add `sprintf` to the C runtime shim
2. Implement string buffer allocation in LLVM IR
3. Generate format strings based on expression types
4. Call sprintf for each interpolated value
5. Concatenate all parts into final string
6. Return pointer to concatenated string

This is a significant undertaking and is deferred for now.

## Current Recommendation

**Use the simplified benchmark** (`pi_benchmark_simple.otter`) which demonstrates:
- ✅ Module imports (`use otter:time`)
- ✅ Member access (`time.now()`)
- ✅ Function parameters with types
- ✅ Return values
- ✅ For loops with ranges
- ✅ Type coercion (int ↔ float)
- ✅ All arithmetic and comparison operators

The core language features work perfectly - only f-string *interpolation* is missing.

