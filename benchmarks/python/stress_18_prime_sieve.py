import time

def prime_sieve(limit):
    primes = [1] * (limit + 1)
    primes[0] = 0
    if limit >= 1:
        primes[1] = 0
    factor = 2
    while factor * factor <= limit:
        if primes[factor] == 1:
            multiple = factor * factor
            while multiple <= limit:
                primes[multiple] = 0
                multiple += factor
        factor += 1
    count = 0
    for i in range(limit + 1):
        if primes[i] == 1:
            count += 1
    return count

print("=== Prime Sieve (Eratosthenes) ===")

print("Finding primes up to 100,000...")
start = time.perf_counter()
count1 = prime_sieve(100000)
elapsed = time.perf_counter() - start
print(f"Prime count: {count1}")
print(f"Time: {elapsed} s")

print("\nFinding primes up to 200,000...")
start = time.perf_counter()
count2 = prime_sieve(200000)
elapsed = time.perf_counter() - start
print(f"Prime count: {count2}")
print(f"Time: {elapsed} s")
