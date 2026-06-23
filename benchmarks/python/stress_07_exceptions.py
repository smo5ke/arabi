import time

print("=== Exception Handling Stress Test ===")

print("Testing 50,000 exceptions (division by zero)...")
start = time.perf_counter()
for i in range(50000):
    try:
        s = 1 / 0
    except ZeroDivisionError:
        error = "done"
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting conditional exceptions (50,000 times)...")
start = time.perf_counter()
for i in range(50000):
    try:
        if i == 25000:
            s = 1 / 0
    except ZeroDivisionError:
        result = 0
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting nested exceptions (5000 times)...")
start = time.perf_counter()
result = 0
for i in range(5000):
    try:
        try:
            s = 1 / 0
        except ZeroDivisionError:
            result = 1
    except Exception:
        result = 2
elapsed = time.perf_counter() - start
print(f"Result: {result}")
print(f"Time: {elapsed} s")

print("\nTesting exceptions with finally (5000 times)...")
start = time.perf_counter()
for i in range(5000):
    try:
        s = 1 / 0
    except ZeroDivisionError:
        result = 0
    finally:
        result = 1
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
