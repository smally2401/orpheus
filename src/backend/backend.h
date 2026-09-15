#ifndef APP_STATE_H
#define APP_STATE_H

#include "../audio/audio.h"
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
	GPtrArray* songs;
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
	GPtrArray* playlists;
	GHashTable* song_paths;
	GHashTable* built_library;
	GPtrArray* library;
	GPtrArray* queue;
	GArray* order;
	int index;
	bool shuffle;
	RepeatMode repeat;
	float volume;
	bool paused;
} OrpheusBackend;

void playlist_free(gpointer data);
OrpheusBackend* backend_init(const char* music_dir);
void backend_destroy(OrpheusBackend* backend);
void backend_toggle_play(OrpheusBackend* backend);
void backend_prev(OrpheusBackend* backend);
void backend_next(OrpheusBackend* backend);
void backend_load_album_to_queue(OrpheusBackend* backend, Album* album,
                                 int idx);
void backend_load_playlist_to_queue(OrpheusBackend* backend, Playlist* playlist,
                                    int idx);
Song* song_from_path(OrpheusBackend* backend, const char* path);
float backend_get_position_seconds(OrpheusBackend* backend);
float backend_get_duration_seconds(OrpheusBackend* backend);
void backend_seek(OrpheusBackend* backend, float seconds);
void backend_tick(OrpheusBackend* backend);
void backend_toggle_repeat(OrpheusBackend* backend);
void backend_toggle_shuffle(OrpheusBackend* backend);

#endif
