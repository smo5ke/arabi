import time

def integrate(n):
    total = 0
    for i in range(n):
        x = i
        x2 = x * x
        x3 = x2 * x
        total += x3 + x2 + x + 1
    return total

print("=== Numerical Integration ===")

print("100,000 slices...")
start = time.perf_counter()
result1 = integrate(100000)
elapsed = time.perf_counter() - start
print(f"Result: {result1}")
print(f"Time: {elapsed} s")

print("\n500,000 slices...")
start = time.perf_counter()
result2 = integrate(500000)
elapsed = time.perf_counter() - start
print(f"Result: {result2}")
print(f"Time: {elapsed} s")
