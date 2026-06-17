import sys, io, os
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

COMMA = '\u060c'
DQ = '"'
S = lambda t: DQ + t + DQ

os.makedirs('benchmarks/arabi', exist_ok=True)
os.makedirs('benchmarks/python', exist_ok=True)

def write_file(path, lines):
    with open(path, 'w', encoding='utf-8-sig') as f:
        f.write('\n'.join(lines) + '\n')

# ============================================================================
# 01: Arithmetic
# ============================================================================
write_file('benchmarks/arabi/01_arithmetic.arabi', [
    'س = 0',
    'لكل ع في مدى(1000000):',
    '    س += ع * 3 + 7 - 2 \\ 1 + 5 % 3',
    'اطبع(س)',
])
write_file('benchmarks/python/01_arithmetic.py', [
    's = 0',
    'for i in range(1000000):',
    '    s += i * 3 + 7 - 2 // 1 + 5 % 3',
    'print(s)',
])

# ============================================================================
# 02: String Concatenation
# ============================================================================
write_file('benchmarks/arabi/02_strings.arabi', [
    'س = ""',
    'لكل ط في مدى(10000):',
    '    س += "x"',
    'اطبع(طول(س))',
])
write_file('benchmarks/python/02_strings.py', [
    's = ""',
    'for i in range(100000):',
    '    s += "x"',
    'print(len(s))',
])

# ============================================================================
# 03: List Operations
# ============================================================================
write_file('benchmarks/arabi/03_lists.arabi', [
    'ق = []',
    'لكل ط في مدى(10000):',
    '    ق.اضف(ع)',
    'ق2 = مرتب(ق)',
    'اطبع(طول(ق2))',
])
write_file('benchmarks/python/03_lists.py', [
    'q = []',
    'for i in range(100000):',
    '    q.append(i)',
    'q2 = sorted(q)',
    'print(len(q2))',
])

# ============================================================================
# 04: Nested Loops
# ============================================================================
write_file('benchmarks/arabi/04_nested_loops.arabi', [
    'س = 0',
    'لكل ع في مدى(500):',
    '    لكل ج في مدى(500):',
    '        س += ع * ج',
    'اطبع(س)',
])
write_file('benchmarks/python/04_nested_loops.py', [
    's = 0',
    'for i in range(500):',
    '    for j in range(500):',
    '        s += i * j',
    'print(s)',
])

# ============================================================================
# 05: Fibonacci (Recursion)
# ============================================================================
write_file('benchmarks/arabi/05_fibonacci.arabi', [
    'دالة فيب(ن):',
    '    اذا ن <= 1:',
    '        ارجع ن',
    '    ارجع فيب(ن - 1) + فيب(ن - 2)',
    'اطبع(فيب(30))',
])
write_file('benchmarks/python/05_fibonacci.py', [
    'def fib(n):',
    '    if n <= 1:',
    '        return n',
    '    return fib(n - 1) + fib(n - 2)',
    'print(fib(30))',
])

# ============================================================================
# 06: Closures
# ============================================================================
write_file('benchmarks/arabi/06_closures.arabi', [
    'دالة صانع(البداية):',
    '    العداد = البداية',
    '    دالة زِد():',
    '        العداد += 1',
    '        ارجع العداد',
    '    ارجع زِد',
    'عد = صانع(0)',
    'لكل ط في مدى(10000):',
    '    عد()',
    'اطبع(عد())',
])
write_file('benchmarks/python/06_closures.py', [
    'def make_counter(start):',
    '    count = start',
    '    def inc():',
    '        nonlocal count',
    '        count += 1',
    '        return count',
    '    return inc',
    'c = make_counter(0)',
    'for i in range(100000):',
    '    c()',
    'print(c())',
])

# ============================================================================
# 07: List Comprehension
# ============================================================================
write_file('benchmarks/arabi/07_comprehension.arabi', [
    'ق = [ع ^ 2 لكل ع في مدى(100000)]',
    'اطبع(طول(ق))',
])
write_file('benchmarks/python/07_comprehension.py', [
    'q = [i ** 2 for i in range(100000)]',
    'print(len(q))',
])

# ============================================================================
# 08: Classes
# ============================================================================
write_file('benchmarks/arabi/08_classes.arabi', [
    'صنف نقطة:',
    '    دالة __تهيئة__(ذ، س، ص):',
    '        ذ.x = س',
    '        ذ.y = ص',
    '    دالة مساحة(ذ):',
    '        ارجع ذ.x + ذ.y',
    'لكل ط في مدى(10000):',
    '    ن = نقطة(ع، ع*2)',
    '    ن.مساحة()',
    'اطبع("done")',
])
write_file('benchmarks/python/08_classes.py', [
    'class Point:',
    '    def __init__(self, x, y):',
    '        self.x = x',
    '        self.y = y',
    '    def area(self):',
    '        return self.x + self.y',
    'for i in range(100000):',
    '    p = Point(i, i*2)',
    '    p.area()',
    'print("done")',
])

# ============================================================================
# 09: F-Strings
# ============================================================================
write_file('benchmarks/arabi/09_fstrings.arabi', [
    'س = ""',
    'لكل ط في مدى(10000):',
    '    س = م"القيمة: {ع}"',
    'اطبع(س)',
])
write_file('benchmarks/python/09_fstrings.py', [
    's = ""',
    'for i in range(100000):',
    '    s = f"value: {i}"',
    'print(s)',
])

# ============================================================================
# 10: Exception Handling
# ============================================================================
write_file('benchmarks/arabi/10_exceptions.arabi', [
    'س = 0',
    'لكل ط في مدى(10000):',
    '    حاول:',
    '        س += 1',
    'اطبع(س)',
])
write_file('benchmarks/python/10_exceptions.py', [
    's = 0',
    'for i in range(100000):',
    '    try:',
    '        s += 1',
    '    except:',
    '        pass',
    'print(s)',
])

# ============================================================================
# 11: Math Functions
# ============================================================================
write_file('benchmarks/arabi/11_math.arabi', [
    'لكل ط في مدى(10000):',
    '    جيب(ع)',
    '    تجيب(ع)',
    '    ظل(ع)',
    'اطبع("done")',
])
write_file('benchmarks/python/11_math.py', [
    'import math',
    'for i in range(100000):',
    '    math.sin(i)',
    '    math.cos(i)',
    '    math.tan(i)',
    'print("done")',
])

# ============================================================================
# 12: Dictionary Operations
# ============================================================================
write_file('benchmarks/arabi/12_dicts.arabi', [
    'ق = {}',
    'لكل ط في مدى(10000):',
    '    ق[نص(ع)] = ع',
    'اطبع(طول(ق))',
])
write_file('benchmarks/python/12_dicts.py', [
    'q = {}',
    'for i in range(100000):',
    '    q[str(i)] = i',
    'print(len(q))',
])

print('All 12 benchmark pairs written!')
