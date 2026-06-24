import time

class Node:
    def __init__(self, value):
        self.value = value
        self.left = None
        self.right = None
        self.is_leaf = True

    def sum_tree(self):
        if self.is_leaf:
            return self.value
        return self.value + self.left.sum_tree() + self.right.sum_tree()

    def depth(self):
        if self.is_leaf:
            return 1
        left_d = self.left.depth()
        right_d = self.right.depth()
        return max(left_d, right_d) + 1

    def node_count(self):
        if self.is_leaf:
            return 1
        return 1 + self.left.node_count() + self.right.node_count()

print("=== Object Tree Hierarchy ===")

print("Building binary tree depth 12...")
start = time.perf_counter()
root = Node(1)
current = [root]
for level in range(12):
    next_level = []
    for node in current:
        left = Node(node.value * 2)
        right = Node(node.value * 2 + 1)
        node.left = left
        node.right = right
        node.is_leaf = False
        next_level.append(left)
        next_level.append(right)
    current = next_level
elapsed = time.perf_counter() - start
print(f"Leaf nodes: {len(current)}")
print(f"Build time: {elapsed} s")

start = time.perf_counter()
total = root.sum_tree()
elapsed = time.perf_counter() - start
print(f"Sum: {total}")
print(f"Sum time: {elapsed} s")

start = time.perf_counter()
count = root.node_count()
elapsed = time.perf_counter() - start
print(f"Node count: {count}")
print(f"Count time: {elapsed} s")

start = time.perf_counter()
d = root.depth()
elapsed = time.perf_counter() - start
print(f"Depth: {d}")
print(f"Depth time: {elapsed} s")
