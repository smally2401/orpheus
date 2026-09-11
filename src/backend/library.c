#include "library.h"

#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <SDL3/SDL.h>

#include <glib.h>

#include "backend.h"
#include "song.h"
#include "../utils/utils.h"

static guint album_key_hash(gconstpointer key)
{
	const AlbumKey* k = (const AlbumKey*)key;
	return g_str_hash(k->album_title) ^ g_str_hash(k->album_artist);
}

static gboolean album_key_equal(gconstpointer a, gconstpointer b)
{
	const AlbumKey* ka = (const AlbumKey*)a;
	const AlbumKey* kb = (const AlbumKey*)b;

	return strcmp(ka->album_title, kb->album_title) == 0 &&
	    strcmp(ka->album_artist, kb->album_artist) == 0;
}

static void song_destroy(gpointer data)
{
	song_free((Song*)data);
}

static char* join_path(const char* dirname, const char* fname)
{
	size_t len = strlen(dirname) + strlen(fname) + 1;
	char* res = g_malloc(len);

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

	g_free(full_path);
	return SDL_ENUM_CONTINUE;
}

static void album_key_destroy(gpointer data)
{
	g_free((AlbumKey*)data);
}

static void album_destroy(gpointer data)
{
	Album* album = (Album*)data;
	g_ptr_array_free(album->tracklist, TRUE);
	g_free(album);
}

void push_song(GPtrArray* tracklist, Song* song)
{
	if (tracklist->len == 0)
	{
		g_ptr_array_add(tracklist, song);
		return;
	}

	for (guint i = 0; i < tracklist->len; i++)
	{
		Song* tracklist_song = (Song*)tracklist->pdata[i];
		if (song->number < tracklist_song->number)
		{
			g_ptr_array_insert(tracklist, (gint)i, song);
			return;
		}
	}

	g_ptr_array_add(tracklist, song);
}

// Groups every Song in song_paths by (album_title, album_artist) into a second
// table of Albums. Caller is responsible for destroying the table.
GHashTable* build_library(GHashTable* song_paths)
{
	GHashTable* library = g_hash_table_new_full(
	    album_key_hash, album_key_equal, album_key_destroy, album_destroy);

	GHashTableIter iter;
	gpointer key;
	gpointer value;
	g_hash_table_iter_init(&iter, song_paths);

	while (g_hash_table_iter_next(&iter, &key, &value))
	{
		Song* song = (Song*)value;

		AlbumKey lookup_key = {
		    .album_title = song->album_title,
		    .album_artist = song->album_artist,
		};

		Album* album = (Album*)g_hash_table_lookup(library, &lookup_key);

		if (album)
		{
			push_song(album->tracklist, song);
		}
		else
		{
			AlbumKey* new_key = g_malloc(sizeof(AlbumKey));
			new_key->album_title = song->album_title;
			new_key->album_artist = song->album_artist;

			Album* new_album = g_malloc(sizeof(Album));
			new_album->title = song->album_title;
			new_album->artist = song->album_artist;
			new_album->art = song->art;
			new_album->tracklist = g_ptr_array_new();
			push_song(new_album->tracklist, song);

			g_hash_table_insert(library, new_key, new_album);
		}
	}

	return library;
}

GHashTable* scan_library(const char* root_path)
{
	GHashTable* song_paths =
	    g_hash_table_new_full(g_str_hash, g_str_equal, NULL, song_destroy);
	SDL_EnumerateDirectory(root_path, visit_entry, song_paths);

	return song_paths;
}

static int compare_albums_by_title(const void* a, const void* b)
{
	const Album* album_a = *(const Album**)a;
	const Album* album_b = *(const Album**)b;
	return strcmp(album_a->title, album_b->title);
}

GPtrArray* sorted_albums(GHashTable* library)
{
	GPtrArray* albums = g_ptr_array_new();

	GHashTableIter iter;
	gpointer album;
	g_hash_table_iter_init(&iter, library);

	while (g_hash_table_iter_next(&iter, NULL, &album))
	{
		g_ptr_array_add(albums, album);
	}

	g_ptr_array_sort(albums, compare_albums_by_title);
	return albums;
}
