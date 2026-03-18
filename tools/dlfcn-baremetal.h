/* Bare-metal stub for dlfcn.h - no dynamic loading */
#ifndef DLFCN_BAREMETAL_H
#define DLFCN_BAREMETAL_H
#define RTLD_LAZY   1
#define RTLD_NOW    2
#define RTLD_LOCAL  4
static inline void *dlopen(const char *path, int mode) { (void)path;(void)mode; return 0; }
static inline void *dlsym(void *handle, const char *name) { (void)handle;(void)name; return 0; }
static inline int dlclose(void *handle) { (void)handle; return 0; }
static inline char *dlerror(void) { return 0; }
#endif
