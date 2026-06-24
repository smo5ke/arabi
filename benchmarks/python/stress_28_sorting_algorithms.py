# Sorting algorithms stress test
import time
import random

def bubble_sort(arr):
    n = len(arr)
    for i in range(n - 1):
        for j in range(n - 1 - i):
            if arr[j] > arr[j + 1]:
                arr[j], arr[j + 1] = arr[j + 1], arr[j]
    return arr

def merge_sort(arr):
    if len(arr) <= 1:
        return arr
    mid = len(arr) // 2
    left = merge_sort(arr[:mid])
    right = merge_sort(arr[mid:])
    result = []
    i = j = 0
    while i < len(left) and j < len(right):
        if left[i] <= right[j]:
            result.append(left[i])
            i += 1
        else:
            result.append(right[j])
            j += 1
    result.extend(left[i:])
    result.extend(right[j:])
    return result

def quicksort(arr):
    if len(arr) <= 1:
        return arr
    pivot = arr[0]
    smaller = [x for x in arr[1:] if x < pivot]
    larger = [x for x in arr[1:] if x >= pivot]
    return quicksort(smaller) + [pivot] + quicksort(larger)

print("=== Sorting Algorithms Stress Test ===")

n = 2000
random.seed(42)
data = [random.randint(0, 9999) for _ in range(n)]

t0 = time.time()
r1 = merge_sort(data[:])
t1 = time.time()
print(f"Merge sort: {len(r1)} elements, {t1 - t0:.3f}s")

t0 = time.time()
r2 = quicksort(data[:])
t1 = time.time()
print(f"Quicksort: {len(r2)} elements, {t1 - t0:.3f}s")

t0 = time.time()
r3 = bubble_sort(data[:])
t1 = time.time()
print(f"Bubble sort: {len(r3)} elements, {t1 - t0:.3f}s")
