#include "util.h"

#include <stdarg.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void die(const char* fmt, ...)
{
    va_list ap;
    fputs("orpheus: fatal: ", stderr);
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
    exit(1);
}

void log_msg(const char* fmt, ...)
{
    va_list ap;
    fputs("orpheus: ", stderr);
    va_start(ap, fmt);
    vfprintf(stderr, fmt, ap);
    va_end(ap);
    fputc('\n', stderr);
}

void* xmalloc(size_t n)
{
    void* p = malloc(n ? n : 1);
    if (!p)
    {
        die("out of memory (%zu bytes)", n);
    }

    return p;
}

void* xcalloc(size_t n, size_t size)
{
    void* p = calloc(n ? n : 1, size ? size : 1);
    if (!p)
    {
        die("out of memory (%zu x %zu bytes)", n, size);
    }

    return p;
}

void* xrealloc(void* p, size_t n)
{
    void* q = realloc(p, n ? n : 1);
    if (!q)
    {
        die("out of memory (%zu bytes)", n);
    }

    return q;
}

char* xstrndup(const char* s, size_t n)
{
    char* d = xmalloc(n + 1);
    memcpy(d, s, n);
    d[n] = '\0';
    return d;
}

char* xstrdup(const char* s)
{
    return xstrndup(s, strlen(s));
}
