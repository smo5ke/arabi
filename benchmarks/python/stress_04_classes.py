import time

class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y

    def area(self):
        return self.x * self.y

    def move(self, dx, dy):
        self.x += dx
        self.y += dy

class Rectangle:
    def __init__(self, width, height):
        self.width = width
        self.height = height

    def area_rect(self):
        return self.width * self.height

    def perimeter(self):
        return 2 * (self.width + self.height)

print("=== Class/Object Stress Test ===")

print("Testing creating 50,000 points...")
start = time.perf_counter()
points = []
for i in range(50000):
    p = Point(i, i * 2)
    points.append(p)
elapsed = time.perf_counter() - start
print(f"Points: {len(points)}")
print(f"Time: {elapsed} s")

print("\nTesting method calls (50,000 times)...")
start = time.perf_counter()
for p in points:
    area = p.area()
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting move (50,000 times)...")
start = time.perf_counter()
for p in points:
    p.move(1, 1)
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")

print("\nTesting rectangles (50,000 times)...")
start = time.perf_counter()
rects = []
for i in range(50000):
    r = Rectangle(i + 1, i + 2)
    rects.append(r)
elapsed = time.perf_counter() - start
print(f"Rectangles: {len(rects)}")
print(f"Time: {elapsed} s")
