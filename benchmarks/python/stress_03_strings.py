import time

print("=== String Operations Stress Test ===")

print("Testing building large string (10,000 chars)...")
start = time.perf_counter()
s = ""
for i in range(10000):
    s = s + "hello"
elapsed = time.perf_counter() - start
print(f"String length: {len(s)}")
print(f"Time: {elapsed} s")

print("\nTesting multiple string operations (50,000 times)...")
start = time.perf_counter()
for i in range(50000):
    s2 = "  Hello World  "
    s2 = s2.strip()
    s2 = s2.upper()
    s2 = s2.lower()
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting string concatenation (50,000 times)...")
start = time.perf_counter()
for i in range(50000):
    result = "line" + " " + "number" + " " + "1"
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
