import time

def transition(state, inp):
    if state == 0:
        return 1 if inp == 1 else 0
    if state == 1:
        return 2 if inp == 0 else 1
    if state == 2:
        return 3 if inp == 1 else 0
    if state == 3:
        return 4 if inp == 1 else 2
    if state == 4:
        return 5 if inp == 0 else 3
    return state

print("=== State Machine ===")

print("Running 100,000 states...")
inputs = []
for i in range(100000):
    if i % 3 == 0:
        inputs.append(1)
    else:
        inputs.append(0)
start = time.perf_counter()
state = 0
for inp in inputs:
    state = transition(state, inp)
elapsed = time.perf_counter() - start
print(f"Final state: {state}")
print(f"Time: {elapsed} s")

print("\nRunning 500,000 states with exception handling...")
start = time.perf_counter()
state = 0
for i in range(500000):
    try:
        state = transition(state, 1)
    except Exception:
        state = 0
elapsed = time.perf_counter() - start
print(f"Final state: {state}")
print(f"Time: {elapsed} s")
