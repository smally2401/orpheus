#ifndef AUDIO_H
#define AUDIO_H

#include "../vendor/miniaudio.h"

#include <stdbool.h>

typedef struct
{
	ma_engine engine;
	ma_sound sound;
	bool has_sound;
	bool paused;
} AudioPlayer;

AudioPlayer* audio_create(void);
void audio_destroy(AudioPlayer* player);
bool audio_load_file(AudioPlayer* player, const char* path);
void audio_play(AudioPlayer* player);
void audio_pause(AudioPlayer* player);
bool audio_seek_seconds(AudioPlayer* player, float seconds);
float audio_get_position_seconds(AudioPlayer* player);
bool audio_is_empty(AudioPlayer* player);
bool audio_is_paused(AudioPlayer* player);
float audio_get_volume(AudioPlayer* player);
void audio_set_volume(AudioPlayer* player, float vol);

#endif
