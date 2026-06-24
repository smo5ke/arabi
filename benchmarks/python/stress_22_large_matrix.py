import time

def build_matrix(size):
    matrix = []
    for i in range(size):
        row = []
        for j in range(size):
            if i == j:
                row.append(1)
            elif i > j:
                row.append(2)
            else:
                row.append(0)
        matrix.append(row)
    return matrix

def transpose_matrix(matrix, size):
    result = []
    for j in range(size):
        row = []
        for i in range(size):
            row.append(matrix[i][j])
        result.append(row)
    return result

print("=== Large Matrix 500x500 ===")

start = time.perf_counter()
m = build_matrix(500)
elapsed = time.perf_counter() - start
print(f"Build: {elapsed} s")
print(f"[0][0]: {m[0][0]}, [499][499]: {m[499][499]}, [100][200]: {m[100][200]}")

start = time.perf_counter()
m2 = transpose_matrix(m, 500)
elapsed = time.perf_counter() - start
print(f"Transpose: {elapsed} s")
print(f"[200][100] after transpose: {m2[200][100]}")
