#include "song.h"
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include "utils/debug.h"

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

static const char* get_extension(const char* path)
{
	const char* dot = strrchr(path, '.');
	if (!dot || strrchr(dot, '/') || strrchr(dot, '\\'))
	{
		return NULL;
	}

	return dot + 1;
}

static uint32_t synchsafe_to_int(unsigned const char* buf)
{
	return (buf[0] << 21) | (buf[1] << 14) | (buf[2] << 7) | (buf[3]);
}

static uint32_t read_u32_be(unsigned const char* buf)
{
	return (buf[0] << 24) | (buf[1] << 16) | (buf[2] << 8) | (buf[3]);
}

static char* decode_text_frame(unsigned const char* content, uint32_t len)
{
	if (len == 0)
	{
		return NULL;
	}

	if (content[0] == 0 || content[0] == 3)
	{
		char* frame = malloc(len);
		if (!frame)
		{
			return NULL;
		}

		memcpy(frame, content + 1, len - 1);
		frame[len - 1] = '\0';
		return frame;
	}

	// TODO: handle other encodings
	DEBUG_PRINT("UNHANDLED ENCODING MARKER: %u\n", content[0]);
	return NULL;
}

static int mp3_tags(Song* song)
{
	FILE* f = fopen(song->path, "rb");
	if (!f)
	{
		return -1;
	}

	unsigned char header[10];
	fread(header, 1, 10, f);

	if (memcmp(header, "ID3", 3) != 0)
	{
		fclose(f);
		return -1;
	}

	uint32_t size = synchsafe_to_int(&header[6]);
	unsigned char* tag_data = malloc(size);
	unsigned long n = fread(tag_data, 1, size, f);
	if (n < size)
	{
		free(tag_data);
		fclose(f);
		return -1;
	}

	unsigned char ver = header[3];
	uint32_t pos = 0;
	DEBUG_PRINT("\n\n");
	while (pos < size)
	{
		if (size - pos < 10)
		{
			break;
		}

		if (memcmp(&tag_data[pos], "\0\0\0\0", 4) == 0)
		{
			break;
		}

		uint32_t frame_size;

		if (ver == 4)
		{
			frame_size = synchsafe_to_int(&tag_data[pos + 4]);
		}
		else
		{
			frame_size = read_u32_be(&tag_data[pos + 4]);
		}

		if (memcmp(&tag_data[pos], "TIT2", 4) == 0)
		{
			char* text = decode_text_frame(&tag_data[pos + 10], frame_size);
			if (text)
			{
				free(song->title);
				song->title = text;
				DEBUG_PRINT("TITLE: %s\n", text);
			}
		}
		else if (memcmp(&tag_data[pos], "TPE1", 4) == 0)
		{
			char* text = decode_text_frame(&tag_data[pos + 10], frame_size);
			if (text)
			{
				free(song->artist);
				song->artist = text;
				DEBUG_PRINT("ARTIST: %s\n", text);
			}
		}
		else if (memcmp(&tag_data[pos], "TALB", 4) == 0)
		{
			char* text = decode_text_frame(&tag_data[pos + 10], frame_size);
			if (text)
			{
				free(song->album_title);
				song->album_title = text;
				DEBUG_PRINT("ALBUM: %s\n", text);
			}
		}
		else if (memcmp(&tag_data[pos], "TPE2", 4) == 0)
		{
			char* text = decode_text_frame(&tag_data[pos + 10], frame_size);
			if (text)
			{
				free(song->album_artist);
				song->album_artist = text;
				DEBUG_PRINT("ALBUM ARTIST: %s\n", text);
			}
		}
		else if (memcmp(&tag_data[pos], "TRCK", 4) == 0)
		{
			char* text = decode_text_frame(&tag_data[pos + 10], frame_size);
			if (text)
			{
				int track_num = (int)strtol(text, NULL, 10);
				free(text);
				song->number = track_num;
				DEBUG_PRINT("TRACK NUMBER: %i\n", track_num);
			}
		}
#ifdef DEBUG
		else
		{
			char* text = decode_text_frame(&tag_data[pos + 10], frame_size);
			if (text)
			{
				DEBUG_PRINT("OTHER: %.4s - %s\n", &tag_data[pos], text);
				free(text);
			}
		}
#endif

		if (pos + 10 + frame_size > size)
		{
			break;
		}

		pos += 10 + frame_size;
	}

	free(tag_data);
	fclose(f);
	return 0;
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
	song->path = path;
	song->number = -1;
	song->album_artist = strdup("Unknown Artist");
	song->artist = strdup("Unknown Artist");
	song->album_title = strdup("Unknown Album");
	song->title = strdup("Unknown Title");
	song->duration_secs = 0;
	song->art = NULL;

	song_from_id3v2(song);
	return song;
}

void song_free(Song* song)
{
	free(song->album_artist);
	free(song->artist);
	free(song->album_title);
	free(song->title);
	free(song);
	song = NULL;
}
