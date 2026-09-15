#include <SDL3/SDL_oldnames.h>
#include <SDL3/SDL_render.h>
#include <SDL3/SDL_video.h>
#include <stdbool.h>
#include <stdio.h>

#include <SDL3/SDL.h>
#include <SDL3_image/SDL_image.h>
#include <glib.h>

#include "config/config.h"
#include "impl/nuklear_config.h"
#include "../vendor/nuklear.h"
#include "../vendor/nuklear_sdl3_renderer.h"

#include "backend/backend.h"
#include "ui/ui_state.h"
#include "ui/ui_view.h"
#include "ui/position_slider.h"

int main(void)
{
	UiState ui_state = ui_state_init();

	OrpheusBackend* backend = backend_init("/home/smally/Music");
	load_lua(backend);

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
	SDL_SetRenderVSync(renderer, 1);

	struct nk_context* context =
	    nk_sdl_init(window, renderer, nk_sdl_allocator());
	nk_sdl_font_stash_begin(context);
	nk_sdl_font_stash_end(context);

	SDL_Texture* art_texture = NULL;

	int running = 1;
	SDL_Event event;

	while (running)
	{
		backend_tick(backend);
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
			display_current_view(backend, &ui_state, context);
			if (nk_button_label(context, "TOGGLE PLAY"))
			{
				backend_toggle_play(backend);
			}
			if (nk_button_label(context, "TOGGLE REPEAT"))
			{
				backend_toggle_repeat(backend);
				printf("%i\n", backend->repeat);
			}
			display_position_slider(backend, context);
		}

		nk_end(context);
		nk_input_end(context);

		SDL_SetRenderDrawColor(renderer, 30, 30, 30, 255);
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
