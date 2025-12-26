use wadup_guest::*;

#[no_mangle]
pub extern "C" fn process() -> i32 {
    // Just return success - this is for testing module loading
    0
}
