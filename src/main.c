#include <SDL3/SDL_oldnames.h>
#include <SDL3/SDL_video.h>
#include <stdbool.h>
#include <stdio.h>

#include <SDL3/SDL.h>
#include <SDL3_image/SDL_image.h>
#include <glib.h>

#include "nuklear_config.h"
#include "../vendor/nuklear.h"
#include "../vendor/nuklear_sdl3_renderer.h"

#include "backend.h"
#include "ui/ui_state.h"
#include "config/config.h"

int main(void)
{
	load_lua();

	UiState ui_state;
	ui_state.win_width = 800;
	ui_state.win_height = 450;
	ui_state.current_view = VIEW_LIBRARY;
	ui_state.current_album = NULL;

	OrpheusBackend* backend = backend_init("/home/smally/Music");

	bool res = SDL_Init(SDL_INIT_VIDEO);
	if (!res)
	{
		fprintf(stderr, "SDL_Init failed: %s", SDL_GetError());
		return 1;
	}

	SDL_Window* window =
	    SDL_CreateWindow("orpheus", ui_state.win_width, ui_state.win_height, 0);
	if (!window)
	{
		fprintf(stderr, "SDL_CreateWindow failed: %s", SDL_GetError());
		return 1;
	}

	SDL_Renderer* renderer = SDL_CreateRenderer(window, NULL);
	struct nk_context* context =
	    nk_sdl_init(window, renderer, nk_sdl_allocator());
	nk_sdl_font_stash_begin(context);
	nk_sdl_font_stash_end(context);

	SDL_Texture* art_texture = NULL;

	int running = 1;
	SDL_Event event;

	while (running)
	{
		nk_input_begin(context);

		while (SDL_PollEvent(&event))
		{
			if (event.type == SDL_EVENT_QUIT)
			{
				running = 0;
			}

			if (event.type == SDL_EVENT_WINDOW_RESIZED)
			{
				SDL_GetWindowSizeInPixels(window, &ui_state.win_width,
				                          &ui_state.win_height);
			}

			nk_sdl_handle_event(context, &event);
		}

		nk_bool res = nk_begin(context, "orpheus",
		                       nk_rect(0, 0, (float)ui_state.win_width,
		                               (float)ui_state.win_height),
		                       0);
		if (res)
		{
			nk_layout_row_static(context, 30, 100, 1);

			switch (ui_state.current_view)
			{
			case VIEW_LIBRARY:

				for (guint i = 0; i < backend->library->len; i++)
				{
					Album* album = backend->library->pdata[i];
					if (nk_button_label(context, album->title))
					{
						ui_state.current_album = album;
						ui_state.current_view = VIEW_ALBUM;
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

				break;

			case VIEW_ALBUM:

				for (guint i = 0; i < ui_state.current_album->tracklist->len;
				     i++)
				{
					Song* song = ui_state.current_album->tracklist->pdata[i];
					if (nk_button_label(context, song->title))
					{
						backend_load_album_to_queue(
						    backend, ui_state.current_album, (int)i);
					}
				}

				if (nk_button_label(context, "GO BACK"))
				{
					ui_state.current_view = VIEW_LIBRARY;
				}

				break;

			case VIEW_PLAYLISTS:

				break;

			case VIEW_OPEN_PLAYLIST:

				break;
			}
		}

		nk_end(context);
		nk_input_end(context);

		const int red = 30;
		const int green = 30;
		const int blue = 30;
		const int alpha = 255;

		SDL_SetRenderDrawColor(renderer, red, green, blue, alpha);
		SDL_RenderClear(renderer);

		if (art_texture)
		{
			if (!SDL_RenderTexture(renderer, art_texture, NULL, NULL))
			{
				fprintf(stderr, "SDL_RenderTexture failed: %s\n",
				        SDL_GetError());
			}
		}

		nk_sdl_render(context, NK_ANTI_ALIASING_ON);
		SDL_RenderPresent(renderer);
	}

	if (art_texture)
	{
		SDL_DestroyTexture(art_texture);
	}

	backend_destroy(backend);
	nk_sdl_shutdown(context);
	SDL_DestroyRenderer(renderer);
	SDL_DestroyWindow(window);
	SDL_Quit();

	return 0;
}
