#define MINIAUDIO_IMPLEMENTATION
#include "miniaudio.h"
#include <stdbool.h>
#include <stdlib.h>

typedef struct {
    ma_engine engine;
    ma_sound sound;
    bool has_sound;
    bool paused;
} MiniAudioPlayerC;

MiniAudioPlayerC* miniaudio_create(void) {
    MiniAudioPlayerC* player = (MiniAudioPlayerC*)calloc(1, sizeof(MiniAudioPlayerC));

    if (!player) {
        return NULL;
    }

    if (ma_engine_init(NULL, &player->engine) != MA_SUCCESS) {
        free(player);
        return NULL;
    }

    return player;
}

void miniaudio_destroy(MiniAudioPlayerC* player) {
    if (!player) {
        return;
    }

    if (player->has_sound) {
        ma_sound_uninit(&player->sound);
    }

    ma_engine_uninit(&player->engine);
    free(player);
}

bool miniaudio_load_file(MiniAudioPlayerC* player, const char* path) {
    if (!player) {
        return false;
    }

    if (player->has_sound) {
        ma_sound_uninit(&player->sound);
        player->has_sound = false;
    }

    if (ma_sound_init_from_file(&player->engine, path, 0, NULL, NULL, &player->sound) == MA_SUCCESS) {
        player->has_sound = true;
        player->paused = false;
        return true;
    }

    return false;
}

bool miniaudio_load_file_win(MiniAudioPlayerC* player, const wchar_t* path) {
    if (!player) {
        return false;
    }

    if (player->has_sound) {
        ma_sound_uninit(&player->sound);
        player->has_sound = false;
    }

    if (ma_sound_init_from_file_w(&player->engine, path, 0, NULL, NULL, &player->sound) == MA_SUCCESS) {
        player->has_sound = true;
        player->paused = false;
        return true;
    }

    return false;
}

void miniaudio_play(MiniAudioPlayerC* player) {
    if (player && player->has_sound) {
        ma_sound_start(&player->sound);
        player->paused = false;
    }
}

void miniaudio_pause(MiniAudioPlayerC* player) {
    if (player && player->has_sound) {
        ma_sound_stop(&player->sound);
        player->paused = true;
    }
}

bool miniaudio_seek_seconds(MiniAudioPlayerC* player, double seconds) {
    if (!player || !player->has_sound) {
        return false;
    }

    ma_uint32 sample_rate = ma_engine_get_sample_rate(&player->engine);
    ma_uint64 frame = (ma_uint64)(seconds * (double)sample_rate);

    return ma_sound_seek_to_pcm_frame(&player->sound, frame) == MA_SUCCESS;
}

double miniaudio_get_position_seconds(MiniAudioPlayerC* player) {
    if (!player || !player->has_sound) {
        return 0.0;
    }

    ma_uint64 cursor = 0;
    ma_sound_get_cursor_in_pcm_frames(&player->sound, &cursor);
    ma_uint32 sample_rate = ma_engine_get_sample_rate(&player->engine);

    return (double)cursor / (double)sample_rate;
}

bool miniaudio_is_empty(MiniAudioPlayerC* player) {
    if (!player || !player->has_sound) {
        return true;
    }

    return ma_sound_at_end(&player->sound) != 0;
}

bool miniaudio_is_paused(MiniAudioPlayerC* player) {
    if (!player || player->paused) {
        return true;
    }

    return false;
}

float miniaudio_volume(MiniAudioPlayerC* player) {
    if (!player) {
        return 0.0;
    }

    return ma_engine_get_volume(&player->engine);
}

void miniaudio_set_volume(MiniAudioPlayerC* player, float volume) {
    if (!player) {
        return;
    }

    ma_engine_set_volume(&player->engine, volume);
}