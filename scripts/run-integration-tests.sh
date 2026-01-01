#!/usr/bin/env bash
# Run WADUP integration tests against pre-built example modules
#
# Usage: ./scripts/run-integration-tests.sh [test-name]
#
# Run all tests:
#   ./scripts/run-integration-tests.sh
#
# Run a specific test:
#   ./scripts/run-integration-tests.sh test_python_pydantic

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WADUP_BIN="$WADUP_ROOT/target/release/wadup"
FIXTURES_DIR="$WADUP_ROOT/tests/fixtures"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results (stored in temp file for compatibility with bash 3.x)
RESULTS_FILE=$(mktemp)
TESTS_PASSED=0
TESTS_FAILED=0
TOTAL_START=$(python3 -c "import time; print(time.time())")

cleanup_results() {
    rm -f "$RESULTS_FILE" 2>/dev/null || true
}
trap cleanup_results EXIT

print_header() {
    echo ""
    echo -e "${BLUE}============================================================${NC}"
    echo -e "${BLUE}  $1${NC}"
    echo -e "${BLUE}============================================================${NC}"
    echo ""
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    if [[ ! -f "$WADUP_BIN" ]]; then
        print_error "wadup binary not found at $WADUP_BIN"
        print_info "Run ./scripts/build-examples.sh first"
        exit 1
    fi
}

# Get WASM path for a module
get_wasm_path() {
    local name="$1"
    local module_name="${name//-/_}"

    # Check Python module location first
    local python_path="$WADUP_ROOT/examples/$name/target/${module_name}.wasm"
    if [[ -f "$python_path" ]]; then
        echo "$python_path"
        return
    fi

    # Check Rust module location
    local rust_path="$WADUP_ROOT/examples/$name/target/wasm32-wasip1/release/${module_name}.wasm"
    if [[ -f "$rust_path" ]]; then
        echo "$rust_path"
        return
    fi

    echo ""
}

# Record test result
record_result() {
    local test_name="$1"
    local result="$2"
    local duration="$3"
    echo "$test_name|$result|$duration" >> "$RESULTS_FILE"
}

# Run a single test
run_test() {
    local test_name="$1"

    local start=$(python3 -c "import time; print(time.time())")

    set +e
    $test_name
    local exit_code=$?
    set -e

    local end=$(python3 -c "import time; print(time.time())")
    local duration=$(python3 -c "print(f'{$end - $start:.2f}')")

    if [[ $exit_code -eq 0 ]]; then
        record_result "$test_name" "PASS" "$duration"
        ((TESTS_PASSED++)) || true
        echo -e "${GREEN}⏱️  $test_name passed in ${duration}s${NC}"
    else
        record_result "$test_name" "FAIL" "$duration"
        ((TESTS_FAILED++)) || true
        echo -e "${RED}⏱️  $test_name failed in ${duration}s${NC}"
    fi
}

# Setup temp directories for a test
setup_test_env() {
    MODULES_DIR=$(mktemp -d)
    INPUT_DIR=$(mktemp -d)
    OUTPUT_DB=$(mktemp)
    rm "$OUTPUT_DB"  # Remove so wadup creates it
    OUTPUT_DB="${OUTPUT_DB}.db"
}

# Cleanup temp directories
cleanup_test_env() {
    rm -rf "$MODULES_DIR" "$INPUT_DIR" "$OUTPUT_DB" 2>/dev/null || true
}

# Copy module to test directory
copy_module() {
    local name="$1"
    local wasm_path=$(get_wasm_path "$name")

    if [[ -z "$wasm_path" || ! -f "$wasm_path" ]]; then
        print_error "Module $name not built. Run ./scripts/build-examples.sh first"
        return 1
    fi

    cp "$wasm_path" "$MODULES_DIR/"

    # Also copy precompiled cache if it exists
    local wasm_dir=$(dirname "$wasm_path")
    local wasm_stem=$(basename "${wasm_path%.wasm}")
    local cache_path="$wasm_dir/${wasm_stem}_precompiled"
    if [[ -f "$cache_path" ]]; then
        cp "$cache_path" "$MODULES_DIR/"
    fi
}

# Run wadup and capture output
run_wadup() {
    local extra_args=("$@")

    "$WADUP_BIN" run \
        --modules "$MODULES_DIR" \
        --input "$INPUT_DIR" \
        --output "$OUTPUT_DB" \
        "${extra_args[@]}" \
        2>&1
}

# Query SQLite and return result
query_db() {
    local query="$1"
    sqlite3 "$OUTPUT_DB" "$query"
}

# Assert table exists
assert_table_exists() {
    local table="$1"
    local count=$(query_db "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='$table'")
    if [[ "$count" -eq 0 ]]; then
        print_error "Table '$table' not found"
        return 1
    fi
}

