/*
 * C++ runtime stubs for WASI
 *
 * WASI SDK's libc++ doesn't provide operator new/delete by default.
 * These implementations use the C malloc/free functions.
 */

#include <stdlib.h>
#include <stddef.h>
#include <stdio.h>
#include <stdarg.h>

/* operator new(unsigned long) */
void* _Znwm(size_t size) {
    void* ptr = malloc(size);
    if (!ptr && size > 0) {
        /* In a real implementation, this would throw std::bad_alloc */
        abort();
    }
    return ptr;
}

/* operator delete(void*, unsigned long) - sized delete */
void _ZdlPvm(void* ptr, size_t size) {
    (void)size;  /* size hint is ignored */
    free(ptr);
}

/* operator delete(void*) - unsized delete */
void _ZdlPv(void* ptr) {
    free(ptr);
}

/* std::__1::__libcpp_verbose_abort - called on internal libc++ errors */
void _ZNSt3__222__libcpp_verbose_abortEPKcz(const char* format, ...) {
    va_list args;
    va_start(args, format);
    vfprintf(stderr, format, args);
    va_end(args);
    fprintf(stderr, "\n");
    abort();
}
