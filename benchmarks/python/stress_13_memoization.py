import time

def make_memo():
    return [-1] * 100

def fibonacci_memo(n, memo):
    if memo[n] != -1:
        return memo[n]
    if n <= 1:
        memo[n] = n
        return n
    memo[n] = fibonacci_memo(n - 1, memo) + fibonacci_memo(n - 2, memo)
    return memo[n]

print("=== Memoization ===")

print("fibonacci(90) with memoization...")
memo = make_memo()
start = time.perf_counter()
result = fibonacci_memo(90, memo)
elapsed = time.perf_counter() - start
print(f"Result: {result}")
print(f"Time: {elapsed} s")

print("\nRecompute 100 times...")
start = time.perf_counter()
for i in range(100):
    memo2 = make_memo()
    result2 = fibonacci_memo(90, memo2)
elapsed = time.perf_counter() - start
print(f"Result: {result2}")
print(f"Time: {elapsed} s")
