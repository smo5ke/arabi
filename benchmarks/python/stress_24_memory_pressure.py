import time

class LightObj:
    def __init__(self, value):
        self.value = value

def create_batch(n):
    batch = []
    for i in range(n):
        batch.append(LightObj(i))
    return batch

def sum_values(batch):
    total = 0
    for obj in batch:
        total += obj.value
    return total

print("=== Memory Pressure ===")

print("Creating 200,000 light objects...")
start = time.perf_counter()
batch1 = create_batch(200000)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nSumming 200,000 object values...")
start = time.perf_counter()
total1 = sum_values(batch1)
elapsed = time.perf_counter() - start
print(f"Sum: {total1}")
print(f"Time: {elapsed} s")

print("\nRecreate 5 times...")
start = time.perf_counter()
for i in range(5):
    batch = create_batch(200000)
    total = sum_values(batch)
elapsed = time.perf_counter() - start
print(f"Sum: {total}")
print(f"Time: {elapsed} s")
