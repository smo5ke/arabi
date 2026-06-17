import time

text = "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10"

start = time.time()
for _ in range(10000):
    t1 = text.upper()
    t2 = t1.lower()
    t3 = t2.strip()
    t4 = t3.replace("word1", "done")
    t5 = t4.split(" ")
    t6 = len(t5)
end = time.time()
print(f"String Operations x 10000: {end - start:.4f}")
