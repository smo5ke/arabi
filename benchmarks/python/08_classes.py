class Point:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    def area(self):
        return self.x + self.y
for i in range(100000):
    p = Point(i, i*2)
    p.area()
print("done")
