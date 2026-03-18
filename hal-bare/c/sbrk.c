/* sbrk for newlib malloc. Required when linking libllama.a.
 * Uses a static heap; llama needs ~64MB. Implements _sbrk for newlib.
 *
 * Build: only compiled when linking libllama (with newlib).
 */

#include <stddef.h>

#define AIOS_C_HEAP_SIZE (32 * 1024 * 1024)  /* 32MB for C malloc */

static char heap[AIOS_C_HEAP_SIZE];
static size_t heap_used;

void * _sbrk(ptrdiff_t incr) {
    char *prev;
    if (incr < 0) {
        return (void *)-1;  /* No shrink support in bump heap */
    }
    if (incr == 0) {
        return (void *)(heap + heap_used);
    }
    if (heap_used + (size_t)incr > AIOS_C_HEAP_SIZE) {
        return (void *)-1;
    }
    prev = heap + heap_used;
    heap_used += (size_t)incr;
    return prev;
}
