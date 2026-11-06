#ifndef ORPHEUS_UTIL_H
#define ORPHEUS_UTIL_H

#include <stddef.h>

void die(const char* fmt, ...);
void log_msg(const char* fmt, ...);

void* xmalloc(size_t n);
void* xcalloc(size_t n, size_t size);
void* xrealloc(void* p, size_t n);
char* xstrndup(const char* s, size_t n);
char* xstrdup(const char* s);

 #endif
