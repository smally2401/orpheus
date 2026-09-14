#ifndef UI_STATE_H
#define UI_STATE_H

#include "../backend/backend.h"

typedef enum
{
	VIEW_LIBRARY,
	VIEW_ALBUM,
	VIEW_PLAYLISTS,
	VIEW_OPEN_PLAYLIST,
} CurrentView;

typedef struct UiState
{
	CurrentView current_view;
	Album* current_album;
	Playlist* current_playlist;
	int win_width;
	int win_height;
} UiState;

UiState ui_state_init(void);

#endif
