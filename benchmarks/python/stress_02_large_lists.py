import time

def bubble_sort(arr):
    n = len(arr)
    for i in range(n):
        for j in range(0, n - i - 1):
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
    return arr

print("=== Large List Stress Test ===")

print("Testing creating list of 200,000 elements...")
start = time.perf_counter()
lst = []
for i in range(200000):
    lst.append(i)
elapsed = time.perf_counter() - start
print(f"Elements: {len(lst)}")
print(f"Time: {elapsed} s")

print("\nTesting list access (50,000 times)...")
start = time.perf_counter()
for i in range(50000):
    x = lst[100000]
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting bubble sort (3000 elements)...")
lst2 = []
for i in range(3000):
    lst2.append(3000 - i)
start = time.perf_counter()
lst2 = bubble_sort(lst2)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting nested lists (1000x100)...")
start = time.perf_counter()
matrix = []
for i in range(1000):
    row = []
    for j in range(100):
        row.append(i * 100 + j)
    matrix.append(row)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
