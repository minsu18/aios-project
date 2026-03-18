/* Bare-metal C++ stubs — avoid libstdc++ exception/length_error in llama.cpp build.
   Also ensure PRI* macros for aarch64-none-elf (newlib inttypes.h ordering bug). */
#if defined(__cplusplus)
#include <cstdint>
#include <cinttypes>
#ifndef PRId64
#if defined(__LP64__) || (defined(__aarch64__) && !defined(__APPLE__))
#define PRId64 "ld"
#define PRIi64 "ld"
#define PRIu64 "lu"
#else
#define PRId64 "lld"
#define PRIi64 "lld"
#define PRIu64 "llu"
#endif
#endif
#ifndef PRIi64
#define PRIi64 PRId64
#endif
#ifndef PRIu64
#if defined(__LP64__) || (defined(__aarch64__) && !defined(__APPLE__))
#define PRIu64 "lu"
#else
#define PRIu64 "llu"
#endif
#endif
#include <cstdlib>
namespace std {
/* Newlib puts strtol/strtod etc in global ns; C++ expects std:: */
using ::strtol;
using ::strtoll;
using ::strtoul;
using ::strtoull;
using ::strtod;
using ::strtof;
__attribute__((noreturn)) inline void __throw_length_error(const char*) { abort(); }
__attribute__((noreturn)) inline void __throw_bad_alloc() { abort(); }
__attribute__((noreturn)) inline void __throw_logic_error(const char*) { abort(); }
/* Add only stubs the library needs but does not define (avoids redefinition) */
__attribute__((noreturn)) inline void __throw_domain_error(const char*) { abort(); }
__attribute__((noreturn)) inline void __throw_bad_array_new_length() { abort(); }
}
#else
/* C stubs for ggml.c (POSIX clock_gettime absent on bare-metal) */
#include <time.h>
#ifndef CLOCK_MONOTONIC
#define CLOCK_MONOTONIC 1
#endif
static inline int ggml_clock_gettime(int clk_id, struct timespec *ts) {
    (void)clk_id;
    if (ts) { ts->tv_sec = 0; ts->tv_nsec = 0; }
    return 0;
}
#define clock_gettime ggml_clock_gettime
#endif
