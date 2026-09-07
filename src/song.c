#include "song.h"

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "backend.h"
#include "decode.h"
#include "utils/debug.h"
#include "utils/utils.h"

typedef enum
{
	EXT_MP3,
	EXT_FLAC,
	EXT_INVALID,
} SongExtension;

static SongExtension get_extension_enum(const char* ext)
{
	if (strcmp(ext, "mp3") == 0)
	{
		return EXT_MP3;
	}
	if (strcmp(ext, "flac") == 0)
	{
		return EXT_FLAC;
	}

	return EXT_INVALID;
}

static int song_from_id3v2(Song* song)
{
	char* path = song->path;
	const char* ext = get_extension(path);

	switch (get_extension_enum(ext))
	{
	case (EXT_MP3):
		mp3_tags(song);
		break;

	case (EXT_FLAC):
		// flac tags
		break;

	case (EXT_INVALID):
		DEBUG_PRINT("invalid extension in song_from_id3v2: %s\n", ext);
		return -1;
	}

	return 0;
}

Song* song_create(char* path)
{
	Song* song = malloc(sizeof(Song));
	song->path = orph_strdup(path);
	song->number = -1;
	song->album_artist = orph_strdup("Unknown Artist");
	song->artist = orph_strdup("Unknown Artist");
	song->album_title = orph_strdup("Unknown Album");
	song->title = orph_strdup("Unknown Title");
	song->duration_secs = 0;
	song->art = NULL;

	song_from_id3v2(song);
	return song;
}

void song_free(Song* song)
{
	free(song->path);
	free(song->album_artist);
	free(song->artist);
	free(song->album_title);
	free(song->title);

	if (song->art)
	{
		free(song->art->data);
		free(song->art);
	}

	free(song);
}
