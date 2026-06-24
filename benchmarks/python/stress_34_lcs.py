import time

print("=== Longest Common Subsequence Stress Test ===")

import random
random.seed(42)

N = 1000
seq1 = [random.randint(0, 26) for _ in range(N)]
seq2 = [random.randint(0, 26) for _ in range(N)]

start = time.time()

dp = [[0] * (N + 1) for _ in range(N + 1)]

for i in range(1, N + 1):
    for j in range(1, N + 1):
        if seq1[i - 1] == seq2[j - 1]:
            dp[i][j] = dp[i - 1][j - 1] + 1
        else:
            dp[i][j] = max(dp[i - 1][j], dp[i][j - 1])

lcs_length = dp[N][N]

elapsed = time.time() - start

print(f"Sequence lengths: {N}, {N}")
print(f"LCS length: {lcs_length}")
print(f"Time: {elapsed:.6f}s")
