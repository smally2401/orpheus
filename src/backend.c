#include "backend.h"

#include "audio.h"
#include "glib.h"
#include "library.h"

OrpheusBackend* backend_init(const char* music_dir)
{
	OrpheusBackend* backend = malloc(sizeof(OrpheusBackend));
	backend->player = audio_create();
	backend->song_paths = scan_library(music_dir);
	GHashTable* built_library = build_library(backend->song_paths);
	backend->library = sorted_albums(built_library);
	g_hash_table_destroy(built_library);
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
	g_ptr_array_free(backend->library, TRUE);
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

void backend_load_track(OrpheusBackend* backend, Song* song)
{
	if (audio_load_file(backend->player, song->path))
	{
		backend->paused = false;
	}
}
