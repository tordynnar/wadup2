/*
 * NumPy trigonometric function stubs for WASI.
 *
 * These are strided loop implementations that match NumPy's ufunc signature:
 *   void FUNC(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data)
 *
 * Only cos and sin are provided here - the rest are already in libnumpy_core.a.
 */

#include <math.h>
#include <stdint.h>
#include <stddef.h>

/* NumPy integer types */
typedef intptr_t npy_intp;

/* NumPy float types */
typedef float npy_float;
typedef double npy_double;

/* UNARY_LOOP macro - iterates over strided input/output arrays */
#define UNARY_LOOP \
    char *ip1 = args[0], *op1 = args[1]; \
    npy_intp is1 = steps[0], os1 = steps[1]; \
    npy_intp n = dimensions[0]; \
    npy_intp i; \
    for(i = 0; i < n; i++, ip1 += is1, op1 += os1)

/*
 * Trigonometric functions stubs for WASI.
 * These wrap standard math.h functions with NumPy's strided loop signature.
 */

void FLOAT_cos(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data) {
    (void)data;
    UNARY_LOOP {
        const npy_float in1 = *(npy_float *)ip1;
        *(npy_float *)op1 = cosf(in1);
    }
}

void DOUBLE_cos(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data) {
    (void)data;
    UNARY_LOOP {
        const npy_double in1 = *(npy_double *)ip1;
        *(npy_double *)op1 = cos(in1);
    }
}

void FLOAT_sin(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data) {
    (void)data;
    UNARY_LOOP {
        const npy_float in1 = *(npy_float *)ip1;
        *(npy_float *)op1 = sinf(in1);
    }
}

void DOUBLE_sin(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data) {
    (void)data;
    UNARY_LOOP {
        const npy_double in1 = *(npy_double *)ip1;
        *(npy_double *)op1 = sin(in1);
    }
}

void FLOAT_tanh(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data) {
    (void)data;
    UNARY_LOOP {
        const npy_float in1 = *(npy_float *)ip1;
        *(npy_float *)op1 = tanhf(in1);
    }
}

void DOUBLE_tanh(char **args, npy_intp const *dimensions, npy_intp const *steps, void *data) {
    (void)data;
    UNARY_LOOP {
        const npy_double in1 = *(npy_double *)ip1;
        *(npy_double *)op1 = tanh(in1);
    }
}
