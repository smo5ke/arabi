import time

def gcd(a, b):
    while b != 0:
        temp = b
        b = a % b
        a = temp
    return a

def factorial_fast(n):
    result = 1
    for i in range(2, n + 1):
        result *= i
    return result

def sum_squares(n):
    total = 0
    for i in range(n + 1):
        total += i * i
    return total

print("=== Negative Math ===")

print(f"gcd(48, 18) = {gcd(48, 18)}")
print(f"gcd(100, 75) = {gcd(100, 75)}")
print(f"gcd(1071, 462) = {gcd(1071, 462)}")

print(f"\nfactorial(20) = {factorial_fast(20)}")
print(f"factorial(15) = {factorial_fast(15)}")

print(f"\nsum_squares(100) = {sum_squares(100)}")
print(f"sum_squares(1000) = {sum_squares(1000)}")

print("\n100,000 GCD calls...")
start = time.perf_counter()
for i in range(100000):
    result = gcd(1000 + i, 500 + i)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
