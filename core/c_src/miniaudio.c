// C-side implementation backing `MiniAudioPlayer` in `audio_player.rs`.
// Wraps a single `ma_engine` + `ma_sound` pair: only one sound is ever
// loaded at a time (see `miniaudio_load_file`), matching how
// `LocalBackend::load_track` uses it: always stop then append, never
// queuing multiple sounds concurrently.

#define MINIAUDIO_IMPLEMENTATION
#include "miniaudio.h"
#include <stdbool.h>
#include <stdlib.h>

typedef struct {
    ma_engine engine;
    ma_sound sound;
    // Only meaningful once a sound has been loaded via
    // `miniaudio_load_file`/`_win`, `sound` itself is otherwise
    // uninitialized.
    bool has_sound;
    // Mirrors "paused" as reported to Rust via `miniaudio_is_paused`.
    // Set by both `miniaudio_pause` and `miniaudio_stop`.
    bool paused;
} MiniAudioPlayerC;

// Allocates a player and initializes its `ma_engine` on the default
// audio device. Returns NULL on allocation failure or engine init
// failure (caller side asserts non-null, see `MiniAudioPlayer::new`).
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

// Tears down the current sound (if any) and the engine, then frees the
// player itself.
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

// Loads `path` as the player's active sound, replacing whatever was
// previously loaded (uninits the old `ma_sound` first, if any). Uses
// flag `0`, i.e. streaming decode rather than up-front full decode:
// loads are fast regardless of file length, at the cost of slower
// seeks on formats without a seek table (notably MP3, FLAC seeks are
// cheap since dr_flag can index via STREAMINFO). Tried full up-front
// decode (`MA_SOUND_FLAG_DECODE`) instead: it made every load
// noticeably slower in exchange for instant seeks, worse trade for how
// this app is used, reverted.
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

// Windows counterpart to `miniaudio_load_file`, taking a wide
// (UTF-16) path via `ma_sound_init_from_file_w`. Same streaming
// behaviour and semantics otherwise.
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

// Resumes/starts playback of the current sound from wherever its
// cursor currently is. No-op if nothing is loaded.
void miniaudio_play(MiniAudioPlayerC* player) {
    if (player && player->has_sound) {
        ma_sound_start(&player->sound);
        player->paused = false;
    }
}

// Halts playback in place: position is left untouched, so a
// subsequent `miniaudio_play` resumes from where it stopped. Distinct
// from `miniaudio_stop`, which additionally resets position to zero.
void miniaudio_pause(MiniAudioPlayerC* player) {
    if (player && player->has_sound) {
        ma_sound_stop(&player->sound);
        player->paused = true;
    }
}

// Halts playback and resets position to the start of the track. Used
// internally by `load_track` before swapping in a new sound.
void miniaudio_stop(MiniAudioPlayerC* player) {
    if (player && player->has_sound) {
        ma_sound_stop(&player->sound);
        ma_sound_seek_to_pcm_frame(&player->sound, 0);
        player->paused = true;
    }
}

// Seeks to an absolute position in seconds, converting to a PCM frame
// index using the engine's sample rate. Returns false if nothing is
// loaded or the underlying seek fails.
bool miniaudio_seek_seconds(MiniAudioPlayerC* player, double seconds) {
    if (!player || !player->has_sound) {
        return false;
    }

    ma_uint32 sample_rate = ma_engine_get_sample_rate(&player->engine);
    ma_uint64 frame = (ma_uint64)(seconds * (double)sample_rate);

    return ma_sound_seek_to_pcm_frame(&player->sound, frame) == MA_SUCCESS;
}

// Current playback position in seconds, derived from the sound's PCM
// cursor and the engine's sample rate. Returns 0.0 if nothing is
// loaded.
double miniaudio_get_position_seconds(MiniAudioPlayerC* player) {
    if (!player || !player->has_sound) {
        return 0.0;
    }

    ma_uint64 cursor = 0;
    ma_sound_get_cursor_in_pcm_frames(&player->sound, &cursor);
    ma_uint32 sample_rate = ma_engine_get_sample_rate(&player->engine);

    return (double)cursor / (double)sample_rate;
}

// True if the current sound has played to its end or if nothing is
// loaded at all.
bool miniaudio_is_empty(MiniAudioPlayerC* player) {
    if (!player || !player->has_sound) {
        return true;
    }

    return ma_sound_at_end(&player->sound) != 0;
}

// Reports the `paused` flag tracked on this struct. A NULL player
// counts as paused.
bool miniaudio_is_paused(MiniAudioPlayerC* player) {
    if (!player || player->paused) {
        return true;
    }

    return false;
}

// Engine-level volume (0.0-1.0).
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