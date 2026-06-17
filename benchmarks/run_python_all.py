import time, math, sys, io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

def bench(name, func):
    t0 = time.perf_counter()
    result = func()
    t1 = time.perf_counter()
    ms = (t1 - t0) * 1000
    print(f"{name}: {ms:.1f}ms")
    return ms

results = {}

# 01: Arithmetic
def arithmetic():
    s = 0
    for i in range(1000000):
        s += i * 3 + 7 - 2 // 1 + 5 % 3
    return s
results["01_arithmetic"] = bench("01_arithmetic", arithmetic)

# 02: Strings
def strings():
    s = ""
    for i in range(10000):
        s += "x"
    return len(s)
results["02_strings"] = bench("02_strings", strings)

# 03: Lists
def lists():
    l = []
    for i in range(10000):
        l.append(i)
    l2 = sorted(l)
    return len(l2)
results["03_lists"] = bench("03_lists", lists)

# 04: Nested Loops
def nested_loops():
    s = 0
    for i in range(500):
        for j in range(500):
            s += i * j
    return s
results["04_nested_loops"] = bench("04_nested_loops", nested_loops)

# 05: Fibonacci
def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)
def fibonacci():
    return fib(30)
results["05_fibonacci"] = bench("05_fibonacci", fibonacci)

# 06: Closures (function calls)
def square(x):
    return x * x
def closures():
    s = 0
    for i in range(100000):
        s += square(i)
    return s
results["06_closures"] = bench("06_closures", closures)

# 07: Comprehension
def comprehension():
    l = [i ** 2 for i in range(50000)]
    return len(l)
results["07_comprehension"] = bench("07_comprehension", comprehension)

# 08: Classes
class Pointra:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    def area(self):
        return self.x + self.y
def classes():
    for i in range(100000):
        p = Pointra(i, i * 2)
        p.area()
    return "done"
results["08_classes"] = bench("08_classes", classes)

# 09: FStrings
def fstrings():
    s = ""
    for i in range(10000):
        s = f"val: {i}"
    return s
results["09_fstrings"] = bench("09_fstrings", fstrings)

# 10: Exceptions
def exceptions():
    s = 0
    for i in range(10000):
        try:
            s += 1
        except:
            pass
    return s
results["10_exceptions"] = bench("10_exceptions", exceptions)

# 11: Math
def math_bench():
    for i in range(100000):
        math.sin(i)
        math.cos(i)
        math.tan(i)
    return "done"
results["11_math"] = bench("11_math", math_bench)

# 12: Dicts
def dicts():
    d = {}
    for i in range(10000):
        d[str(i)] = i
    return len(d)
results["12_dicts"] = bench("12_dicts", dicts)

# 13: Factorial Recursion
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)
def factorial_bench():
    for i in range(50000):
        factorial(20)
    return "done"
results["13_factorial"] = bench("13_factorial", factorial_bench)

# 14: Bubble Sort
def bubble_sort(arr):
    n = len(arr)
    for i in range(n - 1):
        for j in range(n - 1 - i):
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
    return arr
def bubble_bench():
    for i in range(100):
        lst = [100, 99, 98, 97, 96, 95, 94, 93, 92, 91, 90, 89, 88, 87, 86, 85, 84, 83, 82, 81, 80, 79, 78, 77, 76, 75, 74, 73, 72, 71, 70, 69, 68, 67, 66, 65, 64, 63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51]
        bubble_sort(lst)
    return "done"
results["14_bubble_sort"] = bench("14_bubble_sort", bubble_bench)

# 15: String Processing
def string_processing():
    original = "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10"
    for i in range(10000):
        s1 = original.upper()
        s2 = s1.lower()
        s3 = s2.strip()
        s4 = s3.replace("word1", "done")
        s5 = s4.split(" ")
        s6 = len(s5)
    return "done"
results["15_string_processing"] = bench("15_string_processing", string_processing)

print("\n===== PYTHON SUMMARY =====")
for k, v in results.items():
    print(f"{k}: {v:.1f}ms")
