#include "decode.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "utils/debug.h"

static uint32_t synchsafe_to_int(unsigned const char* buf)
{
	return (buf[0] << 21) | (buf[1] << 14) | (buf[2] << 7) | (buf[3]);
}

static uint32_t read_u32_be(unsigned const char* buf)
{
	return (buf[0] << 24) | (buf[1] << 16) | (buf[2] << 8) | (buf[3]);
}

// ID3v2 text frame content starts with a 1-byte encoding marker (0 = Latin1,
// 1/2 = UTF-16, 3 =  UTF-8), followed by the non-null-terminated string. Only
// Latin1/UTF-8 are handled for now, UTF-16 returns NULL, same as an empty
// frame.
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
	// DEBUG_PRINT("UNHANDLED ENCODING MARKER: %u\n", content[0]);
	return NULL;
}

static void assign_text_field(char** field, const unsigned char* frame_data,
                              uint32_t size)
{
	char* text = decode_text_frame(frame_data, size);
	if (!text)
	{
		return;
	}

	free(*field);
	*field = text;
	// DEBUG_PRINT("%s\n", text);
}

// APIC content, in order: 1-byte encoding, null-terminated MOME string, 1-byte
// picture type (0x03 = front cover), null-terminated description, then raw
// image bytes to the end of the frame. A file can have multiple APIC frames:
// front cover always wins, otherwise the first one found is kept.
static void process_apic_frame(Song* song, const unsigned char* content,
                               uint32_t frame_size)
{
	size_t mime_len = strlen((char*)&content[1]);
	size_t offset = 1 + mime_len + 1;
	unsigned char pic_type = content[offset++];

	size_t desc_len = strlen((char*)&content[offset]);
	offset += desc_len + 1;

	size_t image_size = frame_size - offset;
	const unsigned char* image_data = &content[offset];

	if (pic_type == 0x03 || song->art == NULL)
	{
		uint8_t* art_copy = malloc(image_size);
		if (art_copy)
		{
			memcpy(art_copy, image_data, image_size);
			if (!song->art)
			{
				song->art = malloc(sizeof(AlbumArt));
			}
			else
			{
				free(song->art->data);
			}

			song->art->data = art_copy;
			song->art->size = image_size;
		}
	}
}

static void process_frame(Song* song, const unsigned char* frame,
                          uint32_t frame_size)
{
	const unsigned char* content = frame + 10;

	if (memcmp(frame, "TIT2", 4) == 0)
	{
		assign_text_field(&song->title, content, frame_size);
	}
	else if (memcmp(frame, "TPE1", 4) == 0)
	{
		assign_text_field(&song->artist, content, frame_size);
	}
	else if (memcmp(frame, "TALB", 4) == 0)
	{
		assign_text_field(&song->album_title, content, frame_size);
	}
	else if (memcmp(frame, "TPE2", 4) == 0)
	{
		assign_text_field(&song->album_artist, content, frame_size);
	}
	else if (memcmp(frame, "TRCK", 4) == 0)
	{
		char* text = decode_text_frame(content, frame_size);
		if (text)
		{
			int track_num = (int)strtol(text, NULL, 10);
			free(text);
			song->number = track_num;
			// DEBUG_PRINT("TRACK NUMBER: %i\n", track_num);
		}
	}
	else if (memcmp(frame, "APIC", 4) == 0)
	{
		process_apic_frame(song, content, frame_size);
	}
#ifdef DEBUG
	else
	{
		char* text = decode_text_frame(content, frame_size);
		if (text)
		{
			// DEBUG_PRINT("%.4s: %s\n", frame, text);
			free(text);
		}
	}
#endif
}

// Reads the ID3v2 tag at the start of an MP3 file, if present: a 10-byte header
// (with a synchsafe-encoded total size) followed by a sequence of frames, each
// with its own 4-byte ID + size + flags header. Walks that sequence, handing
// each frame to process_frame, until the tag's declared size is exhausted or a
// padding frame (all-zero ID) is hit.
int mp3_tags(Song* song)
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
	// DEBUG_PRINT("\n\n");
	while (pos + 10 < size)
	{
		if (memcmp(&tag_data[pos], "\0\0\0\0", 4) == 0)
		{
			break;
		}

		uint32_t frame_size = (ver == 4) ? synchsafe_to_int(&tag_data[pos + 4])
		                                 : read_u32_be(&tag_data[pos + 4]);

		if (pos + 10 + frame_size > size)
		{
			break;
		}

		process_frame(song, &tag_data[pos], frame_size);
		pos += 10 + frame_size;
	}

	free(tag_data);
	fclose(f);
	return 0;
}
