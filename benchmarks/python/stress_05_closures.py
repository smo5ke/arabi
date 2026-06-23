import time

def make_counter(start):
    count = start
    def inc():
        nonlocal count
        count += 1
        return count
    def dec():
        nonlocal count
        count -= 1
        return count
    return inc, dec

def make_multiplier(factor):
    def multiply(n):
        return n * n * factor
    return multiply

print("=== Closures Stress Test ===")

print("Testing 5000 independent counters...")
start = time.perf_counter()
counters = []
for i in range(5000):
    c = make_counter(i)
    counters.append(c)

for c in counters:
    c[0]()
    c[0]()
    c[1]()
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting 5000 multiplier functions...")
start = time.perf_counter()
funcs = []
for i in range(5000):
    funcs.append(make_multiplier(i))

results = []
for i in range(100):
    for m in funcs:
        result = m(i)
        results.append(result)
elapsed = time.perf_counter() - start
print(f"Results: {len(results)}")
print(f"Time: {elapsed} s")

print("\nTesting nested closures (3 levels)...")
start = time.perf_counter()
result_n = 0
for i in range(100):
    def level1(a):
        def level2(b):
            def level3(c):
                return a + b + c
            return level3
        return level2
    m3 = level1(i)
    m2 = m3(i * 2)
    result_n = m2(i * 3)
elapsed = time.perf_counter() - start
print(f"Result: {result_n}")
print(f"Time: {elapsed} s")
