import time

class Shape:
    def __init__(self, name):
        self.name = name
    def area(self):
        return 0

class Rectangle(Shape):
    def __init__(self, width, height):
        super().__init__("rectangle")
        self.width = width
        self.height = height
    def area(self):
        return self.width * self.height

class Circle(Shape):
    def __init__(self, radius):
        super().__init__("circle")
        self.radius = radius
    def area(self):
        return 3 * self.radius * self.radius

class Triangle(Shape):
    def __init__(self, base, height):
        super().__init__("triangle")
        self.base = base
        self.height = height
    def area(self):
        return self.base * self.height // 2

print("=== Polymorphism ===")

print("Creating 100,000 shapes...")
start = time.perf_counter()
shapes = []
for i in range(100000):
    if i % 3 == 0:
        shapes.append(Rectangle(i + 1, i + 2))
    elif i % 3 == 1:
        shapes.append(Circle(i + 1))
    else:
        shapes.append(Triangle(i + 1, i + 2))
elapsed = time.perf_counter() - start
print(f"Count: {len(shapes)}")
print(f"Time: {elapsed} s")

print("\nCalculating area of 100,000 shapes...")
start = time.perf_counter()
for shape in shapes:
    a = shape.area()
elapsed = time.perf_counter() - start
print(f"Time: {elapsed} s")
