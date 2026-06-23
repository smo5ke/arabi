import sys
import io
import subprocess
import time
import os
import glob
from datetime import datetime

sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')

ARABI_CLI = 'target/release/arabi.exe'
BENCH_DIR = 'benchmarks'

def run_test(name, arabi_file, python_file):
    """Run a single test comparison between Arabi and Python"""
    results = {'name': name}
    
    # Run Arabi
    try:
        start = time.time()
        result = subprocess.run(
            [ARABI_CLI, 'run', arabi_file],
            capture_output=True, timeout=120,
            cwd=os.getcwd()
        )
        arabi_time = (time.time() - start) * 1000
        arabi_output = result.stdout.decode('utf-8', errors='replace').strip()
        arabi_ok = result.returncode == 0
        results['arabi_time'] = arabi_time
        results['arabi_output'] = arabi_output
        results['arabi_ok'] = arabi_ok
    except Exception as e:
        results['arabi_time'] = -1
        results['arabi_output'] = str(e)
        results['arabi_ok'] = False
    
    # Run Python
    try:
        start = time.time()
        result = subprocess.run(
            [sys.executable, python_file],
            capture_output=True, timeout=120,
            cwd=os.getcwd()
        )
        py_time = (time.time() - start) * 1000
        py_output = result.stdout.decode('utf-8', errors='replace').strip()
        py_ok = result.returncode == 0
        results['python_time'] = py_time
        results['python_output'] = py_output
        results['python_ok'] = py_ok
    except Exception as e:
        results['python_time'] = -1
        results['python_output'] = str(e)
        results['python_ok'] = False
    
    # Calculate ratio
    if results['arabi_ok'] and results['python_ok'] and results['arabi_time'] > 0 and results['python_time'] > 0:
        results['ratio'] = results['python_time'] / results['arabi_time']
        results['winner'] = 'Arabi' if results['ratio'] > 1 else 'Python'
    else:
        results['ratio'] = 0
        results['winner'] = 'Error'
    
    return results

def print_header():
    print("=" * 100)
    print("Arabi Stress Test Benchmark Comparison")
    print(f"Date: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 100)
    print(f"{'Test':<35} {'Arabi (ms)':<15} {'Python (ms)':<15} {'Ratio':<10} {'Winner':<10}")
    print("-" * 100)

def print_result(result):
    name = result['name']
    if result['arabi_ok'] and result['python_ok']:
        arabi_time = f"{result['arabi_time']:.1f}ms"
        python_time = f"{result['python_time']:.1f}ms"
        ratio = f"{result['ratio']:.2f}x"
        winner = result['winner']
        print(f"{name:<35} {arabi_time:<15} {python_time:<15} {ratio:<10} {winner:<10}")
    else:
        print(f"{name:<35} {'ERROR':<15} {'ERROR':<15} {'N/A':<10} {'N/A':<10}")

def print_summary(results):
    print("=" * 100)
    print("SUMMARY")
    print("=" * 100)
    
    valid_results = [r for r in results if r['winner'] in ['Arabi', 'Python']]
    arabi_wins = sum(1 for r in valid_results if r['winner'] == 'Arabi')
    python_wins = sum(1 for r in valid_results if r['winner'] == 'Python')
    
    if valid_results:
        avg_ratio = sum(r['ratio'] for r in valid_results) / len(valid_results)
        print(f"Total tests: {len(results)}")
        print(f"Valid comparisons: {len(valid_results)}")
        print(f"Arabi wins: {arabi_wins}/{len(valid_results)}")
        print(f"Python wins: {python_wins}/{len(valid_results)}")
        print(f"Average speed ratio: {avg_ratio:.2f}x")
        
        if avg_ratio > 1:
            print(f"\nArabi is ON AVERAGE {avg_ratio:.2f}x faster than Python!")
        else:
            print(f"\nPython is ON AVERAGE {1/avg_ratio:.2f}x faster than Arabi!")
    
    # Print detailed results
    print("\n" + "=" * 100)
    print("DETAILED RESULTS")
    print("=" * 100)
    
    for result in results:
        print(f"\n{result['name']}:")
        if result['arabi_ok']:
            print(f"  Arabi: {result['arabi_time']:.1f}ms")
            # Print first 3 lines of output
            lines = result['arabi_output'].split('\n')
            for line in lines[:3]:
                print(f"    {line}")
        else:
            print(f"  Arabi: ERROR - {result['arabi_output'][:100]}")
        
        if result['python_ok']:
            print(f"  Python: {result['python_time']:.1f}ms")
            # Print first 3 lines of output
            lines = result['python_output'].split('\n')
            for line in lines[:3]:
                print(f"    {line}")
        else:
            print(f"  Python: ERROR - {result['python_output'][:100]}")

def main():
    # Find all stress test pairs
    arabi_files = sorted(glob.glob(f'{BENCH_DIR}/arabi/stress_*.arabi'))
    
    if not arabi_files:
        print("No stress test files found!")
        return
    
    print_header()
    
    results = []
    for arabi_file in arabi_files:
        name = os.path.basename(arabi_file).replace('.arabi', '')
        python_file = f'{BENCH_DIR}/python/{name}.py'
        
        if not os.path.exists(python_file):
            print(f"{name:<35} {'SKIP (no Python file)':<15}")
            continue
        
        result = run_test(name, arabi_file, python_file)
        print_result(result)
        results.append(result)
    
    print_summary(results)

if __name__ == '__main__':
    main()
