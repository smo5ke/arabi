import time

def process_string(s):
    result = s + "|"
    for i in range(10):
        result = result + "---"
    result = result + "|"
    return result

def build_table(rows, cols):
    table = ""
    for i in range(rows):
        row = ""
        for j in range(cols):
            row = row + "x"
        table = table + row + "\n"
    return table

def reverse_list(lst):
    result = []
    for i in range(len(lst) - 1, -1, -1):
        result.append(lst[i])
    return result

def reverse_words(s):
    words = s.split(" ")
    return reverse_list(words)

print("=== String Pipeline ===")

print("Building long string (10,000 times)...")
start = time.perf_counter()
for i in range(10000):
    s = process_string("test")
elapsed = time.perf_counter() - start
print(f"String length: {len(s)}")
print(f"Time: {elapsed} s")

print("\nBuilding table 50x50...")
start = time.perf_counter()
table = build_table(50, 50)
elapsed = time.perf_counter() - start
print(f"Table length: {len(table)}")
print(f"Time: {elapsed} s")

print("\nReversing words 10,000 times...")
start = time.perf_counter()
for i in range(10000):
    words = reverse_words("hello world this is an arabic language test")
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
