import time
import sys
sys.stdout.reconfigure(encoding='utf-8')

start = time.perf_counter()

def is_valid(board, row, col):
    for r in range(row):
        if board[r] == col:
            return False
        dist_row = row - r
        dist_col = board[r] - col
        if dist_col < 0:
            dist_col = 0 - dist_col
        if dist_row == dist_col:
            return False
    return True

def solve(board, row, size):
    if row == size:
        return 1
    count = 0
    for col in range(size):
        if is_valid(board, row, col):
            board[row] = col
            result = solve(board, row + 1, size)
            count = count + result
    return count

board_size = 11
board = []
for i in range(board_size):
    board.append(0)

total = solve(board, 0, board_size)

end = time.perf_counter()
elapsed_ms = (end - start) * 1000

print(f"N-Queens 11: {total} solutions")
print(f"Time (ms): {elapsed_ms:.1f}")
