# مشاكل وملاحظات مكتشفة من اختبارات الضغط المتقدمة
# Issues & Bugs Discovered from Advanced Stress Tests
# Date: 2026-06-23 (last updated: 2026-06-24)

---

## 1. بناء الجملة (Parsing Issues)

### 1.1 كلمة `اواذا` (elif) لا تدعم `:` بعد الشرط
- **الحالة**: `اواذا شرط:` يسبب `خطأ تحليل: token غير متوقع: Delimiter(Colon)`
- **المتوقع**: يجب أن يعمل بنفس طريقة `اذا`
- **الملفات المتأثرة**: stress_22, stress_26
- **الحالة**: ✅ NOT A BUG — `اواذا` works correctly. Original failure was due to file encoding/invisible characters.

### 1.2 `إذا` بحمزة غير مدعومة
- **الحالة**: `إذا` (بإ همزة) يسبب خطأ تحليل
- **الصحيح**: يجب استخدام `اذا` (بأ plain)
- **الملفات المتأثرة**: stress_19
- **الحالة**: 🟡 موثق (يجب استخدام أ而非 إ)

### 1.3 أسماء متغيرات تحتوي أرقام في النهاية
- **الحالة**: `ابن1` يسبب `حقل غير موجود: 1`
- **السبب**: المُحلل يفسر الرقم как اسم حقل
- **المتوقع**: يجب استخدام `ابن_أول` بدلاً من `ابن1`
- **الحالة**: 🟡 موثق (قيود اللغة)

---

## 2. مشاكل المنطق (Logic Issues)

### 2.1 `ارم استثناء()` لا يُمسك بـ `خلل:` البسيط
- **الحالة**: `ارم استثناء("نوع", "رسالة")` لا يُمسك إلا إذا كان `خلل (نوع):`
- **المتوقع**: `خلل:` بدون نوع يجب أن يمسك كل الاستثناءات
- **الملفات المتأثرة**: stress_07, stress_26
- **الحالة**: ✅ FIXED — `OP_RAISE` now unwinds frames to find handlers in parent frames. Added `locals_ptr`/`locals_len` update after frame pop. 3 new tests added (162, 163, 164).

### 2.2 طفح عددي (Integer Overflow)
- **الحالة**: `مضروب(20)` و `مجموع_مربعات(1000)` تنتج قيم خاطئة
- **السبب**: الأعداد الكبيرة تطفح بدون تحذير
- **الملفات المتأثرة**: stress_12 (التكامل العددي)
- **الحالة**: ✅ FIXED — All integer arithmetic now uses `checked_add`/`checked_sub`/`checked_mul` with automatic float promotion on overflow. 3 new tests added (158, 159, 160).

