#include "ui_state.h"

UiState ui_state_init(void)
{
	return (UiState){
	    .win_width = 800,
	    .win_height = 450,
	    .current_view = VIEW_LIBRARY,
	    .current_album = NULL,
	    .current_playlist = NULL,
	};
}
