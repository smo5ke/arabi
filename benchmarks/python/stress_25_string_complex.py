import time

def create_long_string(length):
    s = ""
    for i in range(length):
        s = s + "a"
    return s

def count_char(s, char):
    count = 0
    for i in range(len(s)):
        if s[i] == char:
            count += 1
    return count

def reverse_string(s):
    result = ""
    length = len(s)
    for i in range(length):
        result = result + s[length - 1 - i]
    return result

print("=== Complex String Operations ===")

print("Building 50,000 char string...")
start = time.perf_counter()
long_str = create_long_string(50000)
elapsed = time.perf_counter() - start
print(f"Length: {len(long_str)}")
print(f"Time: {elapsed} s")

print("\nReversing 50,000 char string...")
start = time.perf_counter()
reversed_str = reverse_string(long_str)
elapsed = time.perf_counter() - start
print(f"Reversed length: {len(reversed_str)}")
print(f"Time: {elapsed} s")

print("\nBuilding repeated string (100K x 'ab')...")
start = time.perf_counter()
s2 = ""
for i in range(100000):
    s2 = s2 + "ab"
elapsed = time.perf_counter() - start
print(f"Length: {len(s2)}")
print(f"Time: {elapsed} s")

print("\nConcatenating 10,000 strings...")
start = time.perf_counter()
merged = ""
for i in range(10000):
    merged = merged + "word"
elapsed = time.perf_counter() - start
print(f"Merged length: {len(merged)}")
print(f"Time: {elapsed} s")
