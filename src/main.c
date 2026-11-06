#include "app.h"
#include "util.h"
#include <SDL3/SDL_events.h>
#include <SDL3/SDL_init.h>
#include <SDL3/SDL_render.h>
#include <SDL3/SDL_video.h>

App app = {
    .fps = 60,
    .idle_fps = 10,
};

int main(void)
{
    if (!SDL_Init(SDL_INIT_VIDEO))
    {
        die("SDL_Init: %s", SDL_GetError());
    }

    if (!SDL_CreateWindowAndRenderer("orpheus", 960, 600, SDL_WINDOW_RESIZABLE, &app.window, &app.renderer))
    {
        die("could not create window: %s", SDL_GetError());
    }

    SDL_SetRenderVSync(app.renderer, 1);
    app.running = true;
    while (app.running)
    {
        SDL_Event e;
        while (SDL_PollEvent(&e))
        {
            if (e.type == SDL_EVENT_QUIT)
            {
                app.running = false;
            }
        }

        SDL_SetRenderDrawColor(app.renderer, 30, 30, 40, 255);
        SDL_RenderClear(app.renderer);
        SDL_RenderPresent(app.renderer);
    }

    SDL_DestroyRenderer(app.renderer);
    SDL_DestroyWindow(app.window);
    SDL_Quit();
    return 0;
}
