import time

def double(x):
    return x * 2

def square(x):
    return x * x

def add(a, b):
    return a + b

def is_less_than(x, threshold):
    return 1 if x < threshold else 0

def map_func(func, lst):
    result = []
    for x in lst:
        result.append(func(x))
    return result

def filter_func(cond, lst):
    result = []
    for x in lst:
        if cond(x) == 1:
            result.append(x)
    return result

def zip_with(func, lst1, lst2):
    result = []
    for x in lst1:
        result.append(func(x, lst2))
    return result

print("=== Functional Patterns ===")

lst = []
for i in range(5000):
    lst.append(i + 1)

print("Map (5000 elements x2)...")
start = time.perf_counter()
doubled = map_func(double, lst)
elapsed = time.perf_counter() - start
print(f"Last element: {doubled[4999]}")
print(f"Time: {elapsed} s")

print("\nMap square (5000 elements)...")
start = time.perf_counter()
squared = map_func(square, lst)
elapsed = time.perf_counter() - start
print(f"Last element: {squared[4999]}")
print(f"Time: {elapsed} s")

print("\nFilter (5000 elements < 2500)...")
start = time.perf_counter()
filtered = list(filter(lambda x: x < 2500, lst))
elapsed = time.perf_counter() - start
print(f"Filtered count: {len(filtered)}")
print(f"Time: {elapsed} s")

print("\nZip-with (5000 elements)...")
start = time.perf_counter()
zipped = [add(a, b) for a, b in zip(doubled, squared)]
elapsed = time.perf_counter() - start
print(f"Result count: {len(zipped)}")
print(f"Last element: {zipped[4999]}")
print(f"Time: {elapsed} s")
