/*
 * Stub trigonometric function implementations for NumPy on WASI.
 * NumPy's generated dispatch code uses uppercase FLOAT_/DOUBLE_ prefixed
 * function names that need to be provided.
 */

#include <math.h>

/* Basic trigonometric functions */
float FLOAT_cos(float x) {
    return cosf(x);
}

double DOUBLE_cos(double x) {
    return cos(x);
}

float FLOAT_sin(float x) {
    return sinf(x);
}

double DOUBLE_sin(double x) {
    return sin(x);
}

float FLOAT_tan(float x) {
    return tanf(x);
}

double DOUBLE_tan(double x) {
    return tan(x);
}

/* Inverse trigonometric functions */
float FLOAT_acos(float x) {
    return acosf(x);
}

double DOUBLE_acos(double x) {
    return acos(x);
}

float FLOAT_asin(float x) {
    return asinf(x);
}

double DOUBLE_asin(double x) {
    return asin(x);
}

float FLOAT_atan(float x) {
    return atanf(x);
}

double DOUBLE_atan(double x) {
    return atan(x);
}

float FLOAT_atan2(float y, float x) {
    return atan2f(y, x);
}

double DOUBLE_atan2(double y, double x) {
    return atan2(y, x);
}

/* Hyperbolic functions (cosh/sinh already defined, but cos/sin wrappers may be needed) */
float FLOAT_cosh(float x) {
    return coshf(x);
}

double DOUBLE_cosh(double x) {
    return cosh(x);
}

float FLOAT_sinh(float x) {
    return sinhf(x);
}

double DOUBLE_sinh(double x) {
    return sinh(x);
}

float FLOAT_tanh(float x) {
    return tanhf(x);
}

double DOUBLE_tanh(double x) {
    return tanh(x);
}

/* Inverse hyperbolic functions */
float FLOAT_acosh(float x) {
    return acoshf(x);
}

double DOUBLE_acosh(double x) {
    return acosh(x);
}

float FLOAT_asinh(float x) {
    return asinhf(x);
}

double DOUBLE_asinh(double x) {
    return asinh(x);
}

float FLOAT_atanh(float x) {
    return atanhf(x);
}

double DOUBLE_atanh(double x) {
    return atanh(x);
}

/* Exponential and logarithmic functions */
float FLOAT_exp(float x) {
    return expf(x);
}

double DOUBLE_exp(double x) {
    return exp(x);
}

float FLOAT_exp2(float x) {
    return exp2f(x);
}

double DOUBLE_exp2(double x) {
    return exp2(x);
}

float FLOAT_expm1(float x) {
    return expm1f(x);
}

double DOUBLE_expm1(double x) {
    return expm1(x);
}

float FLOAT_log(float x) {
    return logf(x);
}

double DOUBLE_log(double x) {
    return log(x);
}

float FLOAT_log2(float x) {
    return log2f(x);
}

double DOUBLE_log2(double x) {
    return log2(x);
}

float FLOAT_log10(float x) {
    return log10f(x);
}

double DOUBLE_log10(double x) {
    return log10(x);
}

float FLOAT_log1p(float x) {
    return log1pf(x);
}

double DOUBLE_log1p(double x) {
    return log1p(x);
}

/* Power functions */
float FLOAT_sqrt(float x) {
    return sqrtf(x);
}

double DOUBLE_sqrt(double x) {
    return sqrt(x);
}

float FLOAT_cbrt(float x) {
    return cbrtf(x);
}

double DOUBLE_cbrt(double x) {
    return cbrt(x);
}

float FLOAT_pow(float x, float y) {
    return powf(x, y);
}

double DOUBLE_pow(double x, double y) {
    return pow(x, y);
}

float FLOAT_hypot(float x, float y) {
    return hypotf(x, y);
}

double DOUBLE_hypot(double x, double y) {
    return hypot(x, y);
}

/* Rounding functions */
float FLOAT_ceil(float x) {
    return ceilf(x);
}

double DOUBLE_ceil(double x) {
    return ceil(x);
}

float FLOAT_floor(float x) {
    return floorf(x);
}

double DOUBLE_floor(double x) {
    return floor(x);
}

float FLOAT_trunc(float x) {
    return truncf(x);
}

double DOUBLE_trunc(double x) {
    return trunc(x);
}

float FLOAT_rint(float x) {
    return rintf(x);
}

double DOUBLE_rint(double x) {
    return rint(x);
}

float FLOAT_round(float x) {
    return roundf(x);
}

double DOUBLE_round(double x) {
    return round(x);
}

/* Absolute value and sign functions */
float FLOAT_fabs(float x) {
    return fabsf(x);
}

double DOUBLE_fabs(double x) {
    return fabs(x);
}

float FLOAT_copysign(float x, float y) {
    return copysignf(x, y);
}

double DOUBLE_copysign(double x, double y) {
    return copysign(x, y);
}

/* Remainder functions */
float FLOAT_fmod(float x, float y) {
    return fmodf(x, y);
}

double DOUBLE_fmod(double x, double y) {
    return fmod(x, y);
}

float FLOAT_remainder(float x, float y) {
    return remainderf(x, y);
}

double DOUBLE_remainder(double x, double y) {
    return remainder(x, y);
}

/* Floating point manipulation */
float FLOAT_ldexp(float x, int exp) {
    return ldexpf(x, exp);
}

double DOUBLE_ldexp(double x, int exp) {
    return ldexp(x, exp);
}

float FLOAT_frexp(float x, int *exp) {
    return frexpf(x, exp);
}

double DOUBLE_frexp(double x, int *exp) {
    return frexp(x, exp);
}

float FLOAT_modf(float x, float *iptr) {
    return modff(x, iptr);
}

double DOUBLE_modf(double x, double *iptr) {
    return modf(x, iptr);
}

/* Min/max functions */
float FLOAT_fmax(float x, float y) {
    return fmaxf(x, y);
}

double DOUBLE_fmax(double x, double y) {
    return fmax(x, y);
}

float FLOAT_fmin(float x, float y) {
    return fminf(x, y);
}

double DOUBLE_fmin(double x, double y) {
    return fmin(x, y);
}

/* Error and gamma functions */
float FLOAT_erf(float x) {
    return erff(x);
}

double DOUBLE_erf(double x) {
    return erf(x);
}

float FLOAT_erfc(float x) {
    return erfcf(x);
}

double DOUBLE_erfc(double x) {
    return erfc(x);
}

float FLOAT_lgamma(float x) {
    return lgammaf(x);
}

double DOUBLE_lgamma(double x) {
    return lgamma(x);
}

float FLOAT_tgamma(float x) {
    return tgammaf(x);
}

double DOUBLE_tgamma(double x) {
    return tgamma(x);
}

/* Classification and comparison */
int FLOAT_isnan(float x) {
    return isnan(x);
}

int DOUBLE_isnan(double x) {
    return isnan(x);
}

int FLOAT_isinf(float x) {
    return isinf(x);
}

int DOUBLE_isinf(double x) {
    return isinf(x);
}

int FLOAT_isfinite(float x) {
    return isfinite(x);
}

int DOUBLE_isfinite(double x) {
    return isfinite(x);
}

int FLOAT_signbit(float x) {
    return signbit(x);
}

int DOUBLE_signbit(double x) {
    return signbit(x);
}
