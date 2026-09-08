#ifndef LIBRARY_H
#define LIBRARY_H

#include <glib.h>

typedef struct
{
	const char* album_title;
	const char* album_artist;
} AlbumKey;

GHashTable* scan_library(const char* root_path);
GHashTable* build_library(GHashTable* song_paths);
GPtrArray* sorted_albums(GHashTable* library);

#endif
