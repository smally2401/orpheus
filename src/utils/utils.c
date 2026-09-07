#include "utils.h"
#include <stdlib.h>
#include <string.h>

char* orph_strdup(const char* str)
{
	if (!str)
	{
		return NULL;
	}

	size_t len = strlen(str) + 1;
	char* dup = malloc(len);
	if (dup)
	{
		memcpy(dup, str, len);
	}

	return dup;
}

const char* get_extension(const char* path)
{
	const char* dot = strrchr(path, '.');
	if (!dot || strrchr(dot, '/') || strrchr(dot, '\\'))
	{
		return NULL;
	}

	return dot + 1;
}
