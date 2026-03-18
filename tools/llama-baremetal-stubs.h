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
__attribute__((noreturn)) inline void __throw_length_error(const char*) { abort(); }
__attribute__((noreturn)) inline void __throw_bad_alloc() { abort(); }
__attribute__((noreturn)) inline void __throw_logic_error(const char*) { abort(); }
__attribute__((noreturn)) inline void __throw_runtime_error(const char*) { abort(); }
__attribute__((noreturn)) inline void __throw_out_of_range_fmt(const char*, ...) { abort(); }
}
#endif
