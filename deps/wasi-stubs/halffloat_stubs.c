/*
 * Stub half-float implementations for WASI 32-bit environment.
 * NumPy's halffloat.cpp uses 64-bit assumptions that don't work on WASI.
 * These stubs provide basic implementations using float intermediates.
 *
 * This file is self-contained and doesn't require numpy headers.
 */

#include <stdint.h>

/* NumPy type definitions */
typedef uint16_t npy_half;
typedef uint16_t npy_uint16;
typedef uint32_t npy_uint32;
typedef int32_t npy_int32;
typedef uint64_t npy_uint64;

/* Half-float special values */
#define NPY_HALF_NAN     ((npy_half)0x7E00)
#define NPY_HALF_PINF    ((npy_half)0x7C00)
#define NPY_HALF_NINF    ((npy_half)0xFC00)
#define NPY_HALF_PZERO   ((npy_half)0x0000)
#define NPY_HALF_NZERO   ((npy_half)0x8000)

/* Simple half-float implementation using IEEE 754 bit manipulation */
npy_half npy_float_to_half(float f) {
    /* Cast float bits to uint32 */
    union { float f; npy_uint32 u; } fu;
    fu.f = f;
    npy_uint32 bits = fu.u;

    /* Extract components */
    npy_uint32 sign = (bits >> 16) & 0x8000;
    npy_int32 exponent = ((bits >> 23) & 0xFF) - 127 + 15;
    npy_uint32 mantissa = bits & 0x7FFFFF;

    /* Handle special cases */
    if (exponent <= 0) {
        if (exponent < -10) {
            return (npy_half)sign;  /* Zero or too small */
        }
        /* Denormalized number */
        mantissa |= 0x800000;
        npy_uint32 shift = 14 - exponent;
        return (npy_half)(sign | (mantissa >> shift));
    } else if (exponent >= 31) {
        if (exponent == 128 && mantissa) {
            /* NaN */
            return (npy_half)(sign | 0x7E00 | (mantissa >> 13));
        }
        /* Infinity or overflow */
        return (npy_half)(sign | 0x7C00);
    }

    return (npy_half)(sign | (exponent << 10) | (mantissa >> 13));
}

float npy_half_to_float(npy_half h) {
    npy_uint32 sign = (h & 0x8000) << 16;
    npy_uint32 exponent = (h >> 10) & 0x1F;
    npy_uint32 mantissa = h & 0x3FF;

    if (exponent == 0) {
        if (mantissa == 0) {
            /* Zero */
            union { npy_uint32 u; float f; } uf;
            uf.u = sign;
            return uf.f;
        }
        /* Denormalized number - normalize it */
        while (!(mantissa & 0x400)) {
            mantissa <<= 1;
            exponent--;
        }
        exponent++;
        mantissa &= 0x3FF;
    } else if (exponent == 31) {
        /* Infinity or NaN */
        union { npy_uint32 u; float f; } uf;
        uf.u = sign | 0x7F800000 | (mantissa << 13);
        return uf.f;
    }

    exponent = exponent - 15 + 127;

    union { npy_uint32 u; float f; } uf;
    uf.u = sign | (exponent << 23) | (mantissa << 13);
    return uf.f;
}

double npy_half_to_double(npy_half h) {
    return (double)npy_half_to_float(h);
}

npy_half npy_double_to_half(double d) {
    return npy_float_to_half((float)d);
}

/* Comparison functions */
int npy_half_eq(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) == npy_half_to_float(h2);
}

int npy_half_ne(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) != npy_half_to_float(h2);
}

int npy_half_le(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) <= npy_half_to_float(h2);
}

int npy_half_lt(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) < npy_half_to_float(h2);
}

int npy_half_ge(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) >= npy_half_to_float(h2);
}

int npy_half_gt(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) > npy_half_to_float(h2);
}

int npy_half_isnan(npy_half h) {
    return ((h & 0x7C00) == 0x7C00) && (h & 0x03FF);
}

int npy_half_isinf(npy_half h) {
    return ((h & 0x7FFF) == 0x7C00);
}

int npy_half_isfinite(npy_half h) {
    return (h & 0x7C00) != 0x7C00;
}

int npy_half_iszero(npy_half h) {
    return (h & 0x7FFF) == 0;
}

int npy_half_signbit(npy_half h) {
    return (h & 0x8000) != 0;
}

