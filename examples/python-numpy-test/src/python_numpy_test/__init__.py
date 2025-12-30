"""NumPy WASM test module for WADUP.

Tests basic NumPy functionality in a WebAssembly environment.
"""

import wadup


def main():
    """Process input and demonstrate NumPy capabilities."""
    # Read input file
    with open('/data.bin', 'rb') as f:
        data = f.read()

    # Define output table
    wadup.define_table("numpy_result", [
        ("numpy_version", "String"),
        ("input_data", "String"),
        ("array_shape", "String"),
        ("array_dtype", "String"),
        ("sum", "Float64"),
        ("mean", "Float64"),
        ("min", "Float64"),
        ("max", "Float64"),
        ("std", "Float64"),
        ("squared", "String"),
        ("sorted", "String"),
        ("status", "String"),
    ])

    try:
        # Try importing just numpy._core to test basic functionality
        # without pulling in linalg
        import sys
        debug_info = []

        # Test: Direct import of _core._multiarray_umath
        try:
            import numpy._core._multiarray_umath as _mu
            debug_info.append("_multiarray_umath: OK")
        except Exception as e:
            debug_info.append(f"_multiarray_umath: {e}")

        # Test: Import numpy._core (avoids linalg)
        try:
            import numpy._core as _core
            debug_info.append("numpy._core: OK")
        except Exception as e:
            debug_info.append(f"numpy._core: {e}")

        # Test: Create a basic array using _core directly
        try:
            arr = _core.array([1.0, 2.0, 3.0, 4.0, 5.0])
            debug_info.append(f"array creation: OK, shape={arr.shape}")

            # Parse input as numbers
            text = data.decode('utf-8', errors='replace').strip()
            numbers = []
            for line in text.split('\n'):
                for item in line.split(','):
                    item = item.strip()
                    if item:
                        try:
                            numbers.append(float(item))
                        except ValueError:
                            pass

            if not numbers:
                numbers = [1, 2, 3, 4, 5]  # Default test data

            arr = _core.array(numbers)

            wadup.insert_row("numpy_result", [
                "2.1.3",  # Version
                str(numbers),
                str(list(arr.shape)),
                str(arr.dtype),
                float(_core.sum(arr)),
                float(_core.mean(arr)),
                float(_core.min(arr)),
                float(_core.max(arr)),
                float(_core.std(arr)),
                str(list(arr ** 2)),
                str(list(_core.sort(arr))),
                "success (using _core)",
            ])
            wadup.flush()
            return

        except Exception as e:
            debug_info.append(f"array ops: {type(e).__name__}: {e}")

        # If _core didn't work, report the debug info
        wadup.insert_row("numpy_result", [
            "N/A",
            data.decode('utf-8', errors='replace')[:100],
            "N/A",
            "N/A",
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            "N/A",
            "N/A",
            f"partial: {', '.join(debug_info)}",
        ])

    except Exception as e:
        import traceback
        tb = traceback.format_exc()
        wadup.insert_row("numpy_result", [
            "N/A",
            data.decode('utf-8', errors='replace')[:100],
            "N/A",
            "N/A",
            0.0,
            0.0,
            0.0,
            0.0,
            0.0,
            "N/A",
            "N/A",
            f"error: {type(e).__name__}: {e} | tb: {tb[:400]}",
        ])

    wadup.flush()
