import time

print("=== Matrix Chain Multiplication Stress Test ===")

dims = [10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 110]
n = len(dims) - 1

start = time.time()

dp = [[0] * n for _ in range(n)]

for length in range(2, n + 1):
    for i in range(n - length + 1):
        j = i + length - 1
        dp[i][j] = float('inf')
        for k in range(i, j):
            cost = dp[i][k] + dp[k + 1][j] + dims[i] * dims[k + 1] * dims[j + 1]
            if cost < dp[i][j]:
                dp[i][j] = cost

elapsed = time.time() - start

print(f"Matrices: {n}")
print(f"Optimal cost: {dp[0][n - 1]}")
print(f"Time: {elapsed:.6f}s")
