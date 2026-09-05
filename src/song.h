#ifndef SONG_H
#define SONG_H

#include "app_state.h"

Song* song_create(char* path);
void song_free(Song* song);

#endif
