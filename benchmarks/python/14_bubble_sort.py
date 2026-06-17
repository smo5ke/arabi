import time

def bubble_sort(lst):
    n = len(lst)
    for i in range(n - 1):
        for j in range(n - 1 - i):
            if lst[j] > lst[j + 1]:
                lst[j], lst[j + 1] = lst[j + 1], lst[j]
    return lst

start = time.time()
for _ in range(500):
    lst = list(range(50, 0, -1))
    bubble_sort(lst)
end = time.time()
print(f"Bubble Sort 50 elements x 500: {end - start:.4f}")
