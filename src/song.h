#ifndef SONG_H
#define SONG_H

#include "backend.h"

Song* song_create(char* path);
void song_free(Song* song);

#endif
