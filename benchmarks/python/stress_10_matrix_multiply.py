import time

def create_matrix(size):
    matrix = []
    for i in range(size):
        row = []
        for j in range(size):
            row.append((i * 7 + j * 13 + 1) % 100)
        matrix.append(row)
    return matrix

def multiply_matrices(a, b, size):
    result = []
    for i in range(size):
        row = []
        for j in range(size):
            total = 0
            for k in range(size):
                total += a[i][k] * b[k][j]
            row.append(total)
        result.append(row)
    return result

print("=== Matrix Multiplication (100x100) ===")

print("Creating and multiplying two 100x100 matrices...")
start = time.perf_counter()
a = create_matrix(100)
b = create_matrix(100)
result = multiply_matrices(a, b, 100)
elapsed = time.perf_counter() - start
print(f"Result matrix size: {len(result)}")
print(f"Element [0][0]: {result[0][0]}")
print(f"Element [50][50]: {result[50][50]}")
print(f"Time: {elapsed} s")
