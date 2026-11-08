#include "api.h"
#include "app.h"
#include "util.h"
#include "script.h"

#include <SDL3/SDL_events.h>
#include <SDL3/SDL_keyboard.h>
#include <SDL3/SDL_keycode.h>
#include <glib.h>
#include <SDL3/SDL_blendmode.h>
#include <SDL3/SDL_render.h>
#include <SDL3/SDL_stdinc.h>
#include <SDL3/SDL_timer.h>

#include <string.h>

App app = {
    .fps = 60,
    .idle_fps = 10,
    .runtime_dir = "runtime",
};

static void mods_string(SDL_Keymod m, char* buf, size_t n)
{
    buf[0] = '\0';
    if (m & SDL_KMOD_CTRL)
    {
        g_strlcat(buf, "ctrl+", n);
    }
    if (m & SDL_KMOD_ALT)
    {
        g_strlcat(buf, "alt+", n);
    }
    if (m & SDL_KMOD_SHIFT)
    {
        g_strlcat(buf, "shift+", n);
    }
    if (m & SDL_KMOD_GUI)
    {
        g_strlcat(buf, "gui+", n);
    }

    size_t l = strlen(buf);
    if (l)
    {
        buf[l - 1] = '\0';
    }
}

static void dispatch(const SDL_Event* e)
{
    char mods[32];
    char key[64];
    switch (e->type)
    {
    case SDL_EVENT_QUIT:
        app.running = false;
        break;

    case SDL_EVENT_MOUSE_MOTION:
        script_event("mouse_move", "nn",
            (double)e->motion.x,
            (double)e->motion.y
        );
        break;

    case SDL_EVENT_MOUSE_BUTTON_DOWN:
    case SDL_EVENT_MOUSE_BUTTON_UP:
        script_event(e->type == SDL_EVENT_MOUSE_BUTTON_DOWN ? "mouse_down" : "mouse_up", "nnii",
            (double)e->button.x,
            (double)e->button.y,
            (int)e->button.button,
            (int)e->button.clicks
        );
        break;

    case SDL_EVENT_MOUSE_WHEEL:
        script_event("wheel", "nnnn",
            (double)e->wheel.x,
            (double)e->wheel.y,
            (double)e->wheel.mouse_x,
            (double)e->wheel.mouse_y
        );
        break;

    case SDL_EVENT_KEY_DOWN:
    case SDL_EVENT_KEY_UP:
    {
        const char* name = SDL_GetKeyName(e->key.key);
        if (!name || !*name)
        {
            break;
        }

        size_t i = 0;
        for (; name[i] && i < sizeof key - 1; i++)
        {
            key[i] = (char)g_ascii_tolower(name[i]);
        }

        key[i] = 0;
        mods_string(e->key.mod, mods, sizeof mods);
        script_event(e->type == SDL_EVENT_KEY_DOWN ? "key_down" : "key_up", "ssb",
            key,
            mods,
            (int)e->key.repeat
        );
        break;
    }

    case SDL_EVENT_TEXT_INPUT:
        script_event("text_input", "s", e->text.text);
        break;

    case SDL_EVENT_WINDOW_RESIZED:
        script_event("resize", "ii", 
            (int)e->window.data1,
            (int)e->window.data2
        );
        break;
        
    case SDL_EVENT_WINDOW_FOCUS_GAINED:
    case SDL_EVENT_WINDOW_FOCUS_LOST:
        script_event("focus", "b", e->type == SDL_EVENT_WINDOW_RESIZED);
        break;

    case SDL_EVENT_WINDOW_MOUSE_LEAVE:
        script_event("mouse_leave", "");
        break;

    case SDL_EVENT_DROP_FILE:
        script_event("drop", "s", e->drop.data);
        break;

    default:
        break;
    }
}

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

    SDL_SetRenderDrawBlendMode(app.renderer, SDL_BLENDMODE_BLEND);
    SDL_SetRenderVSync(app.renderer, 1);
    script_init(0, NULL);
    app.running = true;
    Uint64 last = SDL_GetTicksNS();
    while (app.running)
    {
        SDL_Event e;
        while (SDL_PollEvent(&e))
        {
            dispatch(&e);
        }

        Uint64 now = SDL_GetTicksNS();
        double dt = (double)(now - last) / 1e9;
        last = now;

        SDL_SetRenderDrawColor(app.renderer, 0, 0, 0, 255);
        SDL_RenderClear(app.renderer);
        api_gfx_begin_frame();
        script_event("frame", "nb", dt, 1);
        SDL_RenderPresent(app.renderer);
    }

    script_shutdown();
    SDL_DestroyRenderer(app.renderer);
    SDL_DestroyWindow(app.window);
    SDL_Quit();
    return app.exit_code;
}
