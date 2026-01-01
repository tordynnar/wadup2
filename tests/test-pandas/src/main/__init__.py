"""Test pandas import in WASI."""
import sys
sys.stdout.write("Starting test...\n")
sys.stdout.flush()

print(f"Python version: {sys.version}", flush=True)
print("Importing pandas...", flush=True)

try:
    import pandas as pd
    print(f"Pandas version: {pd.__version__}", flush=True)
    print("SUCCESS: Pandas imported!", flush=True)

    # Try a simple operation
    df = pd.DataFrame({'a': [1, 2, 3], 'b': [4, 5, 6]})
    print(f"Created DataFrame with shape: {df.shape}", flush=True)
    print(df, flush=True)
except Exception as e:
    sys.stdout.flush()
    sys.stderr.flush()
    print(f"ERROR: {type(e).__name__}: {e}", flush=True)
    import traceback
    traceback.print_exc()
    sys.stdout.flush()
    sys.stderr.flush()
    sys.exit(1)
finally:
    print("Test complete.", flush=True)
