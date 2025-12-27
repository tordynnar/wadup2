import wadup

# Global counter - should persist if interpreter is reused
try:
    call_count
except NameError:
    call_count = 0

call_count += 1

def main():
    # Define output table
    wadup.define_table("call_counter", [
        ("call_number", "Int64")
    ])

    # Insert the current call count
    wadup.insert_row("call_counter", [call_count])

if __name__ == "__main__":
    main()
