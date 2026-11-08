#include "app.h"
#include "util.h"
#include "script.h"
#include <SDL3/SDL_stdinc.h>
#include <SDL3/SDL_timer.h>

App app = {
    .fps = 60,
    .idle_fps = 10,
    .runtime_dir = "runtime",
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
    script_init(0, NULL);
    app.running = true;
    Uint64 last = SDL_GetTicksNS();
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

        Uint64 now = SDL_GetTicksNS();
        double dt = (double)(now - last) / 1e9;
        last = now;
        script_event("frame", "nb", dt, 1);

        SDL_SetRenderDrawColor(app.renderer, 30, 30, 40, 255);
        SDL_RenderClear(app.renderer);
        SDL_RenderPresent(app.renderer);
    }

    script_shutdown();
    SDL_DestroyRenderer(app.renderer);
    SDL_DestroyWindow(app.window);
    SDL_Quit();
    return app.exit_code;
}
