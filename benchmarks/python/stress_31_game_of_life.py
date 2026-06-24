import time

print("=== Game of Life Stress Test ===")

SIZE = 100
GENS = 50

grid = [[False] * SIZE for _ in range(SIZE)]

for i in range(SIZE // 2 - 5, SIZE // 2 + 5):
    for j in range(SIZE // 2 - 5, SIZE // 2 + 5):
        grid[i][j] = True

def count_neighbors(g, r, c):
    count = 0
    for dr in (-1, 0, 1):
        for dc in (-1, 0, 1):
            if dr == 0 and dc == 0:
                continue
            nr, nc = r + dr, c + dc
            if 0 <= nr < SIZE and 0 <= nc < SIZE and g[nr][nc]:
                count += 1
    return count

start = time.time()

alive_counts = []
for gen in range(GENS):
    alive = sum(1 for r in range(SIZE) for c in range(SIZE) if grid[r][c])
    alive_counts.append(alive)

    new_grid = [[False] * SIZE for _ in range(SIZE)]
    for r in range(SIZE):
        for c in range(SIZE):
            neighbors = count_neighbors(grid, r, c)
            if grid[r][c]:
                new_grid[r][c] = neighbors == 2 or neighbors == 3
            else:
                new_grid[r][c] = neighbors == 3
    grid = new_grid

elapsed = time.time() - start

print(f"Grid: {SIZE}x{SIZE}, Generations: {GENS}")
print(f"Final alive: {alive_counts[-1]}")
print(f"Time: {elapsed:.6f}s")
