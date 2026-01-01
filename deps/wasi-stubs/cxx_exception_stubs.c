/* C++ exception stubs for WASI
 *
 * These stubs provide minimal implementations of C++ exception-related
 * symbols that are needed when compiling with exceptions enabled.
 * In WASI, exceptions don't work the same way as on native platforms,
 * so we just abort on exception.
 */

#include <stdlib.h>

/* std::bad_array_new_length constructor */
void _ZNSt20bad_array_new_lengthC1Ev(void *this) {
    (void)this;
}

/* std::bad_array_new_length constructor (C2) */
void _ZNSt20bad_array_new_lengthC2Ev(void *this) {
    (void)this;
}

/* std::bad_array_new_length destructor */
void _ZNSt20bad_array_new_lengthD1Ev(void *this) {
    (void)this;
}

/* std::bad_array_new_length virtual destructor */
void _ZNSt20bad_array_new_lengthD0Ev(void *this) {
    (void)this;
}

/* std::bad_alloc constructor */
void _ZNSt9bad_allocC1Ev(void *this) {
    (void)this;
}

/* std::bad_alloc constructor (C2) */
void _ZNSt9bad_allocC2Ev(void *this) {
    (void)this;
}

/* std::bad_alloc destructor */
void _ZNSt9bad_allocD1Ev(void *this) {
    (void)this;
}

/* std::bad_alloc virtual destructor */
void _ZNSt9bad_allocD0Ev(void *this) {
    (void)this;
}

/* std::exception destructor */
void _ZNSt9exceptionD1Ev(void *this) {
    (void)this;
}

/* std::exception virtual destructor */
void _ZNSt9exceptionD0Ev(void *this) {
    (void)this;
}

/* std::exception::what() - returns a C string describing the exception */
const char* _ZNKSt9exception4whatEv(void *this) {
    (void)this;
    return "exception";
}

/* std::bad_alloc::what() */
const char* _ZNKSt9bad_alloc4whatEv(void *this) {
    (void)this;
    return "std::bad_alloc";
}

/* std::bad_array_new_length::what() */
const char* _ZNKSt20bad_array_new_length4whatEv(void *this) {
    (void)this;
    return "std::bad_array_new_length";
}

/* __cxa_allocate_exception - allocate space for exception object */
void* __cxa_allocate_exception(unsigned long size) {
    /* In WASI, we can't really throw exceptions, so just return a dummy buffer */
    return malloc(size);
}

/* __cxa_free_exception - free exception object */
void __cxa_free_exception(void *exception) {
    free(exception);
}

/* __cxa_throw - throw an exception */
void __cxa_throw(void *exception, void *type_info, void (*destructor)(void*)) {
    (void)exception;
    (void)type_info;
    (void)destructor;
    /* In WASI, we can't throw exceptions - just abort */
    abort();
}

/* __cxa_begin_catch - begin catching an exception */
void* __cxa_begin_catch(void *exception) {
    return exception;
}

/* __cxa_end_catch - end catching an exception */
void __cxa_end_catch(void) {
}

/* __cxa_rethrow - rethrow the current exception */
void __cxa_rethrow(void) {
    abort();
}

/* __cxa_pure_virtual - called when a pure virtual function is invoked */
void __cxa_pure_virtual(void) {
    abort();
}

/* std::terminate - called when exception handling fails */
void _ZSt9terminatev(void) {
    abort();
}

/* __gxx_personality_v0 - exception personality routine (for unwinding) */
int __gxx_personality_v0(void) {
    return 0;
}

/* _Unwind_Resume - continue unwinding after catch */
void _Unwind_Resume(void *exception) {
    (void)exception;
    abort();
}
