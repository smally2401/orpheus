#include "audio.h"

#include "util.h"

#include "miniaudio.h"

#include <stdlib.h>

static struct
{
    ma_engine engine;
    bool engine_ok;
    ma_sound sound;
    bool loaded;
    AudioState state;
    double duration;
    char* path;
    float volume;
} A = { 
    .volume = 1.0f,
};

bool audio_init(void)
{
    if (ma_engine_init(NULL, &A.engine) != MA_SUCCESS)
    {
        log_msg("audio: could not initialize playback device");
        return false;
    }

    A.engine_ok = true;
    ma_engine_set_volume(&A.engine, A.volume);
    return true;
}

void audio_unload(void)
{
    if (A.loaded)
    {
        ma_sound_uninit(&A.sound);
        A.loaded = false;
    }

    free(A.path);
    A.path = NULL;
    A.state = AUDIO_STOPPED;
    A.duration = 0;
}

void audio_shutdown(void)
{
    audio_unload();
    if (A.engine_ok)
    {
        ma_engine_uninit(&A.engine);
    }

    A.engine_ok = false;
}

bool audio_load(const char* path, const char** err)
{
    audio_unload();
    if (!A.engine_ok)
    {
        if (err)
        {
            *err = "no audio device";
        }

        return false;
    }

    ma_uint32 flags = MA_SOUND_FLAG_STREAM | MA_SOUND_FLAG_NO_SPATIALIZATION;

#ifdef _WIN32
    gunichar2 *wpath = g_utf8_to_utf16(path, -1, NULL, NULL, NULL);
    ma_result r = ma_sound_init_from_file_w(&A.engine, (const wchar_t *)wpath, flags, NULL, NULL, &A.sound);
    g_free(wpath);
#else
    ma_result r = ma_sound_init_from_file(&A.engine, path, flags, NULL, NULL, &A.sound);
#endif

    if (r != MA_SUCCESS)
    {
        if (err)
        {
            *err = ma_result_description(r);
        }

        return false;
    }

    A.loaded = true;
    A.path = xstrdup(path);
    float len = 0;
    if (ma_sound_get_length_in_seconds(&A.sound, &len) == MA_SUCCESS)
    {
        A.duration = len;
    }

    return true;
}

void audio_play(void)
{
    if (!A.loaded)
    {
        return;
    }

    if (ma_sound_at_end(&A.sound))
    {
        ma_sound_seek_to_pcm_frame(&A.sound, 0);
    }

    ma_sound_start(&A.sound);
    A.state = AUDIO_PLAYING;
}

void audio_pause(void)
{
    if (!A.loaded || A.state != AUDIO_PLAYING)
    {
        return;
    }

    ma_sound_stop(&A.sound);
    A.state = AUDIO_PAUSED;
}

void audio_stop(void)
{
    if (!A.loaded)
    {
        return;
    }

    ma_sound_stop(&A.sound);
    ma_sound_seek_to_pcm_frame(&A.sound, 0);
    A.state = AUDIO_STOPPED;
}

bool audio_seek(double seconds)
{
    if (!A.loaded)
    {
        return false;
    }

    if (seconds < 0)
    {
        seconds = 0;
    }

    if (A.duration > 0 && seconds > A.duration)
    {
        seconds = A.duration;
    }

    return ma_sound_seek_to_second(&A.sound, (float)seconds) == MA_SUCCESS;
}

double audio_position(void)
{
    float c = 0;
    if (!A.loaded || ma_sound_get_cursor_in_seconds(&A.sound, &c) != MA_SUCCESS)
    {
        return 0;
    }

    return c;
}

double audio_duration(void)
{
    return A.duration;
}

AudioState audio_state(void)
{
    return A.state;
}

const char* audio_path(void)
{
    return A.path;
}

float audio_volume(void)
{
    return A.volume;
}

void audio_set_volume(float v)
{
    if (v < 0)
    {
        v = 0;
    }

    if (v > 1)
    {
        v = 1;
    }

    A.volume = v;
    if (A.engine_ok)
    {
        ma_engine_set_volume(&A.engine, v);
    }
}

bool audio_poll_finished(void)
{
    if (A.loaded && A.state == AUDIO_PLAYING && ma_sound_at_end(&A.sound))
    {
        A.state = AUDIO_STOPPED;
        return true;
    }

    return false;
}
