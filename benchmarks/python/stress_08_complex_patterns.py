import time

def compute_matrix(rows, cols):
    matrix = []
    for i in range(rows):
        row = []
        for j in range(cols):
            row.append(i * cols + j)
        matrix.append(row)
    return matrix

def quicksort(arr):
    if len(arr) <= 1:
        return arr
    pivot = arr[len(arr) // 2]
    left = [x for x in arr if x < pivot]
    middle = [x for x in arr if x == pivot]
    right = [x for x in arr if x > pivot]
    return quicksort(left) + middle + quicksort(right)

print("=== Complex Patterns Stress Test ===")

print("Testing matrix computation 200x200...")
start = time.perf_counter()
m1 = compute_matrix(200, 200)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting quicksort (20,000 elements)...")
start = time.perf_counter()
arr = []
for i in range(20000):
    arr.append(20000 - i)
sorted_arr = quicksort(arr)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting multiple list operations...")
start = time.perf_counter()
arr = []
for i in range(20000):
    arr.append(i)
arr2 = [x * 2 for x in arr if x % 2 == 0]
squares = []
for x in arr2:
    squares.append(x * x)
elapsed = time.perf_counter() - start
print(f"Elements: {len(squares)}")
print(f"Time: {elapsed} s")
