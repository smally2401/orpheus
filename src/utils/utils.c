#include "utils.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <glib.h>

const char* get_extension(const char* path)
{
	const char* dot = strrchr(path, '.');
	if (!dot || strrchr(dot, '/') || strrchr(dot, '\\'))
	{
		return NULL;
	}

	return dot + 1;
}

char* expand_tilde(const char* path)
{
	if (!path)
	{
		return NULL;
	}

	if (path[0] != '~')
	{
		return g_strdup(path);
	}

	const char* home = getenv("HOME");
	size_t len = strlen(home) + strlen(path);
	char* expanded_path = g_malloc(len);

	snprintf(expanded_path, len, "%s%s", home, &path[1]);
	return expanded_path;
}

char* get_default_music_dir(void)
{
#ifdef _WIN32
	const char* base = getenv("USERPROFILE");
#else
	const char* base = getenv("HOME");
#endif

	const char* suffix = "/Music";
	size_t len = strlen(base) + strlen(suffix) + 1;
	char* music_dir = g_malloc(len);

	snprintf(music_dir, len, "%s%s", base, suffix);
	return music_dir;
}
