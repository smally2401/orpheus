#include "debug.h"

#include <stdio.h>

#include <glib.h>

#include "../library.h"
#include "../backend.h"
#include "../audio.h"

void print_library(GHashTable* library)
{
	if (!library)
	{
		fputs("no library to print\n", stderr);
		return;
	}

	GHashTableIter iter;
	gpointer key;
	gpointer value;
	g_hash_table_iter_init(&iter, library);

	while (g_hash_table_iter_next(&iter, &key, &value))
	{
		AlbumKey* album_key = (AlbumKey*)key;
		Album* album = (Album*)value;

		if (strcmp(album_key->album_artist, "Soda Stereo") == 0)
		{
			puts(album_key->album_title);

			for (guint i = 0; i < album->tracklist->len; i++)
			{
				Song* song = album->tracklist->pdata[i];
				puts(song->title);
			}
		}
	}
}

void print_albums(GPtrArray* albums)
{
	for (guint i = 0; i < albums->len; i++)
	{
		Album* album = (Album*)albums->pdata[i];
		puts(album->title);
	}
}

AudioPlayer* test_audio(void)
{
	AudioPlayer* player = audio_create();
	if (!player)
	{
		puts("could not create player");
		return NULL;
	}

	audio_set_volume(player, 1.0F);

	if (!audio_load_file(player,
	                     "/home/smally/Music/john-coltrane/a-love-supreme/01 "
	                     "Acknowledgement.mp3"))
	{
		puts("could not play file");
		return NULL;
	}

	audio_play(player);
	return player;
}
