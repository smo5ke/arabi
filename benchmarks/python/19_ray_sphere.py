import time

print("--- 3D Ray-Sphere Intersection Benchmark ---")
start = time.time()


class Sphere:
    def __init__(self, x, y, z, radius):
        self.x = x
        self.y = y
        self.z = z
        self.radius = radius


class Ray:
    def __init__(self, ox, oy, oz, dx, dy, dz):
        self.ox = ox
        self.oy = oy
        self.oz = oz
        self.dx = dx
        self.dy = dy
        self.dz = dz


def intersects(ray, sphere):
    vx = ray.ox - sphere.x
    vy = ray.oy - sphere.y
    vz = ray.oz - sphere.z

    b = 2.0 * (vx * ray.dx + vy * ray.dy + vz * ray.dz)
    c = (vx * vx + vy * vy + vz * vz) - (sphere.radius * sphere.radius)

    discriminant = (b * b) - (4.0 * c)
    if discriminant > 0.0:
        return True
    else:
        return False


spheres = []
spheres.append(Sphere(0.0, 0.0, 10.0, 2.0))
spheres.append(Sphere(5.0, -5.0, 15.0, 3.0))
spheres.append(Sphere(-5.0, 5.0, 15.0, 3.0))

total_intersections = 0
screen_size = 500

for x in range(screen_size):
    for y in range(screen_size):
        fx = (x - 250.0) / 250.0
        fy = (y - 250.0) / 250.0

        length = 1.0 + (fx * fx) + (fy * fy)
        dx = fx / length
        dy = fy / length
        dz = 1.0 / length

        r = Ray(0.0, 0.0, 0.0, dx, dy, dz)

        for s in spheres:
            if intersects(r, s):
                total_intersections += 1

end = time.time()
elapsed = (end - start) * 1000

print("Total intersections:", total_intersections)
print("Time (ms):", round(elapsed, 1))
