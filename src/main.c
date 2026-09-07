#include <stdbool.h>
#include <stdio.h>

#include <SDL3/SDL.h>
#include <SDL3_image/SDL_image.h>

#include "library.h"
#include "nuklear_config.h"
#include "../vendor/nuklear.h"
#include "../vendor/nuklear_sdl3_renderer.h"

#include "backend.h"
#include "song.h"

int main(void)
{
	scan_library("/home/smally/Music");

	bool res = SDL_Init(SDL_INIT_VIDEO);
	if (!res)
	{
		fprintf(stderr, "SDL_Init failed: %s", SDL_GetError());
		return 1;
	}

	const int win_width = 800;
	const int win_height = 450;
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

	Song* test_song =
	    song_create("/home/smally/Music/Portishead/Dummy/mysterons.mp3");
	SDL_Texture* art_texture = NULL;
	if (test_song->art)
	{
		SDL_IOStream* io =
		    SDL_IOFromConstMem(test_song->art->data, test_song->art->size);
		SDL_Surface* surface = IMG_Load_IO(io, true);

		printf("width: %i | height: %i\n", surface->w, surface->h);

		if (!surface)
		{
			fprintf(stderr, "IMG_Load_IO failed: %s\n", SDL_GetError());
		}
		else
		{
			art_texture = SDL_CreateTextureFromSurface(renderer, surface);
			if (!art_texture)
			{
				fprintf(stderr, "SDL_CreateTextureFromSurface failed: %s\n",
				        SDL_GetError());
			}

			SDL_DestroySurface(surface);
		}
	}
	else
	{
		fprintf(stderr, "test_song has no art\n");
	}

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

			// DEBUG_PRINT("TYPE: %u\n", event.type);
			// DEBUG_PRINT("BUTTON: %u\n", event.button.button);

			nk_sdl_handle_event(context, &event);
		}

		nk_bool res = nk_begin(context, "orpheus", nk_rect(0, 0, 200, 100), 0);
		if (res)
		{
			nk_layout_row_static(context, 30, 100, 1);
			if (nk_button_label(context, "click me"))
			{
				puts("click!");
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

	song_free(test_song);
	nk_sdl_shutdown(context);
	SDL_DestroyRenderer(renderer);
	SDL_DestroyWindow(window);
	SDL_Quit();

	return 0;
}
