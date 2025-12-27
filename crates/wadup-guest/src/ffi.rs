// External host functions provided by the WASM runtime

#[link(wasm_import_module = "env")]
extern "C" {
    pub fn define_table(
        name_ptr: *const u8,
        name_len: usize,
        columns_ptr: *const u8,
        columns_len: usize,
    ) -> i32;

    pub fn insert_row(
        table_name_ptr: *const u8,
        table_name_len: usize,
        row_ptr: *const u8,
        row_len: usize,
    ) -> i32;

    pub fn emit_subcontent_bytes(
        data_ptr: *const u8,
        data_len: usize,
        filename_ptr: *const u8,
        filename_len: usize,
    ) -> i32;

    pub fn emit_subcontent_slice(
        offset: usize,
        length: usize,
        filename_ptr: *const u8,
        filename_len: usize,
    ) -> i32;
}
