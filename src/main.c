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
#include "song.h"
#include "utils/debug.h"
#include "audio.h"
#include "library.h"

typedef enum
{
	VIEW_LIBRARY,
	VIEW_ALBUM,
} CurrentView;

int main(void)
{
	OrpheusBackend* backend = backend_init("/home/smally/Music");
	Album* open_album = NULL;
	CurrentView current_view = VIEW_LIBRARY;

	bool res = SDL_Init(SDL_INIT_VIDEO);
	if (!res)
	{
		fprintf(stderr, "SDL_Init failed: %s", SDL_GetError());
		return 1;
	}

	int win_width = 800;
	int win_height = 450;
	SDL_Window* window = SDL_CreateWindow("orpheus", win_width, win_height, 0);
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
				SDL_GetWindowSizeInPixels(window, &win_width, &win_height);
			}

			nk_sdl_handle_event(context, &event);
		}

		nk_bool res =
		    nk_begin(context, "orpheus",
		             nk_rect(0, 0, (float)win_width, (float)win_height), 0);
		if (res)
		{
			nk_layout_row_static(context, 30, 100, 1);

			switch (current_view)
			{
			case VIEW_LIBRARY:
				// for (guint i = 0; i < albums->len; i++)
				// {
				// 	Album* album = albums->pdata[i];
				// 	if (nk_button_label(context, album->title))
				// 	{
				// 		open_album = album;
				// 		current_view = VIEW_ALBUM;
				// 	}
				// }

				break;

			case VIEW_ALBUM:
				for (guint i = 0; i < open_album->tracklist->len; i++)
				{
					Song* song = open_album->tracklist->pdata[i];
					if (nk_button_label(context, song->title))
					{
						// audio_load_file(player, song->path);
						// audio_play(player);
					}
				}

				if (nk_button_label(context, "GO BACK"))
				{
					current_view = VIEW_LIBRARY;
				}

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
