import time


def newton_sqrt(x):
    guess = x / 2
    for i in range(1000):
        guess = (guess + x / guess) / 2
    return guess


def square(x):
    return x * x


def simpsons_rule(func, a, b, slices):
    width = (b - a) / slices
    total = func(a) + func(b)

    for i in range(slices - 1):
        point = a + width * (i + 1)
        if i % 2 == 0:
            total += 4 * func(point)
        else:
            total += 2 * func(point)

    return total * width / 3


print("=== Numerical Methods Stress Test ===")

start = time.time()

for x in range(100):
    result = newton_sqrt(x * 1000 + 1)

g10000 = newton_sqrt(10000)
print(f"sqrt(10000) = {g10000}")

g1000000 = newton_sqrt(1000000)
print(f"sqrt(1000000) = {g1000000}")

g2 = newton_sqrt(2)
print(f"sqrt(2) = {g2}")

g999999 = newton_sqrt(999999)
print(f"sqrt(999999) = {g999999}")

for i in range(100):
    result = newton_sqrt(i + 1)

i1 = simpsons_rule(square, 0, 1, 10000)
print(f"Integral of x^2 from 0 to 1 = {i1}")

i2 = simpsons_rule(square, 0, 2, 10000)
print(f"Integral of x^2 from 0 to 2 = {i2}")

i3 = simpsons_rule(square, -1, 1, 10000)
print(f"Integral of x^2 from -1 to 1 = {i3}")

for i in range(50):
    result = simpsons_rule(square, 0, 1, 100)

elapsed = time.time() - start

print("Completed numerical methods")
print(f"Time: {elapsed} s")
