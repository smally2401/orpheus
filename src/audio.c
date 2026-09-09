#include "audio.h"

#define MINIAUDIO_IMPLEMENTATION
#include "../vendor/miniaudio.h"

#include <stdlib.h>

#ifdef _WIN32
#define ma_sound_init_from_path ma_sound_init_from_file_w
#else
#define ma_sound_init_from_path ma_sound_init_from_file
#endif

AudioPlayer* audio_create(void)
{
	AudioPlayer* player = (AudioPlayer*)calloc(1, sizeof(AudioPlayer));
	if (!player)
	{
		return NULL;
	}

	ma_result res = ma_engine_init(NULL, &player->engine);
	if (res != MA_SUCCESS)
	{
		free(player);
		return NULL;
	}

	return player;
}

void audio_destroy(AudioPlayer* player)
{
	if (!player)
	{
		return;
	}

	if (player->has_sound)
	{
		ma_sound_uninit(&player->sound);
	}

	ma_engine_uninit(&player->engine);
	free(player);
}

bool audio_load_file(AudioPlayer* player, const path_string path)
{
	if (!player)
	{
		return false;
	}

	if (player->has_sound)
	{
		ma_sound_uninit(&player->sound);
		player->has_sound = false;
	}

	ma_result res = ma_sound_init_from_path(&player->engine, path, 0, NULL,
	                                        NULL, &player->sound);
	if (res == MA_SUCCESS)
	{
		player->has_sound = true;
		player->paused = false;
		return true;
	}

	return false;
}

void audio_play(AudioPlayer* player)
{
	if (player && player->has_sound)
	{
		ma_sound_start(&player->sound);
		player->paused = false;
	}
}

void audio_pause(AudioPlayer* player)
{
	if (player && player->has_sound)
	{
		ma_sound_stop(&player->sound);
		player->paused = true;
	}
}

static void __attribute__((unused)) audio_stop(AudioPlayer* player)
{
	if (player && player->has_sound)
	{
		ma_sound_stop(&player->sound);
		ma_sound_seek_to_pcm_frame(&player->sound, 0);
		player->paused = true;
	}
}

bool audio_seek_seconds(AudioPlayer* player, float seconds)
{
	if (!player || !player->has_sound)
	{
		return false;
	}

	ma_uint32 sample_rate = ma_engine_get_sample_rate(&player->engine);
	ma_uint64 frame = (ma_uint64)(seconds * (float)sample_rate);

	ma_result res = ma_sound_seek_to_pcm_frame(&player->sound, frame);
	return res == MA_SUCCESS ? true : false;
}

float audio_get_position_seconds(AudioPlayer* player)
{
	if (!player || !player->has_sound)
	{
		return 0.0F;
	}

	ma_uint64 cursor = 0;
	ma_sound_get_cursor_in_pcm_frames(&player->sound, &cursor);
	ma_uint32 sample_rate = ma_engine_get_sample_rate(&player->engine);

	return (float)cursor / (float)sample_rate;
}

bool audio_is_empty(AudioPlayer* player)
{
	if (!player || !player->has_sound)
	{
		return true;
	}

	return ma_sound_at_end(&player->sound) != 0;
}

bool audio_is_paused(AudioPlayer* player)
{
	if (!player || player->paused)
	{
		return true;
	}

	return false;
}

float audio_get_volume(AudioPlayer* player)
{
	if (!player)
	{
		return 0.0F;
	}

	return ma_engine_get_volume(&player->engine);
}

void audio_set_volume(AudioPlayer* player, float vol)
{
	if (!player)
	{
		return;
	}

	ma_engine_set_volume(&player->engine, vol);
}
