import time

def make_counter(start):
    count = start
    def inc():
        nonlocal count
        count += 1
        return count
    def dec():
        nonlocal count
        count -= 1
        return count
    return inc, dec

def make_repeater(factor):
    def repeater(n):
        result = n
        for i in range(factor):
            result += n
        return result
    return repeater

def make_layer(num_layers):
    if num_layers == 0:
        return 0
    count = [0]
    def layer():
        count[0] += num_layers
        return count[0]
    return layer

print("=== Deep Closures ===")

print("100,000 counter create+call...")
start = time.perf_counter()
for i in range(100000):
    c = make_counter(i)
    c[0]()
    c[1]()
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\n100,000 repeater create+call...")
start = time.perf_counter()
for i in range(100000):
    r = make_repeater(5)
    r(i)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\n100,000 layer create+call...")
start = time.perf_counter()
for i in range(100000):
    layer = make_layer(5)
    layer()
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
