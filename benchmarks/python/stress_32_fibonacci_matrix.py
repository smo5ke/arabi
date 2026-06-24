import time

print("=== Fibonacci Matrix Exponentiation Stress Test ===")

def mat_mul(a, b):
    return [
        [a[0][0] * b[0][0] + a[0][1] * b[1][0], a[0][0] * b[0][1] + a[0][1] * b[1][1]],
        [a[1][0] * b[0][0] + a[1][1] * b[1][0], a[1][0] * b[0][1] + a[1][1] * b[1][1]],
    ]

def mat_pow(m, p):
    result = [[1, 0], [0, 1]]
    while p > 0:
        if p % 2 == 1:
            result = mat_mul(result, m)
        m = mat_mul(m, m)
        p //= 2
    return result

N = 40

start = time.time()

base = [[1, 1], [1, 0]]
result = mat_pow(base, N)
fib_n = result[0][1]

elapsed = time.time() - start

print(f"Fibonacci({N}) = {fib_n}")
print(f"Time: {elapsed:.6f}s")
