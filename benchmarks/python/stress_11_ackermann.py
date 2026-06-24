import time

def ackermann(m, n):
    if m == 0:
        return n + 1
    if n == 0:
        return ackermann(m - 1, 1)
    return ackermann(m - 1, ackermann(m, n - 1))

print("=== Ackermann Function ===")

print("ackermann(2, 20)...")
start = time.perf_counter()
r1 = ackermann(2, 20)
elapsed = time.perf_counter() - start
print(f"Result: {r1}")
print(f"Time: {elapsed} s")

print("\nackermann(3, 4)...")
start = time.perf_counter()
r2 = ackermann(3, 4)
elapsed = time.perf_counter() - start
print(f"Result: {r2}")
print(f"Time: {elapsed} s")

print("\nackermann(3, 5)...")
start = time.perf_counter()
r3 = ackermann(3, 5)
elapsed = time.perf_counter() - start
print(f"Result: {r3}")
print(f"Time: {elapsed} s")

print("\nackermann(3, 6)...")
start = time.perf_counter()
r4 = ackermann(3, 6)
elapsed = time.perf_counter() - start
print(f"Result: {r4}")
print(f"Time: {elapsed} s")
