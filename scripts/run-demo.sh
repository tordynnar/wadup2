#!/usr/bin/env bash
# Run all WADUP example modules over demo data
#
# This script:
# 1. Creates sample data demonstrating all module types
# 2. Copies all built example modules to a demo directory
# 3. Clears and recreates the wadup_demo Elasticsearch index
# 4. Runs wadup once with all modules
# 5. Leaves data in Elasticsearch for exploration
#
# Usage: ./scripts/run_demo.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WADUP_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WADUP_BIN="$WADUP_ROOT/target/release/wadup"

# Demo directories
DEMO_DIR="$WADUP_ROOT/demo"
MODULES_DIR="$DEMO_DIR/modules"
DATA_DIR="$DEMO_DIR/data"

# Elasticsearch configuration
ES_URL="http://localhost:9200"
ES_INDEX="wadup_demo"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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
        print_info "Run 'cargo build --release' first"
        exit 1
    fi

    if ! curl -s "$ES_URL/_cluster/health" > /dev/null 2>&1; then
        print_error "Elasticsearch not running at $ES_URL"
        print_info "Run 'docker-compose up -d elasticsearch' first"
        exit 1
    fi
}

# Create demo directories
setup_directories() {
    print_info "Setting up demo directories..."
    rm -rf "$DEMO_DIR"
    mkdir -p "$MODULES_DIR" "$DATA_DIR"
    print_success "Created $DEMO_DIR"
}

# Copy all built example modules
copy_modules() {
    print_info "Copying example modules..."

    local count=0

    # Rust modules
    for module in byte-counter zip-extractor sqlite-parser simple-test; do
        local module_name="${module//-/_}"
        local wasm_path="$WADUP_ROOT/examples/$module/target/wasm32-wasip1/release/${module_name}.wasm"
        if [[ -f "$wasm_path" ]]; then
            cp "$wasm_path" "$MODULES_DIR/"
            # Copy precompiled cache if exists
            local cache_path="${wasm_path%.wasm}_precompiled"
            [[ -f "$cache_path" ]] && cp "$cache_path" "$MODULES_DIR/"
            print_success "  $module_name.wasm"
            ((count++))
        else
            print_error "  $module not built (skipping)"
        fi
    done

    # Python modules
    for module in python-sqlite-parser python-counter python-module-test python-multi-file python-lxml-test python-pydantic-test; do
        local module_name="${module//-/_}"
        local wasm_path="$WADUP_ROOT/examples/$module/target/${module_name}.wasm"
        if [[ -f "$wasm_path" ]]; then
            cp "$wasm_path" "$MODULES_DIR/"
            local cache_path="${wasm_path%.wasm}_precompiled"
            [[ -f "$cache_path" ]] && cp "$cache_path" "$MODULES_DIR/"
            print_success "  $module_name.wasm"
            ((count++))
        else
            print_error "  $module not built (skipping)"
        fi
    done

    # Go modules
    for module in go-sqlite-parser; do
        local module_name="${module//-/_}"
        local wasm_path="$WADUP_ROOT/examples/$module/target/${module_name}.wasm"
        if [[ -f "$wasm_path" ]]; then
            cp "$wasm_path" "$MODULES_DIR/"
            local cache_path="${wasm_path%.wasm}_precompiled"
            [[ -f "$cache_path" ]] && cp "$cache_path" "$MODULES_DIR/"
            print_success "  $module_name.wasm"
            ((count++))
        else
            print_error "  $module not built (skipping)"
        fi
    done

    print_info "Copied $count modules"
}

