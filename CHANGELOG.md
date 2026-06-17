# Changelog

All notable changes to the Arabi programming language will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] - 2026-06-17

### Added
- **Core Language:**
  - 39 Arabic keywords (control flow, functions, classes, imports, exceptions, generators)
  - Variables, assignment, walrus operator, multi-assign
  - Arithmetic operators (+, -, *, /, //, %, ^)
  - Comparison operators (==, !=, <, >, <=, >=)
  - Logical operators (و, او, ليس)
  - Bitwise operators (&, |, <<, >>, ~)
  - Ternary expressions
  - String interpolation (f-strings)
  - List/dict/set comprehensions with `لكل` syntax
  - Decorator support (`@` and `زخرف` keywords)
  - Context managers (`باستخدام`)
  - Generators and `سلم` (yield)

- **Type System:**
  - Integers, floats, booleans, strings, null
  - Lists, tuples, dictionaries, sets
  - Functions, classes, instances
  - Exceptions (custom and built-in)
  - Closures and lexical scoping

- **Built-in Functions (170+):**
  - I/O: `اطبع`, `ادخل`
  - Type conversion: `صحيح`, `عشري`, `منطق`, `نص`
  - Collections: `مصفوفة`, `مترابطة`, `مميزة`, `طول`, `قائمة_مليئة`
  - Higher-order: `ضغط`, `تصفية`, `اختزال`, `تتبع`, `مقرون`
  - String operations: 30+ string functions
  - Math: 20+ functions + trigonometry + logarithms + constants
  - Time: 13 time functions
  - Random: 11 random functions
  - File I/O: 13 functions
  - JSON: 6 functions
  - System/OS: 22 functions
  - HTTP: `طلب`, `طلب_نص`, `طلب_كائن`, `طلب_ارسال`
  - Crypto: Base64, Hex, URL encoding
  - Regex: 6 pattern matching functions
  - Statistics: mean, median, mode, std_dev, variance

- **Standard Library Modules (6):**
  - `رياضيات` (math) - Advanced math functions
  - `وقت` (time) - Time and date operations
  - `نص` (string) - String processing utilities
  - `عشوائي` (random) - Random number generation
  - `json` - JSON serialization/deserialization
  - `نظام` (os) - System and OS interaction

- **VM and Runtime:**
  - Stack-based virtual machine
  - 30+ peephole optimization passes
  - Tail call optimization (TCO)
  - LoadGlobal optimization for known globals
  - Rc::get_mut fast paths for string/list operations
  - Algebraic simplification (x - A + A -> x, x + 0 -> x, x * 1 -> x)
  - INT_INT specialized opcodes
  - FastMethod dispatch for common operations
  - mimalloc global allocator

- **JIT Compiler:**
  - Cranelift-based JIT compilation
  - Loop function compilation
  - Integer fast path for single-argument functions
  - Runtime helper functions (20 registered symbols)

- **CLI:**
  - File execution (`arabi <file>`)
  - REPL with readline support
  - Debug mode (`--debug`)
  - Package management

- **Testing:**
  - 129 integration tests
  - 15 performance benchmarks
  - 14-section comprehensive stability test

### Security
- Recursion depth limit (1000 frames)
- Memory limit (256MB)
- Division by zero error handling
- Critical `.unwrap()` elimination in VM and builtins

### Performance
- 2.27x faster than CPython (geometric mean across 15 benchmarks)
- 14 wins, 1 tie, 0 losses vs Python
