import time

print("--- Mandelbrot Benchmark ---")
start = time.time()

width = 60
height = 40
max_iter = 1000

result = []

for y in range(height):
    cy = (y / 20.0) - 1.0
    line = ""
    for x in range(width):
        cx = (x / 20.0) - 2.0
        
        zr = 0.0
        zi = 0.0
        iteration = 0
        
        while (zr * zr + zi * zi <= 4.0) and (iteration < max_iter):
            tmp = zr * zr - zi * zi + cx
            zi = 2.0 * zr * zi + cy
            zr = tmp
            iteration += 1
        
        if iteration == max_iter:
            line += "#"
        else:
            line += " "
    
    result.append(line)

end = time.time()
elapsed = (end - start) * 1000

print("Generated successfully.")
print("Time (ms):", round(elapsed, 1))
