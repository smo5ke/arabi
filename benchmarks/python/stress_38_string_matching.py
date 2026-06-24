import time


def build_text(length):
    text = ""
    chars = "abcdefghijklmnopqrstuvwxyz"
    for i in range(length):
        text += chars[i % 26]
    return text


def naive_search(text, pattern):
    count = 0
    text_len = len(text)
    pattern_len = len(pattern)

    if pattern_len > text_len:
        return 0

    for i in range(text_len - pattern_len + 1):
        found = True
        for j in range(pattern_len):
            if text[i + j] != pattern[j]:
                found = False
                break
        if found:
            count += 1
    return count


print("=== String Matching Stress Test ===")

start = time.time()

big_text = build_text(5000)
pattern1 = "abcdefghij"
pattern2 = "mnopqrstuvwx"
pattern3 = "abcdefghijklmno"

result1 = naive_search(big_text, pattern1)
print(f"Pattern 1: {pattern1} found {result1} times")

result2 = naive_search(big_text, pattern2)
print(f"Pattern 2: {pattern2} found {result2} times")

result3 = naive_search(big_text, pattern3)
print(f"Pattern 3: {pattern3} found {result3} times")

for i in range(100):
    result = naive_search(big_text, pattern1)

repeated_text = ""
for i in range(100):
    repeated_text += "abcdefghijklmnopqrstuvwxyz"

result4 = naive_search(repeated_text, "abcde")
print(f"Repeated text: found {result4} times")

for i in range(20):
    result = naive_search(repeated_text, "abcdefghijklmno")

elapsed = time.time() - start

print("Completed string matching")
print(f"Time: {elapsed} s")
