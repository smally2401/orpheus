#include "backend.h"

#include "audio.h"
#include "glib.h"
#include "library.h"

OrpheusBackend* backend_init(const char* music_dir)
{
	OrpheusBackend* backend = malloc(sizeof(OrpheusBackend));
	backend->player = audio_create();
	backend->song_paths = scan_library(music_dir);
	backend->built_library = build_library(backend->song_paths);
	backend->library = sorted_albums(backend->built_library);
	backend->playlists = g_ptr_array_new();
	backend->queue = g_ptr_array_new();
	backend->index = 0;
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
	free(backend);
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

void backend_tick(OrpheusBackend* backend)
{
	if (audio_is_empty(backend->player) && !audio_is_paused(backend->player))
	{
	}
}
