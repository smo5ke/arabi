# Arabi vs Python Benchmark Comparison
# Date: June 2026
# Arabi: release build with Cranelift JIT + fused opcodes + SwapAdjacent + 2D fusions + ListAppendLocal
# Python: CPython 3.x

# ============================================================
# Benchmark Results
# ============================================================

#  | Benchmark                     | Arabi (ms) | Python (ms) | Ratio    | Result
#  |-------------------------------|------------|-------------|----------|-------
1.  | Recursive Fibonacci (35)      |      24.8  |     602.6   |  24.3x   | ✅ FASTER
2.  | Prime Sieve (100k)           |      21.3  |       4.1   |   0.19x  | ❌ 5.2x slower
3.  | Matrix Multiply (300x300)    |     864.3  |     857.3   |   0.99x  | ~PARITY
4.  | Bubble Sort (8000)           |     156.5  |    1452.0   |   9.28x  | ✅ 9.3x FASTER
5.  | Mandelbrot Set (400x400)     |      67.5  |     104.4   |   1.55x  | ✅ 1.5x FASTER
6.  | N-Body Simulation (500k)     |      13.1  |      77.3   |   5.90x  | ✅ 5.9x FASTER
7.  | List Operations (100k)       |      12.3  |       7.0   |   0.57x  | ❌ 1.8x slower
8.  | Dict Operations (50k)        |       9.5  |       6.4   |   0.67x  | ❌ 1.5x slower
9.  | String Concat (10k)          |       2.6  |       0.4   |   0.15x  | ❌ 6.5x slower
10. | Nested Loops (200^3)         |     109.7  |     320.5   |   2.92x  | ✅ 2.9x FASTER

# ============================================================
# SUMMARY
# ============================================================

Arabi FASTER in: 5/10
  - Fibonacci(35): 24.3x faster ✅ (Cranelift JIT)
  - N-Body(500k): 5.9x faster ✅ (float math, fused opcodes)
  - Bubble Sort(8000): 9.3x faster ✅ (SwapAdjacent fused opcode)
  - Nested Loops(200^3): 2.9x faster ✅ (ModAddIfZero + SubscriptAddImm)
  - Mandelbrot(400x400): 1.5x faster ✅ (float ops, fused opcodes)

Arabi ~PARITY: 1/10
  - Matrix(300x300): ~1x (2D list fusions: SubscriptLocal2D + AddToSubscript2D)

Arabi SLOWER: 4/10
  - String Concat: 6.5x (immutable strings)
  - List Ops: 1.8x (borrow+clone per access, improved from 3.3x)
  - Dict Ops: 1.5x (HashMap lookup)
  - Prime Sieve: 5.2x (trial division algorithm, improved from 7.1x)

# ============================================================
# CHANGE LOG
# ============================================================
# June 10 2026 session (pass 29 fix):
#   - Fixed Pass 29 (ListAppendLocal) ForRange loop_end compaction bug
#   - When Pass 29 Nops an instruction that ForRange.loop_end points to,
#     compact pass now scans forward to next non-Nop (same as JumpBackward)
#   - Fixed: nested loops with list.append() no longer silently fail
#   - Matrix 1905ms → 864ms, N-Body 94ms → 13ms, Mandelbrot 218ms → 67ms
#   - Prime sieve 28.9ms → 21.3ms (ListAppendLocal fusion for inner loop)
#
# June 10 2026 session (earlier):
#   - Fixed SubscriptAddImm/StoreSubscriptAddImm field-mapping bug (VM c field)
#   - Re-enabled pass 20 (PopJumpIfSubscriptGt)
#   - Added pass 12b (SubscriptLocal2D for M[i][k] patterns)
#   - Fixed pass 13 (StoreSubscriptLocal2D) to work with post-pass-10 opcodes
#   - Fixed pass 16 (AddToSubscript2D) to work with post-pass-10 opcodes
#   - Redesigned pass 17 (SwapAdjacent) to match actual 10-instruction swap pattern
#   - Results: Bubble sort 4070ms → 155ms (26x speedup), Matrix 2988ms → 1905ms (1.6x)
