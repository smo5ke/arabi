import time
import sys
sys.stdout.reconfigure(encoding='utf-8')

start = time.perf_counter()

class Node:
    def __init__(self, value):
        self.value = value
        self.left = None
        self.right = None

    def insert(self, new_value):
        if new_value < self.value:
            if self.left is None:
                self.left = Node(new_value)
            else:
                self.left.insert(new_value)
        elif new_value > self.value:
            if self.right is None:
                self.right = Node(new_value)
            else:
                self.right.insert(new_value)

    def search(self, target):
        if target == self.value:
            return True
        elif target < self.value:
            if self.left is not None:
                return self.left.search(target)
            else:
                return False
        else:
            if self.right is not None:
                return self.right.search(target)
            else:
                return False

root = Node(5000)
data_size = 10000

for i in range(1, data_size):
    num = (i * 3141) % 10000
    root.insert(num)

success = 0
for i in range(1, data_size):
    search_num = (i * 3141) % 10000
    if root.search(search_num):
        success += 1

end = time.perf_counter()
elapsed_ms = (end - start) * 1000

print(f"BST: inserted and searched {data_size - 1} nodes")
print(f"Successful searches: {success}")
print(f"Time (ms): {elapsed_ms:.1f}")
