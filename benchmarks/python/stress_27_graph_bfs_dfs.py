# Graph BFS + DFS stress test
import time

def create_graph(n):
    adj = [[] for _ in range(n)]
    for i in range(n - 1):
        adj[i].append(i + 1)
        adj[i + 1].append(i)
        if i + 3 < n:
            adj[i].append(i + 3)
            adj[i + 3].append(i)
    return adj

def bfs(adj, start):
    n = len(adj)
    visited = [0] * n
    queue = [start]
    visited[start] = 1
    count = 0
    while queue:
        node = queue.pop(0)
        count += 1
        for neighbor in adj[node]:
            if not visited[neighbor]:
                visited[neighbor] = 1
                queue.append(neighbor)
    return count

def dfs_iterative(adj, n):
    result = 0
    for start in range(n):
        if result > 1000:
            return result
        visited = [0] * n
        stack = [start]
        while stack:
            node = stack.pop()
            if not visited[node]:
                visited[node] = 1
                result += 1
                for j in range(len(adj[node]) - 1, -1, -1):
                    if not visited[adj[node][j]]:
                        stack.append(adj[node][j])
    return result

print("=== Graph Search Stress Test ===")

n = 5000
t0 = time.time()
adj = create_graph(n)
t1 = time.time()
print(f"Build: {t1 - t0:.3f}s")

t0 = time.time()
c1 = bfs(adj, 0)
t1 = time.time()
print(f"BFS: visited {c1} nodes, time: {t1 - t0:.3f}s")

t0 = time.time()
c2 = dfs_iterative(adj, n)
t1 = time.time()
print(f"DFS: visited {c2} nodes, time: {t1 - t0:.3f}s")
