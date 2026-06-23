import time

print("--- Conway's Game of Life Benchmark ---")
start = time.time()

size = 40
generations = 50


def create_board(s):
    return [[0] * s for _ in range(s)]


board = create_board(size)

# Initialize scattered pattern
for i in range(size):
    board[i][(i * 7) % size] = 1
    board[(i * 3) % size][i] = 1


def count_neighbors(b, row, col, s):
    neighbors = 0
    for tr in range(3):
        for tc in range(3):
            dr = tr - 1
            dc = tc - 1
            if dr == 0 and dc == 0:
                continue
            r = (row + dr + s) % s
            c = (col + dc + s) % s
            neighbors += b[r][c]
    return neighbors


for gen in range(generations):
    new_board = create_board(size)
    for row in range(size):
        for col in range(size):
            neighbors = count_neighbors(board, row, col, size)
            state = board[row][col]

            if state == 1:
                if neighbors == 2 or neighbors == 3:
                    new_board[row][col] = 1
            else:
                if neighbors == 3:
                    new_board[row][col] = 1

    board = new_board

total_alive = 0
for row in range(size):
    for col in range(size):
        total_alive += board[row][col]

end = time.time()
elapsed = (end - start) * 1000

print("Alive cells after", generations, "generations:", total_alive)
print("Time (ms):", round(elapsed, 1))
