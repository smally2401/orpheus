#ifndef APP_STATE_H
#define APP_STATE_H

#include "audio.h"
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include <glib.h>

typedef struct
{
	uint8_t* data;
	size_t size;
} AlbumArt;

typedef struct
{
	char* path;
	char* title;
	char* artist;
	char* album_title;
	char* album_artist;
	int number;
	int duration_secs;
	AlbumArt* art;
} Song;

typedef struct
{
	char* title;
	char* artist;
	GPtrArray* tracklist;
	AlbumArt* art;
} Album;

typedef struct
{
	char* name;
	char** songs;
	int track_count;
	bool sort;
	AlbumArt* art;
} Playlist;

typedef enum
{
	REPEAT_OFF,
	REPEAT_QUEUE,
	REPEAT_TRACK,
} RepeatMode;

typedef struct
{
	AudioPlayer* player;
	// Playlist* playlists;
	// int playlist_count;
	GHashTable* song_paths;
	GPtrArray* library;
	GPtrArray* queue;
	// int* order;
	int index;
	// bool shuffle;
	// RepeatMode repeat;
	float volume;
	bool paused;
} OrpheusBackend;

OrpheusBackend* backend_init(const char* music_dir);
void backend_destroy(OrpheusBackend* backend);

#endif
