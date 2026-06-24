import time

print("=== Nested Object Access Stress Test ===")

class Node:
    __slots__ = ('name', 'value', 'children', 'metadata')

    def __init__(self, name, value):
        self.name = name
        self.value = value
        self.children = []
        self.metadata = {}

def build_tree(depth, breadth, node_id):
    if depth == 0:
        return Node(f"leaf_{node_id}", node_id * 7)
    node = Node(f"node_{node_id}", node_id * 13)
    for i in range(breadth):
        child = build_tree(depth - 1, breadth, node_id * breadth + i + 1)
        node.children.append(child)
    node.metadata["depth"] = depth
    node.metadata["id"] = node_id
    return node

start = time.time()

root = build_tree(4, 4, 0)

ACCESS_COUNT = 500
total = 0

def access_tree(node, count):
    global total
    if count <= 0:
        return count
    total += node.value
    for key in node.metadata:
        total += node.metadata[key]
    remaining = count - 1
    for child in node.children:
        if remaining <= 0:
            return 0
        remaining = access_tree(child, remaining)
    return remaining

access_tree(root, ACCESS_COUNT)

elapsed = time.time() - start

print(f"Tree depth: 4, branching: 4")
print(f"Accesses: {ACCESS_COUNT}, Accumulated: {total}")
print(f"Time: {elapsed:.6f}s")
