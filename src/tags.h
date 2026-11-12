#ifndef ORPHEUS_TAGS_H
#define ORPHEUS_TAGS_H

#include <stdbool.h>
#include <stddef.h>

typedef void (*TagFn)(const char* ket, const char* value, void* ud);
bool tags_read(const char* path, TagFn fn, void* ud);
bool tags_read_picture(const char* path, unsigned char** data, size_t* len);

#endif
