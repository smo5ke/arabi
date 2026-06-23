import time

def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)

def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

print("=== Recursion Depth Stress Test ===")

print("Testing fibonacci(35)...")
start = time.perf_counter()
r1 = fibonacci(35)
elapsed = time.perf_counter() - start
print(f"Result: {r1}")
print(f"Time: {elapsed} s")

print("\nTesting factorial(20)...")
start = time.perf_counter()
r2 = factorial(20)
elapsed = time.perf_counter() - start
print(f"Result: {r2}")
print(f"Time: {elapsed} s")

print("\nTesting repeated fibonacci(25)...")
start = time.perf_counter()
for i in range(50):
    result = fibonacci(25)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
