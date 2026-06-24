import time
import random

def estimate_pi(n):
    inside = 0
    for i in range(n):
        x = (12345 + i * 6789) % 10000
        y = (9876 + i * 5432) % 10000
        if x * x + y * y <= 100000000:
            inside += 1
    return inside

print("=== Pi Estimation (Monte Carlo) ===")

print("500,000 points...")
start = time.perf_counter()
result = estimate_pi(500000)
elapsed = time.perf_counter() - start
print(f"Result: {result}")
print(f"Time: {elapsed} s")

print("\n1,000,000 points...")
start = time.perf_counter()
result2 = estimate_pi(1000000)
elapsed = time.perf_counter() - start
print(f"Result: {result2}")
print(f"Time: {elapsed} s")