npy_half npy_half_copysign(npy_half x, npy_half y) {
    return (x & 0x7FFF) | (y & 0x8000);
}

npy_half npy_half_spacing(npy_half h) {
    npy_half ret;
    npy_uint32 exp = (h & 0x7C00);
    if (exp == 0x7C00) {
        ret = NPY_HALF_NAN;
    } else if (exp == 0) {
        ret = 1;  /* Smallest subnormal */
    } else {
        ret = (exp >> 10) - 24;
        if (ret < 1) ret = 1;
        ret = (npy_half)(ret << 10);
    }
    return ret;
}

npy_half npy_half_nextafter(npy_half x, npy_half y) {
    float fx = npy_half_to_float(x);
    float fy = npy_half_to_float(y);

    if (npy_half_isnan(x) || npy_half_isnan(y)) {
        return NPY_HALF_NAN;
    }
    if (fx == fy) {
        return y;
    }

    if (npy_half_iszero(x)) {
        if (fy > 0) return 1;
        else return 0x8001;
    }

    npy_half ret;
    if ((fx > fy) == !npy_half_signbit(x)) {
        ret = x - 1;
    } else {
        ret = x + 1;
    }
    return ret;
}

npy_half npy_half_divmod(npy_half x, npy_half y, npy_half *modulus) {
    float fx = npy_half_to_float(x);
    float fy = npy_half_to_float(y);
    float div = fx / fy;
    float mod = fx - div * fy;
    *modulus = npy_float_to_half(mod);
    return npy_float_to_half(div);
}

/* Additional bit conversion functions */
npy_uint32 npy_halfbits_to_floatbits(npy_uint16 h) {
    npy_uint32 sign = (h & 0x8000) << 16;
    npy_uint32 exponent = (h >> 10) & 0x1F;
    npy_uint32 mantissa = h & 0x3FF;

    if (exponent == 0) {
        if (mantissa == 0) {
            return sign;  /* Zero */
        }
        /* Denormalized - normalize */
        while (!(mantissa & 0x400)) {
            mantissa <<= 1;
            exponent--;
        }
        exponent++;
        mantissa &= 0x3FF;
    } else if (exponent == 31) {
        /* Infinity or NaN */
        return sign | 0x7F800000 | (mantissa << 13);
    }

    exponent = exponent - 15 + 127;
    return sign | (exponent << 23) | (mantissa << 13);
}

npy_uint16 npy_floatbits_to_halfbits(npy_uint32 f) {
    npy_uint16 sign = (f >> 16) & 0x8000;
    npy_int32 exponent = ((f >> 23) & 0xFF) - 127 + 15;
    npy_uint32 mantissa = f & 0x7FFFFF;

    if (exponent <= 0) {
        if (exponent < -10) {
            return sign;
        }
        mantissa |= 0x800000;
        npy_uint32 shift = 14 - exponent;
        return sign | (mantissa >> shift);
    } else if (exponent >= 31) {
        if (exponent == 128 && mantissa) {
            return sign | 0x7E00 | (mantissa >> 13);
        }
        return sign | 0x7C00;
    }

    return sign | (exponent << 10) | (mantissa >> 13);
}

npy_uint64 npy_halfbits_to_doublebits(npy_uint16 h) {
    /* Convert half to float bits first, then extend to double */
    npy_uint32 fbits = npy_halfbits_to_floatbits(h);
    union { float f; npy_uint32 u; } fu;
    fu.u = fbits;
    union { double d; npy_uint64 u; } du;
    du.d = (double)fu.f;
    return du.u;
}

npy_uint16 npy_doublebits_to_halfbits(npy_uint64 d) {
    union { npy_uint64 u; double d; } du;
    du.u = d;
    return npy_float_to_half((float)du.d);
}

/* Additional nonan comparison functions */
int npy_half_lt_nonan(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) < npy_half_to_float(h2);
}

int npy_half_le_nonan(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) <= npy_half_to_float(h2);
}

int npy_half_gt_nonan(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) > npy_half_to_float(h2);
}

int npy_half_ge_nonan(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) >= npy_half_to_float(h2);
}

int npy_half_eq_nonan(npy_half h1, npy_half h2) {
    return npy_half_to_float(h1) == npy_half_to_float(h2);
}

npy_half npy_half_neg(npy_half h) {
    return h ^ 0x8000;
}

npy_half npy_half_abs(npy_half h) {
    return h & 0x7FFF;
}
