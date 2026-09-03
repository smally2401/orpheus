#include <SDL3/SDL.h>
#include <SDL3/SDL_error.h>
#include <SDL3/SDL_events.h>
#include <SDL3/SDL_init.h>
#include <SDL3/SDL_render.h>
#include "nuklear_config.h"
#include "../vendor/nuklear.h"
#include "../vendor/nuklear_sdl3_renderer.h"
#include <stdio.h>

#ifdef DEBUG
#define DEBUG_PRINT(...) printf(__VA_ARGS__)
#else
#define DEBUG_PRINT(...) ((void)0)
#endif

int main(void)
{
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
	struct nk_context* context = nk_sdl_init(window, renderer, nk_sdl_allocator());
	nk_sdl_font_stash_begin(context);
	nk_sdl_font_stash_end(context);

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

		nk_bool res =
		    nk_begin(context, "orpheus", nk_rect(0, 0, (float)win_width, (float)win_height), 0);
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
		nk_sdl_render(context, NK_ANTI_ALIASING_ON);
		SDL_RenderPresent(renderer);
	}

	nk_sdl_shutdown(context);
	SDL_DestroyRenderer(renderer);
	SDL_DestroyWindow(window);
	SDL_Quit();

	return 0;
}
