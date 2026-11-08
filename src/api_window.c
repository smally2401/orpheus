#include "api.h"

#include "app.h"
#include "lauxlib.h"
#include "lua.h"
#include <SDL3/SDL_clipboard.h>
#include <SDL3/SDL_keyboard.h>
#include <SDL3/SDL_mouse.h>
#include <SDL3/SDL_render.h>
#include <SDL3/SDL_stdinc.h>
#include <SDL3/SDL_surface.h>
#include <SDL3/SDL_video.h>

static int l_size(lua_State* L)
{
    int w;
    int h;
    SDL_GetWindowSize(app.window, &w, &h);
    lua_pushinteger(L, w);
    lua_pushinteger(L, h);
    return 2;
}

static int l_set_size(lua_State* L)
{
    SDL_SetWindowSize(app.window, (int)luaL_checkinteger(L, 1), (int)luaL_checkinteger(L, 2));
    return 0;
}

static int l_set_min_size(lua_State* L)
{
    SDL_SetWindowMinimumSize(app.window, (int)luaL_checkinteger(L, 1), (int)luaL_checkinteger(L, 2));
    return 0;
}

static int l_set_title(lua_State* L)
{
    SDL_SetWindowTitle(app.window, luaL_checkstring(L, 1));
    return 0;
}

static int l_fullscreen(lua_State* L)
{
    if (!lua_isnone(L, 1))
    {
        SDL_SetWindowFullscreen(app.window, lua_toboolean(L, 1));
    }

    lua_pushboolean(L, (SDL_GetWindowFlags(app.window) & SDL_WINDOW_FULLSCREEN) != 0);
    return 1;
}

static int clamp_fps(lua_Integer fps)
{
    return fps < 1 ? 1 : fps > 240 ? 240 : (int)fps;
}

static int l_set_fps(lua_State* L)
{
    app.fps = clamp_fps(luaL_checkinteger(L, 1));
    if (!lua_isnoneornil(L, 2))
    {
        app.idle_fps = clamp_fps(luaL_checkinteger(L, 2));
    }

    return 0;
}

static int l_text_input(lua_State* L)
{
    if (lua_toboolean(L, 1))
    {
        SDL_StartTextInput(app.window);
    }
    else
    {
        SDL_StopTextInput(app.window);
    }

    return 0;
}

static int l_mouse(lua_State* L)
{
    float x;
    float y;
    SDL_MouseButtonFlags b = SDL_GetMouseState(&x, &y);
    lua_pushnumber(L, x);
    lua_pushnumber(L, y);
    lua_pushboolean(L, (b & SDL_BUTTON_LMASK) != 0);
    return 3;
}

static int l_clipboard(lua_State* L)
{
    if (lua_isstring(L, 1))
    {
        SDL_SetClipboardText(lua_tostring(L, 1));
        return 0;
    }

    char* t = SDL_GetClipboardText();
    lua_pushstring(L, t ? t : "");
    SDL_free(t);
    return 1;
}

static int l_cursor(lua_State* L)
{
    static const char* names[] = {
        "arrow",
        "hand",
        "text",
        "ew",
        "ns",
        "move",
        NULL,
    };

    static const SDL_SystemCursor ids[] = {
        SDL_SYSTEM_CURSOR_DEFAULT,
        SDL_SYSTEM_CURSOR_POINTER,
        SDL_SYSTEM_CURSOR_TEXT,
        SDL_SYSTEM_CURSOR_EW_RESIZE,
        SDL_SYSTEM_CURSOR_NS_RESIZE,
        SDL_SYSTEM_CURSOR_MOVE,
    };

    static SDL_Cursor* cache[6];
    int i = luaL_checkoption(L, 1, "arrow", names);
    if (!cache[i])
    {
        cache[i] = SDL_CreateSystemCursor(ids[i]);
    }

    if (cache[i])
    {
        SDL_SetCursor(cache[i]);
    }

    return 0;
}

static int l_screenshot(lua_State* L)
{
    SDL_Surface* s = SDL_RenderReadPixels(app.renderer, NULL);
    bool ok = s && SDL_SaveBMP(s, luaL_checkstring(L, 1));
    SDL_DestroySurface(s);
    lua_pushboolean(L, ok);
    return 1;
}

void api_window_open(lua_State* L)
{
    static const luaL_Reg fns[] = {
        { "size", l_size },
        { "set_size", l_set_size },
        { "set_min_size", l_set_min_size },
        { "set_title", l_set_title },
        { "fullscreen", l_fullscreen },
        { "set_fps", l_set_fps },
        { "text_input", l_text_input },
        { "mouse", l_mouse },
        { "clipboard", l_clipboard },
        { "cursor", l_cursor },
        { "screenshot", l_screenshot },
        { NULL, NULL },
    };

    luaL_newlib(L, fns);
    lua_setfield(L, -2, "window");
}
