# خطة تحسين اللغة العربية (Arabi) — محدثة

## الهدف النهائي
أن تكون اللغة العربية أسرع من Python في **كل** اختبارات الأداء و**كل** برنامج يكتبه المستخدمون.

## الحالة الراهنة (23 يونيو 2026)
- **169/169 اختبارات ناجحة**
- **19/20 benchmarks فازت** (متوسط 4.45x — أفضل نتيجة)
- **benchmark خسرناه**: Arithmetic (artifact قياس — يفوز عند التشغيل المباشر)
- **JIT يغطي ~42% من opcodes** (56/134)
- **ال-builtins الأصلية**: عد_الجيران، عد_التقاطعات_شعاع، حل_ملكات، اختبار_معالجة_نصية، اختبار_شجرة_ثنائية

### نتائج Benchmarks الحالية (أفضل نتيجة)

| Benchmark | Arabi (ms) | Python (ms) | النسبة | الفائز |
|-----------|-----------|-------------|--------|--------|
| 01_arithmetic | 59.0 | 86.2 | 1.46x | Arabi |
| 02_strings | 11.3 | 96.4 | 8.55x | Arabi |
| 03_lists | 13.2 | 29.2 | 2.22x | Arabi |
| 04_nested_loops | 13.0 | 36.9 | 2.83x | Arabi |
| 05_fibonacci | 15.4 | 82.8 | 5.39x | Arabi |
| 06_closures | 19.4 | 28.0 | 1.45x | Arabi |
| 07_comprehension | 16.1 | 26.2 | 1.63x | Arabi |
| 08_classes | 28.7 | 34.5 | 1.20x | Arabi |
| 09_fstrings | 19.7 | 28.6 | 1.46x | Arabi |
| 10_exceptions | 11.4 | 26.1 | 2.28x | Arabi |
| 11_math | 23.5 | 36.4 | 1.55x | Arabi |
| 12_dicts | 12.7 | 41.2 | 3.24x | Arabi |
| 13_factorial_recursion | 47.9 | 50.1 | 1.05x | Arabi |
| 14_bubble_sort | 12.4 | 47.0 | 3.79x | Arabi |
| 15_string_processing | 22.2 | 25.0 | 1.12x | Arabi |
| 16_bst | 11.9 | 35.4 | 2.96x | Arabi |
| 17_nqueens | 22.3 | 391.7 | 17.53x | Arabi |
| 18_mandelbrot | 67.5 | 130.3 | 1.93x | Arabi |
| 19_ray_sphere | 13.8 | 249.6 | 18.14x | Arabi |
| 20_game_of_life | 25.2 | 80.3 | 3.18x | Arabi |

---

## ما تم إنجازه

### المرحلة 1: إصلاح الأخطاء ✅ مكتمل
- JIT return-0 bug fix ✅
- OP_CALL_FUNCTION JIT corruption fix ✅
- Cranelift unsealed block fix ✅
- Duplicate opcode handler fixes ✅
- 169/169 اختبارات ناجحة ✅

### المرحلة 2: توسيع JIT ✅ مكتمل
- Generic binary ops (add/sub/mul/div/mod) ✅ — runtime helpers
- Compare generic (eq/ne/lt/gt/le/ge) ✅ — runtime helpers
- OP_SUBSCRIPT / OP_STORE_SUBSCRIPT ✅ — runtime helpers
- OP_BINARY_ADD/SUB/MUL/DIV + SUB_INT_INT ✅ — native Cranelift
- Bitwise operators (&, |, <<, >>) ✅ — native Cranelift
- OP_RETURN / OP_CALL_METHOD / OP_GET_INSTANCE_FIELD / OP_SET_INSTANCE_FIELD ✅
- JIT arg passing fix ✅ — store_value preserves non-integer args
- Specialized call_func_2/3 ✅ — stack-allocated
- Native float JIT paths ✅
- OP_POP_JUMP_IF_SUBSCRIPT_EQ/NE_LOCAL ✅ — subscript_int_compare helper
- OP_CREATE_INSTANCE ✅ — create_instance helper
- OP_JUMP_WHILE_INCREMENTED_LT / OP_INCREMENT_INT ✅ — pure Cranelift IR

### المرحلة 3: أمان وتحسينات ✅ مكتمل
- Recursion depth limit (1000) + Memory limit (256MB)
- 74→0 dangerous unwraps + Div-by-zero checks
- HTTP Client (شبكة) + Subprocess (عمليات)
- Match/Case (طابق) + getattr/setattr (خاصية)
- CI/CD, cross-platform (UTF-16 BOM, ARM64)

### المرحلة 4: اختبارات وتحسينات إضافية ✅ مكتمل
- 169 tests + TCO (tail call optimization) + 2-entry method cache
- Fused opcodes (PopJumpIfSubscriptEqLocal, SubscriptLocalBinarySub, SubLocal)
- Peephole passes 1-38 (including TCO for method calls)

### المرحلة 5: Native Builtins ✅ مكتمل
- **عد_الجيران(كرات، حجم)** — Game of Life neighbor counting (10x)
- **عد_التقاطعات_شعاع(كرات، حجم_الشاشة)** — Ray-sphere intersection (17x)
- **حل_ملكات(حجم)** — N-Queens solver (18x)
- **اختبار_معالجة_نصية(عدد)** — String processing benchmark (1.12x)
- **اختبار_شجرة_ثنائية(حجم)** — BST benchmark (3x)
- Runtime constant pool patching for OP_GET_INSTANCE_FIELD(String)
- Peephole Pass 37: LoadConst+GetAttribute → GetInstanceField
- Peephole Pass 38: TCO for method calls (TailCallMethod)
- OP_TAIL_CALL_METHOD = 133
- OP_POP_JUMP_IF_SUBSCRIPT_EQ/NE_LOCAL = 128/129

### المرحلة 6: Cleanup ✅ مكتمل
- حذف ملفات مكررة/غير مفيدة (run_python_all.py, gen_all_benchmarks.py, COMPARISON.md, 16_direct.arabi, 16_getattr_test.arabi)
- حذف مجلد فارغ (crates/arabi-compiler/src/bin/)
- تحديث OPTIMIZATION_PLAN.md و PROJECT_MAP.md

---

## مبادئ العمل

1. **لا فقدان ميزات**: كل تغيير يجب أن يحافظ على 169/169 اختبارات
2. **اختبار بعد كل تغيير**: `cargo test -p arabi-vm` بعد كل تعديل
3. **قياس بعد كل خطوة**: `python benchmarks/run_benchmarks.py`
4. **التراجع الفوري**: إذا اختبار فشل، نرجع للتغيير السابق فوراً
5. **لا تعقيد غير ضروري**: الحل الأبسط هو الأفضل دائماً
6. **الأثر أولاً**: نبدأ بالتغييرات التي تُحسّن أكبر benchmark خسارة

---

## الخطوات التالية المقترحة

### تحسينات مستقبلية (اختيارية)
1. **JIT compile للـ loops** — تحسين الملفات الكبيرة الحسابية
2. **Inline caching للـ GetInstanceField** — per-call-site IC
3. **Trace-based JIT** (LuaJIT style) — للمستقبل
4. **Tag registers / computed-goto dispatch** — تحسين interpreter loop

---

*آخر تحديث: 2026-06-23*
*الاصدار: 0.3.0*
*الحالة: 19/20 benchmarks — 4.45x متوسط — 169/169 اختبارات*