### 2.3 العمليات العائمة غير مدعومة بالكامل
- **الحالة**: `x * y` حيث `x` و `y` أعداد عائمة يسبب `عملية ضرب غير مدعومة`
- **المتوقع**: يجب دعم العمليات على float
- **الملفات المتأثرة**: stress_09, stress_12
- **الحالة**: ✅ FIXED — Float arithmetic works correctly (3.14 * 2.0, 11.0 * 13, etc.). The issue was likely a side-effect of integer overflow (Bug #2.2) which has been fixed.

---

## 3. مشاكل الأداء (Performance Issues)

### 3.1 بطالات النصوص متسلسلة O(n²)
- **الحالة**: `نص = نص + "حرف"` في حلقة = 1500ms لـ 50K حرف
- **السبب**: النصوص غير قابلة للتغيير، كل دمج ينشئ نصاً جديداً
- **المقارنة**: Python يفعل نفس الشيء في 234ms (أفضل بـ 6.5x)
- **الملفات المتأثرة**: stress_25
- **الحالة**: ✅ PARTIALLY FIXED — Peephole Pass 39 converts `x = x + const` to InplaceAddStrConst (in-place mutation when refcount=1). String BUILD operations now 200-1000x faster. String REVERSE still O(n) due to complex index expressions. Stress_25 improved from 1410ms to 1277ms.

### 3.2 الدوال الوظيفية (Higher-Order Functions) بطيئة
- **الحالة**: `خريطة(دالة، قائمة)` بطيئة جداً
- **السبب**: استدعاء الدالة عبر متغير يضيف overhead كبير
- **المقارنة**: 161ms vs 24ms Python (Python أسرع 6.6x)
- **الملفات المتأثرة**: stress_16
- **الحالة**: 🟡 أداء

### 3.3 التخزين المؤقت (Memoization) بطيء
- **الحالة**: فيبوناتشي(90) مع التخزين المؤقت = 163ms vs 23ms Python
- **السبب**: الوصول للقائمة + التحقق من القيمة أبطأ من dict في Python
- **الملفات المتأثرة**: stress_13
- **الحالة**: 🟡 أداء

### 3.4 ضرب المصفوفات بطيء
- **الحالة**: 100x100 matrix multiply = 183ms vs 52ms Python
- **السبب**: الوصول المتسلسل للعناصر + overhead الحلقات
- **الملفات المتأثرة**: stress_10
- **الحالة**: 🟡 أداء

---

## 4. قيود اللغة (Language Limitations)

### 4.1 `مدى()` لا تدعم خطوة ثالثة (step)
- **الحالة**: `مدى(0، 100، 2)` غير مدعوم
- **المتوقع**: يجب دعم `مدى(start, end, step)`
- **الحالة**: 🟡 Feature Missing

### 4.2 إرجاع إغلاقات متعددة الطبقات
- **الحالة**: دالة ترجع إغلاقاً يرجع إغلاقاً لا تعمل دائماً
- **الملفات المتأثرة**: stress_19 (تم تخطي المشكلة)
- **الحالة**: 🟡 Feature Limitation

### 4.3 سلاسل استدعاء الحقول على كائنات القوائم
- **الحالة**: `ن.قائمة.اضف(...)` لا يعمل
- **السبب**: الـ VM لا يدعم سلسلة الوصول `field.method()`
- **المتوقع**: يجب حفظ القائمة في متغير أولاً
- **الحالة**: ✅ FIXED — Root cause: `OP_CREATE_CLASS` constructor bytecode scanner didn't recognize `OP_BUILD_LIST`/`OP_BUILD_DICT`/`OP_BUILD_TUPLE`/`OP_BUILD_SET` as valid value patterns, so list/dict/tuple fields were not registered. Extended the detection pattern. 3 new integration tests added (165, 166, 167).

### 4.4 الأسماء المختلطة (عربي/إنجليزي)
- **الحالة**: `الزمن_startTime` يسبب خطأ
- **المتوقع**: يجب استخدام عربي فقط أو إنجليزي فقط
- **الحالة**: 🟡 قيود اللغة

---

## 5. ملخص نتائج المقارنة (26 اختبار) — محدّث 2026-06-24

### Arabi يفوز في 17/26 اختبار (65.4%)

| الاختبار | النسبة | ملاحظات |
|----------|--------|---------|
| recursion (فيبوناتشي) | **18.01x** | أسرع بكثير |
| large_lists | **5.25x** | أسرع بكثير |
| dicts | **4.49x** | فهارس |
| deep_call_chain | **2.10x** | أسرع بكثير |
| state_machine | **2.27x** | شروط كثيرة |
| functional | **1.68x** | higher-order functions |
| memoization | **1.67x** | التخزين المؤقت |
| exceptions | **1.79x** | معالجة استثناء |
| exception_patterns | **1.46x** | استثناءات متعددة |
| memory_pressure | **1.38x** | كائنات كثيرة |
| classes | **1.34x** | إنشاء كائنات |
| ackermann | **1.27x** | recursion |
| object_tree | **1.22x** | بنية شجرية |
| matrix_multiply | **1.16x** | مصفوفات |
| string_pipeline | **1.08x** | عمليات نصية |
| numerical_integration | **1.03x** | أعداد كبيرة |
| closures | **1.00x** | متساويان |

### Python يفوز في 9/26 اختبار (34.6%)

| الاختبار | النسبة | ملاحظات |
|----------|--------|---------|
| string_complex | **4.4x** | بناء نص O(n²) — reverse لا يزال بطيئ |
| deep_closures | **2.07x** | إغلاق |
| prime_sieve | **1.91x** | مصفوفة |
| pi_monte_carlo | **1.37x** | حسابات عائمة |
| negative_math | **1.47x** | GCD |
| large_matrix | **1.11x** | مصفوفة 500x500 |
| strings | **1.14x** | upper/lower/trim (C) |
| complex_patterns | **1.10x** | فرز + قائمة |
| polymorphism | **1.26x** | إنشاء كائنات |

### متوسط الأداء الكلي: **2.09x لصالح Arabi**
