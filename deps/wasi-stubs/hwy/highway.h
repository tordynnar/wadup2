// Stub highway.h for WASI - disables Highway SIMD sorting
#ifndef HWY_HIGHWAY_H_
#define HWY_HIGHWAY_H_

// Disable Highway sorting completely
#define NPY_DISABLE_HIGHWAY_SORT 1
#define HWY_COMPILER_MSVC 0
#define HWY_IS_DEBUG_BUILD 0
#define HWY_ARCH_ARM_V7 0
#define HWY_ARCH_ARM_A64 0
#define HWY_COMPILER_GCC_ACTUAL 0
#define HWY_COMPILER_CLANG 0
#define HWY_IS_ASAN 0
#define HWY_IS_HWASAN 0
#define HWY_IS_MSAN 0
#define HWY_IS_TSAN 0

#endif // HWY_HIGHWAY_H_
