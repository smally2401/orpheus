#include "api.h"
#include "app.h"
#include "audio.h"
#include "util.h"
#include "script.h"

#include <SDL3/SDL_events.h>
#include <SDL3/SDL_keyboard.h>
#include <SDL3/SDL_keycode.h>
#include <SDL3/SDL_video.h>
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

#define ACTIVE_NS (750 * SDL_NS_PER_MS)

static void usage(void)
{
    puts(
        "usage: orpheus [--config FILE] [--runtime DIR] [FILE|DIR...]\n"
        "  --config FILE   use FILE instead of $XDG_CONFIG_HOME/orpheus/init.lua\n"
        "  --runtime DIR   load runtime modules from DIR before the embedded copy\n"
        "  --version       print the version"
    );
}

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

static void run(void)
{
    Uint64 last = SDL_GetTicksNS();
    Uint64 last_input = last;
    while (app.running)
    {
        bool active = SDL_GetTicksNS() - last_input < ACTIVE_NS;
        Uint64 frame_ns = SDL_NS_PER_SECOND / (Uint64)(active ? app.fps : app.idle_fps);
        SDL_WindowFlags wf = SDL_GetWindowFlags(app.window);
        bool hidden = (wf & (
            SDL_WINDOW_MINIMIZED |
            SDL_WINDOW_HIDDEN |
            SDL_WINDOW_OCCLUDED)) != 0;

        if (hidden)
        {
            frame_ns = SDL_NS_PER_SECOND / 4;
        }

        Uint64 now = SDL_GetTicksNS();
        Uint64 wait = last + frame_ns > now ? last + frame_ns - now : 0;
        SDL_Event e;
        bool got = SDL_WaitEventTimeout(&e, (Sint32)(wait / SDL_NS_PER_MS));
        while (got)
        {
            dispatch(&e);
            if (e.type != SDL_EVENT_POLL_SENTINEL)
            {
                if (!active)
                {
                    last = 0;
                }

                last_input = SDL_GetTicksNS();
            }

            got = app.running && SDL_PollEvent(&e);
        }

        while(g_main_context_iteration(NULL, FALSE))
        {
        }

        if (audio_poll_finished())
        {
            script_event("track_end", "");
        }

        now = SDL_GetTicksNS();
        if (now - last < frame_ns)
        {
            continue;
        }

        double dt = (double)(now - last) / 1e9;
        last = now;
        if (!hidden)
        {
            SDL_SetRenderDrawColor(app.renderer, 0, 0, 0, 255);
            SDL_RenderClear(app.renderer);
            api_gfx_begin_frame();
        }

        script_event("frame", "nb", dt, !hidden);
        if (!hidden)
        {
            SDL_RenderPresent(app.renderer);
        }
    }
}

int main(int argc, char** argv)
{
    char** files = xcalloc((size_t)argc, sizeof *files);
    int nfiles = 0;
    for (int i = 1; i < argc; i++)
    {
        if (!strcmp(argv[i], "--config") && i + 1 < argc)
        {
            i++;
            app.config_file = argv[i];
        }
        else if (!strcmp(argv[i], "-h") || !strcmp(argv[i], "--help"))
        {
            usage();
            return 0;
        }
        else if (!strcmp(argv[i], "--version"))
        {
            puts("orpheus " ORPHEUS_VERSION);
            return 0;
        }
        else if (!strcmp(argv[i], "--"))
        {
            while (++i < argc)
            {
                files[nfiles] = argv[i];
                nfiles++;
            }
        }
        else if (argv[i][0] == '-' && argv[i][1])
        {
            fprintf(stderr, "orpheus: unknown option '%s'\n", argv[i]);
            usage();
            return 1;
        }
        else
        {
            files[nfiles] = argv[i];
            nfiles++;
        }
    }

    SDL_SetHint(SDL_HINT_APP_NAME, "orpheus");
    if (!SDL_Init(SDL_INIT_VIDEO))
    {
        die("SDL_Init: %s", SDL_GetError());
    }

    if (!SDL_CreateWindowAndRenderer("orpheus", 960, 600, SDL_WINDOW_RESIZABLE, &app.window, &app.renderer))
    {
        die("could not create window: %s", SDL_GetError());
    }

    SDL_SetRenderVSync(app.renderer, 1);
    SDL_SetRenderDrawBlendMode(app.renderer, SDL_BLENDMODE_BLEND);
    SDL_SetWindowMinimumSize(app.window, 320, 200);
    SDL_StopTextInput(app.window);

    audio_init();
    app.running = true;
    script_init(nfiles, files);
    free(files);

    run();

    script_shutdown();
    audio_shutdown();
    SDL_DestroyRenderer(app.renderer);
    SDL_DestroyWindow(app.window);
    SDL_Quit();

    return app.exit_code;
}
