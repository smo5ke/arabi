import sys, io, subprocess, time, os, glob
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

ARABI_CLI = 'target/release/arabi.exe'
BENCH_DIR = 'benchmarks'

results = []

# Find all Arabi benchmark files
arabi_files = sorted(glob.glob(f'{BENCH_DIR}/arabi/*.arabi'))

print('=' * 70)
print(f'{"Benchmark":<25} {"Arabi (ms)":<15} {"Python (ms)":<15} {"Ratio":<10} {"Winner":<10}')
print('=' * 70)

for arabi_file in arabi_files:
    name = os.path.basename(arabi_file).replace('.arabi', '')
    py_file = f'{BENCH_DIR}/python/{name}.py'
    
    if not os.path.exists(py_file):
        print(f'{name:<25} {"SKIP (no Python file)":<15}')
        continue
    
    # Run Arabi
    try:
        start = time.time()
        result = subprocess.run(
            [ARABI_CLI, 'run', arabi_file],
            capture_output=True, timeout=60,
            cwd=os.getcwd()
        )
        arabi_time = (time.time() - start) * 1000
        arabi_output = result.stdout.decode('utf-8', errors='replace').strip()
        arabi_ok = result.returncode == 0
    except Exception as e:
        arabi_time = -1
        arabi_output = str(e)
        arabi_ok = False
    
    # Run Python
    try:
        start = time.time()
        result = subprocess.run(
            [sys.executable, py_file],
            capture_output=True, timeout=60,
            cwd=os.getcwd()
        )
        py_time = (time.time() - start) * 1000
        py_output = result.stdout.decode('utf-8', errors='replace').strip()
        py_ok = result.returncode == 0
    except Exception as e:
        py_time = -1
        py_output = str(e)
        py_ok = False
    
    if arabi_ok and py_ok and arabi_time > 0 and py_time > 0:
        ratio = py_time / arabi_time
        if ratio > 1:
            winner = 'Arabi'
        else:
            winner = 'Python'
        print(f'{name:<25} {arabi_time:>8.1f}ms     {py_time:>8.1f}ms     {ratio:>5.2f}x    {winner}')
        results.append((name, arabi_time, py_time, ratio, winner))
    elif not arabi_ok:
        print(f'{name:<25} {"FAIL":<15} {py_time:>8.1f}ms')
    elif not py_ok:
        print(f'{name:<25} {arabi_time:>8.1f}ms     {"FAIL":<15}')

print('=' * 70)

if results:
    arabi_wins = sum(1 for r in results if r[4] == 'Arabi')
    python_wins = sum(1 for r in results if r[4] == 'Python')
    avg_ratio = sum(r[3] for r in results) / len(results)
    print(f'\nSummary: Arabi wins {arabi_wins}/{len(results)}, Python wins {python_wins}/{len(results)}')
    print(f'Average speed ratio: {avg_ratio:.2f}x')
    if avg_ratio > 1:
        print(f'Arabi is ON AVERAGE {avg_ratio:.2f}x faster than Python!')
    else:
        print(f'Python is ON AVERAGE {1/avg_ratio:.2f}x faster than Arabi')
