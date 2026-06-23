import time

print("=== Dictionary Stress Test ===")

print("Testing creating large dictionary (100,000 elements)...")
start = time.perf_counter()
d = {}
for i in range(100000):
    d[str(i)] = i
elapsed = time.perf_counter() - start
print(f"Elements: {len(d)}")
print(f"Time: {elapsed} s")

print("\nTesting dictionary search (50,000 times)...")
start = time.perf_counter()
for i in range(50000):
    x = d["50000"]
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting dictionary update (50,000 times)...")
start = time.perf_counter()
for i in range(50000):
    d["0"] = i
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting nested dictionaries...")
start = time.perf_counter()
dicts = []
for i in range(1000):
    d = {}
    for j in range(100):
        d[str(j)] = j * 3
    dicts.append(d)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
