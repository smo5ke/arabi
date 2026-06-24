import time

def test_exceptions(n):
    attempts = 0
    for i in range(n):
        try:
            if i % 3 == 0:
                s = 1 / 0
            elif i % 3 == 1:
                s = 1 / 0
            else:
                result = i * 2
        except ZeroDivisionError:
            attempts += 1
    return attempts

print("=== Complex Exception Patterns ===")

print("50,000 exceptions with 3 types...")
start = time.perf_counter()
attempts = test_exceptions(50000)
elapsed = time.perf_counter() - start
print(f"Attempts: {attempts}")
print(f"Time: {elapsed} s")

print("\nNested exceptions depth 3 (10,000 times)...")
start = time.perf_counter()
for i in range(10000):
    try:
        try:
            try:
                s = 1 / 0
            except ZeroDivisionError:
                x = 1
        except Exception:
            x = 2
    except Exception:
        x = 3
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nException with finally (50,000 times)...")
start = time.perf_counter()
for i in range(50000):
    try:
        if i % 2 == 0:
            s = 1 / 0
    except ZeroDivisionError:
        result = i
    finally:
        result = i + 1
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
