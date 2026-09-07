#include "library.h"

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <SDL3/SDL.h>

#include <glib.h>

#include "song.h"
#include "utils/utils.h"

static void song_destroy(gpointer data)
{
	song_free((Song*)data);
}

static char* join_path(const char* dirname, const char* fname)
{
	size_t len = strlen(dirname) + strlen(fname) + 1;
	char* res = malloc(len);
	if (!res)
	{
		return NULL;
	}

	snprintf(res, len, "%s%s", dirname, fname);
	return res;
}

static SDL_EnumerationResult visit_entry(void* userdata, const char* dirname,
                                         const char* fname)
{
	GHashTable* song_paths = (GHashTable*)userdata;
	char* full_path = join_path(dirname, fname);
	const char* ext = get_extension(full_path);

	SDL_PathInfo info;
	SDL_GetPathInfo(full_path, &info);

	if (info.type == SDL_PATHTYPE_DIRECTORY)
	{
		SDL_EnumerateDirectory(full_path, visit_entry, userdata);
	}
	else if (ext && strcmp(ext, "mp3") == 0)
	{
		Song* song = song_create(full_path);
		g_hash_table_insert(song_paths, song->path, song);
	}

	free(full_path);
	return SDL_ENUM_CONTINUE;
}

GHashTable* scan_library(const char* root_path)
{
	GHashTable* song_paths =
	    g_hash_table_new_full(g_str_hash, g_str_equal, NULL, song_destroy);
	SDL_EnumerateDirectory(root_path, visit_entry, song_paths);

	return song_paths;
}
