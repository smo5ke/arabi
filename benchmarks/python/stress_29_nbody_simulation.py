# N-Body physics simulation
import time
import random

def update(bodies, n, dt):
    for i in range(n):
        fx = 0.0
        fy = 0.0
        for j in range(n):
            if i != j:
                dx = bodies[j][0] - bodies[i][0]
                dy = bodies[j][1] - bodies[i][1]
                dist2 = dx * dx + dy * dy + 0.01
                force = 100.0 / dist2
                fx += force * dx / dist2
                fy += force * dy / dist2
        bodies[i][2] += fx * dt
        bodies[i][3] += fy * dt
        bodies[i][0] += bodies[i][2] * dt
        bodies[i][1] += bodies[i][3] * dt

print("=== N-Body Simulation Stress Test ===")

n_bodies = 100
n_steps = 50
random.seed(42)
bodies = [[random.uniform(-500, 500), random.uniform(-500, 500),
           random.uniform(-5, 5), random.uniform(-5, 5)] for _ in range(n_bodies)]

t0 = time.time()
for step in range(n_steps):
    update(bodies, n_bodies, 0.01)
t1 = time.time()

total_x = sum(b[0] for b in bodies)
print(f"Bodies: {n_bodies}, Time: {t1 - t0:.3f}s, Sum X: {total_x:.2f}")