# Assert row count
assert_row_count() {
    local table="$1"
    local expected="$2"
    local op="${3:-eq}"  # eq, ge, gt, le, lt

    local count=$(query_db "SELECT COUNT(*) FROM $table")

    case "$op" in
        eq) [[ "$count" -eq "$expected" ]] || { print_error "Expected $expected rows in $table, got $count"; return 1; } ;;
        ge) [[ "$count" -ge "$expected" ]] || { print_error "Expected >= $expected rows in $table, got $count"; return 1; } ;;
        gt) [[ "$count" -gt "$expected" ]] || { print_error "Expected > $expected rows in $table, got $count"; return 1; } ;;
        le) [[ "$count" -le "$expected" ]] || { print_error "Expected <= $expected rows in $table, got $count"; return 1; } ;;
        lt) [[ "$count" -lt "$expected" ]] || { print_error "Expected < $expected rows in $table, got $count"; return 1; } ;;
    esac
}

# Assert value equals
assert_value() {
    local query="$1"
    local expected="$2"
    local actual=$(query_db "$query")

    if [[ "$actual" != "$expected" ]]; then
        print_error "Expected '$expected', got '$actual'"
        return 1
    fi
}

# ============================================================
# Test implementations
# ============================================================

test_sqlite_parser() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "sqlite-parser" || return 1
    cp "$FIXTURES_DIR/sample.db" "$INPUT_DIR/"

    run_wadup > /dev/null || return 1

    assert_table_exists "db_table_stats" || return 1
    assert_row_count "db_table_stats" 1 "ge" || return 1
}

test_zip_extractor_and_byte_counter() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "zip-extractor" || return 1
    copy_module "byte-counter" || return 1
    cp "$FIXTURES_DIR/test.zip" "$INPUT_DIR/"

    run_wadup > /dev/null || return 1

    assert_table_exists "file_sizes" || return 1
    assert_row_count "file_sizes" 3 "ge" || return 1
}

test_combined_sqlite_and_zip() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "sqlite-parser" || return 1
    copy_module "zip-extractor" || return 1
    copy_module "byte-counter" || return 1
    cp "$FIXTURES_DIR/sample.db" "$INPUT_DIR/"
    cp "$FIXTURES_DIR/test.zip" "$INPUT_DIR/"

    run_wadup > /dev/null || return 1

    assert_table_exists "db_table_stats" || return 1
    assert_table_exists "file_sizes" || return 1
}

test_python_sqlite_parser() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "python-sqlite-parser" || return 1
    cp "$FIXTURES_DIR/sample.db" "$INPUT_DIR/"

    run_wadup > /dev/null || return 1

    assert_table_exists "db_table_stats" || return 1
    assert_row_count "db_table_stats" 1 "ge" || return 1
}

test_go_sqlite_parser() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "go-sqlite-parser" || return 1
    cp "$FIXTURES_DIR/sample.db" "$INPUT_DIR/"

    run_wadup > /dev/null || return 1

    assert_table_exists "db_table_stats" || return 1
    assert_row_count "db_table_stats" 1 "ge" || return 1
}

test_python_module_reuse() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "python-counter" || return 1

    # Create 3 input files
    echo "file1" > "$INPUT_DIR/file1.txt"
    echo "file2" > "$INPUT_DIR/file2.txt"
    echo "file3" > "$INPUT_DIR/file3.txt"

    # Use single thread for deterministic ordering
    run_wadup --threads 1 > /dev/null || return 1

    assert_table_exists "call_counter" || return 1

    # Verify counter increments (module reuse)
    local values=$(query_db "SELECT call_number FROM call_counter ORDER BY call_number")
    local expected=$'1\n2\n3'

    if [[ "$values" != "$expected" ]]; then
        print_error "Counter values should be 1,2,3 (module reused), got: $values"
        return 1
    fi
}

test_python_c_extensions() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "python-module-test" || return 1
    echo "test" > "$INPUT_DIR/test.txt"

    run_wadup > /dev/null || return 1

    assert_table_exists "c_extension_imports" || return 1
}

test_csharp_json_analyzer() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "csharp-json-analyzer" || return 1

    # Create test JSON file
    echo '{"name": "test", "values": [1, 2, 3], "nested": {"a": 1}}' > "$INPUT_DIR/test.json"

    run_wadup > /dev/null || return 1

    assert_table_exists "json_metadata" || return 1
    assert_table_exists "json_keys" || return 1
    assert_row_count "json_metadata" 1 || return 1
}

test_python_multi_file() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "python-multi-file" || return 1

    # Create test files
    echo -e "Hello World\nThis is a test\nThree lines" > "$INPUT_DIR/text.txt"
    printf '\x00\x01\x02\x03\x04' > "$INPUT_DIR/binary.bin"

    run_wadup > /dev/null || return 1

    assert_table_exists "file_analysis" || return 1
    assert_row_count "file_analysis" 2 || return 1
}

test_simple_module() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "simple-test" || return 1
    echo "test" > "$INPUT_DIR/test.txt"

    # Simple module doesn't create tables, just verify wadup runs without error
    run_wadup > /dev/null || return 1

    # Verify the database was created
    if [[ ! -f "$OUTPUT_DB" ]]; then
        print_error "Output database was not created"
        return 1
    fi
}

test_python_lxml() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "python-lxml-test" || return 1

    # Create test XML file
    cat > "$INPUT_DIR/test.xml" << 'EOF'
