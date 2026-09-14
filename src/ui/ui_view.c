#include "ui_view.h"
#include "ui_state.h"

static void display_library_view(OrpheusBackend* backend, UiState* state,
                                 struct nk_context* context)
{
	for (guint i = 0; i < backend->library->len; i++)
	{
		Album* album = backend->library->pdata[i];
		if (nk_button_label(context, album->title))
		{
			state->current_album = album;
			state->current_view = VIEW_ALBUM;
		}
	}

	if (nk_button_label(context, "PREV"))
	{
		backend_prev(backend);
	}

	if (nk_button_label(context, "NEXT"))
	{
		backend_next(backend);
	}

	if (nk_button_label(context, "PLAYLISTS"))
	{
		state->current_view = VIEW_PLAYLISTS;
	}
}

static void display_album_view(OrpheusBackend* backend, UiState* state,
                               struct nk_context* context)
{
	for (guint i = 0; i < state->current_album->tracklist->len; i++)
	{
		Song* song = state->current_album->tracklist->pdata[i];
		if (nk_button_label(context, song->title))
		{
			backend_load_album_to_queue(backend, state->current_album, (int)i);
		}
	}

	if (nk_button_label(context, "GO BACK"))
	{
		state->current_view = VIEW_LIBRARY;
	}
}

static void display_playlists_view(OrpheusBackend* backend, UiState* state,
                                   struct nk_context* context)
{
	for (guint i = 0; i < backend->playlists->len; i++)
	{
		Playlist* playlist = backend->playlists->pdata[i];
		if (nk_button_label(context, playlist->name))
		{
			state->current_playlist = playlist;
			state->current_view = VIEW_OPEN_PLAYLIST;
		}
	}
}

static void display_open_playlist_view(OrpheusBackend* backend, UiState* state,
                                       struct nk_context* context)
{
	for (guint i = 0; i < state->current_playlist->songs->len; i++)
	{
		Song* song = state->current_playlist->songs->pdata[i];
		if (nk_button_label(context, song->title))
		{
			backend_load_playlist_to_queue(backend, state->current_playlist,
			                               (int)i);
		}
	}
}

void display_current_view(OrpheusBackend* backend, UiState* state,
                          struct nk_context* context)
{
	CurrentView view = state->current_view;

	switch (view)
	{
	case VIEW_LIBRARY:
		display_library_view(backend, state, context);
		break;

	case VIEW_ALBUM:
		display_album_view(backend, state, context);
		break;

	case VIEW_PLAYLISTS:
		display_playlists_view(backend, state, context);
		break;

	case VIEW_OPEN_PLAYLIST:
		display_open_playlist_view(backend, state, context);
		break;
	}
}
