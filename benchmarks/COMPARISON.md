# Arabi vs Python Benchmark Comparison
# Date: June 2026
# Arabi: release build (Cranelift JIT + 30+ peephole passes + mimalloc)
# Python: CPython 3.x
# Tool: benchmarks/run_benchmarks.py

======================================================================
Benchmark                 Arabi (ms)      Python (ms)     Ratio      Winner    
======================================================================
01_arithmetic                 58.3ms         75.9ms      1.30x    Arabi
02_strings                     9.8ms         93.5ms      9.58x    Arabi
03_lists                       9.9ms         23.4ms      2.37x    Arabi
04_nested_loops                9.9ms         30.2ms      3.04x    Arabi
05_fibonacci                  51.7ms         70.0ms      1.35x    Arabi
06_closures                   15.2ms         24.2ms      1.59x    Arabi
07_comprehension              14.7ms         21.3ms      1.45x    Arabi
08_classes                    26.7ms         29.8ms      1.12x    Arabi
09_fstrings                   17.8ms         24.1ms      1.36x    Arabi
10_exceptions                  9.0ms         21.2ms      2.36x    Arabi
11_math                       21.1ms         33.8ms      1.60x    Arabi
12_dicts                      10.3ms         37.9ms      3.68x    Arabi
13_factorial_recursion        44.6ms         43.3ms      0.97x    Python
14_bubble_sort                10.2ms         40.0ms      3.91x    Arabi
15_string_processing          25.3ms         22.8ms      0.90x    Python

======================================================================
SUMMARY
======================================================================

Arabi wins: 13/15
Arabi loses: 2/15

Average speed ratio: 2.44x
Arabi is ON AVERAGE 2.44x faster than Python!

Arabi FASTER in:
  - Strings (100K concat): 9.58x faster (Rc<String> + O(1) trim)
  - Bubble Sort (8K): 3.91x faster (fused opcodes + JIT)
  - Dicts (100K): 3.68x faster (optimized HashMap)
  - Nested Loops (1M): 3.04x faster (JIT + fused opcodes)
  - Lists (100K append): 2.37x faster (ListAppendLocal fusion)
  - Exceptions (100K): 2.36x faster (stack unwinding)
  - Math (sin/cos 1M): 1.60x faster (native calls)
  - Closures (100K): 1.59x faster (shared Rc closures)
  - Comprehension (100K): 1.45x faster (fused loops)
  - F-Strings (100K): 1.36x faster (string building)
  - Fibonacci (recursive 35): 1.35x faster (JIT compilation)
  - Arithmetic (10M): 1.30x faster (specialized int/float ops)
  - Classes (100K): 1.12x faster (constructor inlining)

Arabi SLOWER in:
  - String Processing: 0.90x (regex-heavy workload)
  - Factorial Recursion: 0.97x (pure recursion overhead)

======================================================================
CHANGE LOG
======================================================================
June 17 2026: Phase 5 stability hardening — 74 unwrap() calls eliminated
June 15 2026: Match/case + getattr/setattr features added
June 10 2026: JIT TCO + expanded opcodes + JIT bug fixes (3 critical)
June 5 2026: 30+ peephole passes + field_vec + INT_INT specialization
June 1 2026: Initial benchmarks — geometric mean 0.50x vs Python
