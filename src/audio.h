#ifndef ORPHEUS_AUDIO_H
#define ORPHEUS_AUDIO_H

#include <stdbool.h>

typedef enum
{
    AUDIO_STOPPED,
    AUDIO_PLAYING,
    AUDIO_PAUSED,
} AudioState;

bool audio_init(void);
void audio_shutdown(void);

bool audio_load(const char* path, const char** err);
void audio_unload(void);

void audio_play(void);
void audio_pause(void);
void audio_stop(void);
bool audio_seek(double seconds);

double audio_position(void);
double audio_duration(void);
void audio_set_volume(float v);
float audio_volume(void);
AudioState audio_state(void);
const char* audio_path(void);

bool audio_poll_finished(void);

#endif
