/* Newlib syscall stubs for bare-metal. Required when linking libllama.a + libc.
 * Provides minimal implementations so newlib/libc can link; no real I/O.
 *
 * Build: only compiled when linking libllama (AIOS_LLAMA_LINKED).
 */

#include <stddef.h>
#include <stdint.h>
#include <errno.h>

/* --- C runtime / atexit --- */
void _fini(void) { /* no-op for bare-metal */ }

/* --- Reentrant syscall stubs (newlib _*_r call these) --- */
int _stat(const char *path, void *buf) { (void)path; (void)buf; return -1; }
int _fstat(int fd, void *buf)          { (void)fd; (void)buf; return -1; }
int _gettimeofday(void *tv, void *tz)   { if (tv) { ((long *)tv)[0] = 0; ((long *)tv)[1] = 0; } (void)tz; return 0; }
int _write(int fd, const void *buf, size_t n) { (void)fd; (void)buf; (void)n; return (int)n; }
int _read(int fd, void *buf, size_t n)  { (void)fd; (void)buf; (void)n; return 0; }
int _open(const char *path, int flags, int mode) { (void)path; (void)flags; (void)mode; return -1; }
long _lseek(int fd, long off, int whence) { (void)fd; (void)off; (void)whence; return -1; }
int _close(int fd)                      { (void)fd; return -1; }
int _isatty(int fd)                     { (void)fd; return 0; }
long _times(void *buf)                  { (void)buf; return -1; }
int _kill(int pid, int sig)             { (void)pid; (void)sig; return -1; }
int _getpid(void)                       { return 1; }

/* --- Exit / abort --- */
void _exit(int status) {
    (void)status;
    for (;;) { __asm__ volatile ("wfi"); }  /* Halt; WFI on ARM */
}

/* --- posix_memalign: required by ggml_aligned_malloc --- */
int posix_memalign(void **memptr, size_t alignment, size_t size) {
    extern void *malloc(size_t);
    if (memptr == NULL) return EINVAL;
    if (alignment % sizeof(void *) != 0 || (alignment & (alignment - 1)) != 0)
        return EINVAL;
    void *p = malloc(size + alignment - 1);
    if (p == NULL) return ENOMEM;
    uintptr_t a = (uintptr_t)p;
    uintptr_t aligned = (a + alignment - 1) & ~(alignment - 1);
    *memptr = (void *)aligned;
    return 0;
}

/* --- sysconf: required by ggml_backend_cpu_device_get_memory --- */
long sysconf(int name) {
    (void)name;
    return 4096;  /* _SC_PAGESIZE; typical page size */
}

/* --- lroundf: required by ggml-quants --- */
long lroundf(float x) {
    return (long)(x + (x >= 0 ? 0.5f : -0.5f));
}
