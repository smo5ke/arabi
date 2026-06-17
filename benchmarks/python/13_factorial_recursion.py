import time

def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

start = time.time()
for _ in range(50000):
    factorial(20)
end = time.time()
print(f"Factorial 20 x 50000: {end - start:.4f}")
