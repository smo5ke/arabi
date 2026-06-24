import time

def step10(n): return step9(n + 1)
def step9(n): return step8(n + 1)
def step8(n): return step7(n + 1)
def step7(n): return step6(n + 1)
def step6(n): return step5(n + 1)
def step5(n): return step4(n + 1)
def step4(n): return step3(n + 1)
def step3(n): return step2(n + 1)
def step2(n): return step1(n + 1)
def step1(n): return n

print("=== Deep Call Chain ===")

print("10,000 calls x 10 deep...")
start = time.perf_counter()
for i in range(10000):
    result = step10(0)
elapsed = time.perf_counter() - start
print(f"Result: {result}")
print(f"Time: {elapsed} s")