<?xml version="1.0"?>
<root>
    <item id="1">First</item>
    <item id="2">Second</item>
</root>
EOF

    run_wadup > /dev/null || return 1

    assert_table_exists "xml_elements" || return 1
    assert_row_count "xml_elements" 1 "ge" || return 1
}

test_python_numpy() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "python-numpy-test" || return 1
    echo "test" > "$INPUT_DIR/test.txt"

    run_wadup > /dev/null || return 1

    assert_table_exists "numpy_result" || return 1

    # Verify numpy version is reported
    local version=$(query_db "SELECT numpy_version FROM numpy_result LIMIT 1" 2>/dev/null || echo "")
    if [[ -z "$version" ]]; then
        print_error "NumPy version not found in results"
        return 1
    fi
}

test_python_pandas() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "python-pandas-test" || return 1
    echo "test" > "$INPUT_DIR/test.txt"

    run_wadup > /dev/null || return 1

    assert_table_exists "pandas_result" || return 1

    # Verify pandas version is reported
    local version=$(query_db "SELECT pandas_version FROM pandas_result LIMIT 1" 2>/dev/null || echo "")
    if [[ -z "$version" ]]; then
        print_error "Pandas version not found in results"
        return 1
    fi
}

test_python_pydantic() {
    setup_test_env
    trap cleanup_test_env RETURN

    copy_module "python-pydantic-test" || return 1
    echo "test" > "$INPUT_DIR/test.txt"

    # Pydantic needs larger stack
    run_wadup --max-stack 8388608 > /dev/null || return 1

    assert_table_exists "users" || return 1
    assert_table_exists "info" || return 1

    # Verify 3 users were created
    assert_row_count "users" 3 || return 1

    # Verify status is success
    local status=$(query_db "SELECT value FROM info WHERE key='status'")
    if [[ "$status" != "success" ]]; then
        print_error "Expected status 'success', got '$status'"
        return 1
    fi

    # Verify pydantic version
    local pydantic_version=$(query_db "SELECT value FROM info WHERE key='pydantic_version'")
    if [[ -z "$pydantic_version" ]]; then
        print_error "Pydantic version not found"
        return 1
    fi

    print_info "Pydantic $pydantic_version working with BaseModel"
}

# ============================================================
# Print results
# ============================================================

print_timing_table() {
    echo ""
    echo -e "${BLUE}============================================================${NC}"
    echo -e "${BLUE}  INTEGRATION TEST TIMING SUMMARY${NC}"
    echo -e "${BLUE}============================================================${NC}"
    echo ""

    printf "  %-40s %8s %10s\n" "Test Name" "Result" "Duration"
    printf "  %-40s %8s %10s\n" "----------------------------------------" "--------" "----------"

    # Sort by duration (descending)
    sort -t'|' -k3 -rn "$RESULTS_FILE" | while IFS='|' read -r name result duration; do
        if [[ "$result" == "PASS" ]]; then
            printf "  %-40s ${GREEN}%8s${NC} %9ss\n" "$name" "$result" "$duration"
        else
            printf "  %-40s ${RED}%8s${NC} %9ss\n" "$name" "$result" "$duration"
        fi
    done

    local total_end=$(python3 -c "import time; print(time.time())")
    local total_duration=$(python3 -c "print(f'{$total_end - $TOTAL_START:.2f}')")

    printf "  %-40s %8s %10s\n" "----------------------------------------" "--------" "----------"
    printf "  %-40s %8s %9ss\n" "TOTAL" "" "$total_duration"
    echo ""
    echo -e "  Tests passed: ${GREEN}$TESTS_PASSED${NC}"
    echo -e "  Tests failed: ${RED}$TESTS_FAILED${NC}"
    echo ""
}

# ============================================================
# Main
# ============================================================

check_prerequisites

# Define all tests
ALL_TESTS=(
    "test_sqlite_parser"
    "test_zip_extractor_and_byte_counter"
    "test_combined_sqlite_and_zip"
    "test_python_sqlite_parser"
    "test_go_sqlite_parser"
    "test_python_module_reuse"
    "test_python_c_extensions"
    "test_csharp_json_analyzer"
    "test_python_multi_file"
    "test_simple_module"
    "test_python_lxml"
    "test_python_numpy"
    "test_python_pandas"
    "test_python_pydantic"
)

# Run specific test or all tests
if [[ -n "$1" ]]; then
    # Run specific test
    test_name="$1"
    if declare -f "$test_name" > /dev/null; then
        print_header "Running: $test_name"
        run_test "$test_name"
    else
        print_error "Unknown test: $test_name"
        echo "Available tests:"
        for t in "${ALL_TESTS[@]}"; do
            echo "  - $t"
        done
        exit 1
    fi
else
    # Run all tests
    print_header "Running All Integration Tests"

    for test_name in "${ALL_TESTS[@]}"; do
        run_test "$test_name"
    done
fi

print_timing_table

# Exit with error if any tests failed
if [[ $TESTS_FAILED -gt 0 ]]; then
    exit 1
fi
