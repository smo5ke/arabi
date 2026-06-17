def make_counter(start):
    count = start
    def inc():
        nonlocal count
        count += 1
        return count
    return inc
c = make_counter(0)
for i in range(100000):
    c()
print(c())
