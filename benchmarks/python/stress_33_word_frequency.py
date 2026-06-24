import time

print("=== Word Frequency Stress Test ===")

words = ["the", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog",
         "a", "an", "is", "was", "are", "been", "be", "have", "has", "had",
         "do", "does", "did", "will", "would", "could", "should", "may", "might",
         "shall", "can", "need", "dare", "ought", "used", "to", "of", "in",
         "for", "on", "with", "at", "by", "from", "as", "into", "through",
         "during", "before", "after", "above", "below", "between", "out", "off"]

lines = []
for _ in range(5000):
    line = " ".join(words[i % len(words)] for i in range(0, len(words) * 3, 1))
    lines.append(line)

text = " ".join(lines)

start = time.time()

freq = {}
for word in text.split():
    if word in freq:
        freq[word] += 1
    else:
        freq[word] = 1

elapsed = time.time() - start

sorted_words = sorted(freq.items(), key=lambda x: x[1], reverse=True)

print(f"Total words: {sum(freq.values())}")
print(f"Unique words: {len(freq)}")
print(f"Top 5: {sorted_words[:5]}")
print(f"Time: {elapsed:.6f}s")
