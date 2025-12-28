package wadup

import (
	"unsafe"
)

// Import host functions from WASM runtime
//
//go:wasmimport env define_table
func defineTableFFI(namePtr, nameLen, columnsPtr, columnsLen uint32) int32

//go:wasmimport env insert_row
func insertRowFFI(tablePtr, tableLen, rowPtr, rowLen uint32) int32

// stringToFFI converts a Go string to pointer/length pair for FFI calls
func stringToFFI(s string) (uint32, uint32) {
	if len(s) == 0 {
		return 0, 0
	}
	bytes := []byte(s)
	ptr := &bytes[0]
	return uint32(uintptr(unsafe.Pointer(ptr))), uint32(len(s))
}
