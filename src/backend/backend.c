#include "backend.h"

#include <glib.h>

#include "../audio/audio.h"
#include "library.h"

OrpheusBackend* backend_init(const char* music_dir)
{
	OrpheusBackend* backend = g_malloc(sizeof(OrpheusBackend));
	backend->player = audio_create();
	backend->song_paths = scan_library(music_dir);
	backend->built_library = build_library(backend->song_paths);
	backend->library = sorted_albums(backend->built_library);
	backend->playlists =
	    g_ptr_array_new_with_free_func((GDestroyNotify)playlist_free);
	backend->queue = g_ptr_array_new();
	backend->index = 0;
	backend->repeat = REPEAT_OFF;
	backend->volume = 1.0F;
	backend->paused = true;

	return backend;
}

void backend_destroy(OrpheusBackend* backend)
{
	audio_destroy(backend->player);
	g_hash_table_destroy(backend->song_paths);
	g_hash_table_destroy(backend->built_library);
	g_ptr_array_free(backend->library, TRUE);
	g_ptr_array_free(backend->playlists, TRUE);
	g_ptr_array_free(backend->queue, TRUE);
	g_free(backend);
}

void backend_play(OrpheusBackend* backend)
{
	audio_play(backend->player);
	backend->paused = false;
}

void backend_pause(OrpheusBackend* backend)
{
	audio_pause(backend->player);
	backend->paused = true;
}

void backend_toggle_play(OrpheusBackend* backend)
{
	if (backend->paused)
	{
		backend_play(backend);
	}
	else
	{
		backend_pause(backend);
	}
}

static void backend_load_track(OrpheusBackend* backend, Song* song)
{
	if (audio_load_file(backend->player, song->path))
	{
		backend->paused = false;
		audio_play(backend->player);
	}
}

void backend_next(OrpheusBackend* backend)
{
	if (backend->index + 1 < (int)backend->queue->len)
	{
		backend->index++;
		backend_load_track(backend, backend->queue->pdata[backend->index]);
	}
}

void backend_prev(OrpheusBackend* backend)
{
	if (backend->index != 0)
	{
		backend->index--;
		backend_load_track(backend, backend->queue->pdata[backend->index]);
	}
}

void backend_load_album_to_queue(OrpheusBackend* backend, Album* album, int idx)
{
	g_ptr_array_free(backend->queue, TRUE);
	backend->queue = g_ptr_array_new();
	for (guint i = 0; i < album->tracklist->len; i++)
	{
		g_ptr_array_add(backend->queue, album->tracklist->pdata[i]);
	}

	backend->index = idx;
	backend_load_track(backend, backend->queue->pdata[idx]);
}

void backend_load_playlist_to_queue(OrpheusBackend* backend, Playlist* playlist,
                                    int idx)
{
	g_ptr_array_free(backend->queue, TRUE);
	backend->queue = g_ptr_array_new();
	for (guint i = 0; i < playlist->songs->len; i++)
	{
		g_ptr_array_add(backend->queue, playlist->songs->pdata[i]);
	}

	backend->index = idx;
	backend_load_track(backend, backend->queue->pdata[idx]);
}

void backend_tick(OrpheusBackend* backend)
{
	bool track_ended = backend->queue->len > 0 &&
	    audio_is_empty(backend->player) && !audio_is_paused(backend->player);

	if (track_ended)
	{
		switch (backend->repeat)
		{
		case REPEAT_OFF:
			backend_next(backend);
			break;

		case REPEAT_QUEUE:
			bool last_track = backend->index + 1 == (int)backend->queue->len;
			backend->index = last_track ? 0 : backend->index + 1;
			backend_load_track(backend, backend->queue->pdata[backend->index]);
			break;

		case REPEAT_TRACK:
			backend_load_track(backend, backend->queue->pdata[backend->index]);
			break;
		}
	}
}

Song* song_from_path(OrpheusBackend* backend, const char* path)
{
	return g_hash_table_lookup(backend->song_paths, path);
}

void playlist_free(gpointer data)
{
	Playlist* playlist = data;
	g_free(playlist->name);
	g_ptr_array_free(playlist->songs, TRUE);
	g_free(playlist);
}

float backend_get_position_seconds(OrpheusBackend* backend)
{
	return audio_get_position_seconds(backend->player);
}

float backend_get_duration_seconds(OrpheusBackend* backend)
{
	if (backend->queue->len == 0)
	{
		return 0.0F;
	}

	Song* current_song = (Song*)backend->queue->pdata[backend->index];
	return audio_get_duration_seconds(backend->player, current_song->path);
}

void backend_seek(OrpheusBackend* backend, float seconds)
{
	audio_seek_seconds(backend->player, seconds);
}

void backend_toggle_repeat(OrpheusBackend* backend)
{
	switch (backend->repeat)
	{
	case REPEAT_OFF:
		backend->repeat = REPEAT_QUEUE;
		break;

	case REPEAT_QUEUE:
		backend->repeat = REPEAT_TRACK;
		break;

	case REPEAT_TRACK:
		backend->repeat = REPEAT_OFF;
		break;
	}
}
