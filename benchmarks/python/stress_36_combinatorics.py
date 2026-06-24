import time


def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)


def combinations(n, k):
    if k == 0 or k == n:
        return 1
    if k == 1:
        return n
    return combinations(n - 1, k - 1) + combinations(n - 1, k)


def permutations(n, k):
    if k == 0:
        return 1
    return n * permutations(n - 1, k - 1)


print("=== Combinatorics Stress Test ===")

start = time.time()

factorial20 = factorial(20)
print(f"20! = {factorial20}")

factorial10 = factorial(10)
print(f"10! = {factorial10}")

factorial15 = factorial(15)
print(f"15! = {factorial15}")

c30_15 = combinations(30, 15)
print(f"C(30,15) = {c30_15}")

c20_10 = combinations(20, 10)
print(f"C(20,10) = {c20_10}")

c25_5 = combinations(25, 5)
print(f"C(25,5) = {c25_5}")

p20_5 = permutations(20, 5)
print(f"P(20,5) = {p20_5}")

p15_3 = permutations(15, 3)
print(f"P(15,3) = {p15_3}")

for i in range(20):
    c = combinations(i, i // 2)

elapsed = time.time() - start

print("Completed combinatorics and permutations")
print(f"Time: {elapsed} s")