# Create sample demo data
create_demo_data() {
    print_info "Creating demo data..."

    # --- SQLite Databases ---
    # Database 1: Users and posts (blog-like)
    sqlite3 "$DATA_DIR/blog.db" <<'EOF'
CREATE TABLE users (
    id INTEGER PRIMARY KEY,
    username TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TEXT DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE posts (
    id INTEGER PRIMARY KEY,
    user_id INTEGER,
    title TEXT NOT NULL,
    content TEXT,
    published INTEGER DEFAULT 0,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE TABLE comments (
    id INTEGER PRIMARY KEY,
    post_id INTEGER,
    user_id INTEGER,
    body TEXT,
    FOREIGN KEY (post_id) REFERENCES posts(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
INSERT INTO users (username, email) VALUES
    ('alice', 'alice@example.com'),
    ('bob', 'bob@example.com'),
    ('charlie', 'charlie@example.com');
INSERT INTO posts (user_id, title, content, published) VALUES
    (1, 'Hello World', 'My first blog post!', 1),
    (1, 'WADUP is awesome', 'Processing files with WebAssembly...', 1),
    (2, 'Draft post', 'Work in progress...', 0);
INSERT INTO comments (post_id, user_id, body) VALUES
    (1, 2, 'Great post!'),
    (1, 3, 'Welcome to blogging!'),
    (2, 3, 'Interesting stuff');
EOF
    print_success "  blog.db (3 tables: users, posts, comments)"

    # Database 2: E-commerce style
    sqlite3 "$DATA_DIR/shop.db" <<'EOF'
CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    price REAL NOT NULL,
    stock INTEGER DEFAULT 0
);
CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    customer_name TEXT,
    total REAL,
    status TEXT DEFAULT 'pending'
);
INSERT INTO products (name, price, stock) VALUES
    ('Widget', 9.99, 100),
    ('Gadget', 24.99, 50),
    ('Gizmo', 14.99, 75),
    ('Thingamajig', 39.99, 25);
INSERT INTO orders (customer_name, total, status) VALUES
    ('John Doe', 49.97, 'shipped'),
    ('Jane Smith', 24.99, 'pending');
EOF
    print_success "  shop.db (2 tables: products, orders)"

    # --- XML Files ---
    cat > "$DATA_DIR/catalog.xml" <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<catalog>
    <book id="1">
        <title>The Rust Programming Language</title>
        <author>Steve Klabnik</author>
        <year>2019</year>
        <price>39.99</price>
    </book>
    <book id="2">
        <title>Programming WebAssembly with Rust</title>
        <author>Kevin Hoffman</author>
        <year>2019</year>
        <price>34.99</price>
    </book>
    <book id="3">
        <title>Python Crash Course</title>
        <author>Eric Matthes</author>
        <year>2023</year>
        <price>35.99</price>
    </book>
</catalog>
EOF
    print_success "  catalog.xml (book catalog with 3 entries)"

    cat > "$DATA_DIR/config.xml" <<'EOF'
<?xml version="1.0"?>
<configuration>
    <database>
        <host>localhost</host>
        <port>5432</port>
        <name>myapp</name>
    </database>
    <cache enabled="true">
        <ttl>3600</ttl>
        <max_size>1024</max_size>
    </cache>
    <features>
        <feature name="dark_mode" enabled="true"/>
        <feature name="beta_features" enabled="false"/>
    </features>
</configuration>
EOF
    print_success "  config.xml (application configuration)"

    # --- Text Files ---
    cat > "$DATA_DIR/readme.txt" <<'EOF'
WADUP Demo Data
===============

This directory contains sample files for demonstrating WADUP's
processing capabilities across multiple module types.

File Types Included:
- SQLite databases (.db)
- XML documents (.xml)
- Text files (.txt)
- JSON data (.json)
- Binary files (.bin)
- ZIP archives (.zip)

Each file will be processed by all applicable modules, with
results indexed to Elasticsearch for querying via Kibana.
EOF
    print_success "  readme.txt (documentation)"

    cat > "$DATA_DIR/log_sample.txt" <<'EOF'
2024-01-03 10:00:00 INFO  Application started
2024-01-03 10:00:01 DEBUG Loading configuration
2024-01-03 10:00:02 INFO  Connected to database
2024-01-03 10:00:05 WARN  Cache miss for key: user_123
2024-01-03 10:00:10 ERROR Failed to send email: timeout
2024-01-03 10:00:15 INFO  Request processed in 42ms
2024-01-03 10:00:20 DEBUG Garbage collection completed
EOF
    print_success "  log_sample.txt (application logs)"

    # --- JSON Files ---
    cat > "$DATA_DIR/users.json" <<'EOF'
{
    "users": [
        {"id": 1, "name": "Alice", "role": "admin", "active": true},
        {"id": 2, "name": "Bob", "role": "user", "active": true},
        {"id": 3, "name": "Charlie", "role": "user", "active": false}
    ],
    "metadata": {
        "version": "1.0",
        "generated": "2024-01-03"
    }
}
EOF
    print_success "  users.json (user data)"

    # --- Binary Files ---
    # Create a small binary file with recognizable patterns
    printf '\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR' > "$DATA_DIR/sample.bin"
    dd if=/dev/urandom bs=256 count=1 >> "$DATA_DIR/sample.bin" 2>/dev/null
    print_success "  sample.bin (binary data, 256 bytes)"

    # --- ZIP Archives ---
    # Create a ZIP containing various file types
    local zip_temp=$(mktemp -d)
    echo "Hello from inside the ZIP!" > "$zip_temp/greeting.txt"
    echo '{"nested": true, "level": 1}' > "$zip_temp/data.json"
    sqlite3 "$zip_temp/embedded.db" "CREATE TABLE items (id INTEGER, name TEXT); INSERT INTO items VALUES (1, 'nested_item');"
    (cd "$zip_temp" && zip -q "$DATA_DIR/archive.zip" greeting.txt data.json embedded.db)
    rm -rf "$zip_temp"
    print_success "  archive.zip (contains: greeting.txt, data.json, embedded.db)"

    # Create another ZIP with different content
    zip_temp=$(mktemp -d)
    for i in 1 2 3; do
        echo "File number $i with some content for processing" > "$zip_temp/file$i.txt"
    done
    cat > "$zip_temp/manifest.xml" <<'EOF'
<?xml version="1.0"?>
<manifest>
    <file name="file1.txt" type="text"/>
    <file name="file2.txt" type="text"/>
    <file name="file3.txt" type="text"/>
</manifest>
EOF
    (cd "$zip_temp" && zip -q "$DATA_DIR/documents.zip" *.txt manifest.xml)
    rm -rf "$zip_temp"
    print_success "  documents.zip (contains: file1.txt, file2.txt, file3.txt, manifest.xml)"

    # Summary
    local file_count=$(find "$DATA_DIR" -type f | wc -l | tr -d ' ')
    print_info "Created $file_count demo files"
}

# Clear and setup Elasticsearch index
setup_elasticsearch() {
    print_info "Setting up Elasticsearch index '$ES_INDEX'..."

    # Delete existing index
    curl -s -X DELETE "$ES_URL/$ES_INDEX" > /dev/null 2>&1 || true

    # Create fresh index
    curl -s -X PUT "$ES_URL/$ES_INDEX" -H "Content-Type: application/json" -d '{}' > /dev/null

    print_success "Index '$ES_INDEX' ready"
}

# Run WADUP
run_wadup() {
    print_header "Running WADUP"

    print_info "Modules directory: $MODULES_DIR"
    print_info "Data directory: $DATA_DIR"
    print_info "Elasticsearch index: $ES_INDEX"
    echo ""

    "$WADUP_BIN" run \
        --modules "$MODULES_DIR" \
        --input "$DATA_DIR" \
        --es-url "$ES_URL" \
        --es-index "$ES_INDEX" \
        --threads 4 \
        --max-stack 8388608

    echo ""
}

# Print summary
print_summary() {
    print_header "Demo Complete"

    # Refresh index
    curl -s -X POST "$ES_URL/$ES_INDEX/_refresh" > /dev/null

    # Get document counts
    local total=$(curl -s "$ES_URL/$ES_INDEX/_count" | python3 -c "import sys,json; print(json.load(sys.stdin).get('count', 0))")
    local content_count=$(curl -s "$ES_URL/$ES_INDEX/_count" -H "Content-Type: application/json" -d '{"query":{"term":{"doc_type":"content"}}}' | python3 -c "import sys,json; print(json.load(sys.stdin).get('count', 0))")
    local row_count=$(curl -s "$ES_URL/$ES_INDEX/_count" -H "Content-Type: application/json" -d '{"query":{"term":{"doc_type":"row"}}}' | python3 -c "import sys,json; print(json.load(sys.stdin).get('count', 0))")
    local output_count=$(curl -s "$ES_URL/$ES_INDEX/_count" -H "Content-Type: application/json" -d '{"query":{"term":{"doc_type":"module_output"}}}' | python3 -c "import sys,json; print(json.load(sys.stdin).get('count', 0))")

    echo "Documents indexed to Elasticsearch:"
    echo "  - Total documents:    $total"
    echo "  - Content documents:  $content_count"
    echo "  - Row documents:      $row_count"
    echo "  - Module outputs:     $output_count"
    echo ""
    echo "Explore results:"
    echo "  - Kibana:  http://localhost:5601"
    echo "  - ES API:  curl '$ES_URL/$ES_INDEX/_search?pretty'"
    echo ""
    echo "Example queries:"
    echo "  # List all tables"
    echo "  curl -s '$ES_URL/$ES_INDEX/_search' -H 'Content-Type: application/json' -d '"
    echo '    {"query":{"term":{"doc_type":"row"}},"size":0,"aggs":{"tables":{"terms":{"field":"_table.keyword"}}}}'\'
    echo ""
    echo "  # Find all content by filename"
    echo "  curl -s '$ES_URL/$ES_INDEX/_search?pretty' -H 'Content-Type: application/json' -d '"
    echo '    {"query":{"bool":{"must":[{"term":{"doc_type":"content"}},{"match":{"filename":"blog.db"}}]}}}'\'
    echo ""
}

# Main
main() {
    print_header "WADUP Demo"

    check_prerequisites
    setup_directories
    copy_modules
    create_demo_data
    setup_elasticsearch
    run_wadup
    print_summary
}

main "$@"
