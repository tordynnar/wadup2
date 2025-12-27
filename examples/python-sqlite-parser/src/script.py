import sqlite3
import wadup

def main():
    db_path = "/data.bin"

    # Validate SQLite header
    try:
        with open(db_path, 'rb') as f:
            header = f.read(16)
            if header != b'SQLite format 3\x00':
                return  # Not a SQLite database
    except:
        return  # File doesn't exist or can't be read

    # Open database
    try:
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()

        # Query for user tables
        cursor.execute("""
            SELECT name FROM sqlite_master
            WHERE type='table' AND name NOT LIKE 'sqlite_%'
        """)
        tables = cursor.fetchall()

        # Define output table
        wadup.define_table("db_table_stats", [
            ("table_name", "String"),
            ("row_count", "Int64")
        ])

        # Count rows in each table
        for (table_name,) in tables:
            cursor.execute(f'SELECT COUNT(*) FROM "{table_name}"')
            count = cursor.fetchone()[0]

            wadup.insert_row("db_table_stats", [table_name, count])

        conn.close()
    except Exception as e:
        # Silently ignore errors - file might not be a valid SQLite database
        pass

if __name__ == "__main__":
    main()
