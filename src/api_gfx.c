#include "api.h"

#include "app.h"
#include "lauxlib.h"
#include "lua.h"

#include <SDL3/SDL_pixels.h>
#include <SDL3/SDL_rect.h>
#include <SDL3/SDL_render.h>
#include <glib.h>

#include <string.h>

#define CLIP_DEPTH 64

static int default_font_ref = LUA_NOREF;
static SDL_Rect clips[CLIP_DEPTH];
static int nclips;

static int hexval(char c)
{
    if (c >= '0' && c <= '9')
    {
        return c - '0';
    }

    if (c >= 'a' && c <= 'f')
    {
        return c - 'a' + 10;
    }

    if (c >= 'A' && c <= 'F')
    {
        return c - 'A' + 10;
    }

    return -1;
}

SDL_FColor api_check_color(lua_State* L, int idx)
{
    SDL_FColor c = { 1, 1, 1, 1 };
    if (lua_type(L, idx) == LUA_TSTRING)
    {
        size_t n;
        const char* s = lua_tolstring(L, idx, &n);
        if (n && *s == '#')
        {
            s++;
            n--;
        }

        int v[8];
        for (size_t i = 0; i < n && i < 8; i++)
        {
            if ((v[i] = hexval(s[i])) < 0)
            {
                luaL_error(L, "invalid color '%s'", lua_tostring(L, idx));
            }
        }

        if (n == 3 || n == 4)
        {
            c.r = v[0] / 15.0f;
            c.g = v[1] / 15.0f;
            c.b = v[2] / 15.0f;
            c.a = n == 4 ? v[3] : 1;
        }
        else if (n == 6 || n == 8)
        {
            c.r = (v[0] * 16 + v[1]) / 255.0f;
            c.r = (v[2] * 16 + v[3]) / 255.0f;
            c.r = (v[4] * 16 + v[5]) / 255.0f;
            c.a = n == 8 ? (v[6] * 16 + v[7]) / 255.0f : 1;
        }
        else
        {
            luaL_error(L, "invalid color '%s'", lua_tostring(L, idx));
        }
    }
    else if (lua_istable(L, idx))
    {
        float* ch[4] = { &c.r, &c.g, &c.b, &c.a };
        for (int i = 0; i < 4; i++)
        {
            lua_rawgeti(L, idx, i + 1);
            if (lua_isnumber(L, -1))
            {
                *ch[i] = (float)lua_tonumber(L, -1) / 255.0f;
            }

            lua_pop(L, 1);
        }
    }
    else if (!lua_isnoneornil(L, idx))
    {
        luaL_typeerror(L, idx, "color");
    }

    return c;
}

static void set_color(SDL_FColor c)
{
    SDL_SetRenderDrawColorFloat(app.renderer, c.r, c.g, c.b, c.a);
}

static int l_line_height(lua_State* L)
{
    lua_pushnumber(L, SDL_DEBUG_TEXT_FONT_CHARACTER_SIZE);
    return 1;
}

static int l_measure(lua_State* L)
{
    size_t n;
    const char* s = luaL_checklstring(L, 1, &n);
    lua_pushnumber(L, (float)(g_utf8_strlen(s, (gssize)n) * SDL_DEBUG_TEXT_FONT_CHARACTER_SIZE));
    lua_pushnumber(L, SDL_DEBUG_TEXT_FONT_CHARACTER_SIZE);
    return 2;
}

static int l_fit(lua_State* L)
{
    luaL_checkstring(L, 1);
    lua_settop(L, 1);
    return 1;
}

static int l_text(lua_State* L)
{
    const char* s = luaL_checkstring(L, 1);
    float x = (float)luaL_checknumber(L, 2);
    float y = (float)luaL_checknumber(L, 3);
    set_color(api_check_color(L, 4));
    SDL_RenderDebugText(app.renderer, x, y, s);
    lua_pushnumber(L, (float)(strlen(s) * SDL_DEBUG_TEXT_FONT_CHARACTER_SIZE));
    return 1;
}

static int l_rect(lua_State* L)
{
    SDL_FRect r = {
        (float)luaL_checknumber(L, 1),
        (float)luaL_checknumber(L, 2),
        (float)luaL_checknumber(L, 3),
        (float)luaL_checknumber(L, 4),
    };

    set_color(api_check_color(L, 5));
    SDL_RenderFillRect(app.renderer, &r);
    return 0;
}

static int l_outline(lua_State* L)
{
    float x = (float)luaL_checknumber(L, 1);
    float y = (float)luaL_checknumber(L, 2);
    float w = (float)luaL_checknumber(L, 3);
    float h = (float)luaL_checknumber(L, 4);
    set_color(api_check_color(L, 5));
    float t = (float)luaL_optnumber(L, 6, 1);
    if (t <= 0)
    {
        return 0;
    }

    if (t * 2 >= w || t * 2 >= h)
    {
        SDL_FRect r = { x, y, w, h };
        SDL_RenderFillRect(app.renderer, &r);
        return 0;
    }

    SDL_FRect r[4] = {
        { x, y, w, t },
        { x, y + h - t, w, t },
        { x, y + t, t, h - 2 * t },
        { x + w - t, y + t, t, h - 2 * t },
    };

    SDL_RenderFillRects(app.renderer, r, 4);
    return 0;
}

static int l_line(lua_State* L)
{
    set_color(api_check_color(L, 5));
    SDL_RenderLine(app.renderer,
        (float)luaL_checknumber(L, 1),
        (float)luaL_checknumber(L, 2),
        (float)luaL_checknumber(L, 3),
        (float)luaL_checknumber(L, 4)
    );

    return 0;
}

static int l_poly(lua_State* L)
{
    luaL_checktype(L, 1, LUA_TTABLE);
    SDL_FColor c = api_check_color(L, 2);
    int n = (int)lua_rawlen(L, 1) / 2;
    if (n < 3)
    {
        return 0;
    }

    SDL_Vertex* v = lua_newuserdatauv(L, sizeof *v * (size_t)n, 0);
    for (int i = 0; i < n; i++)
    {
        lua_rawgeti(L, 1, i * 2 + 1);
        lua_rawgeti(L, 1, i * 2 + 2);
        v[i].position.x = (float)lua_tonumber(L, -2);
        v[i].position.y = (float)lua_tonumber(L, -1);
        v[i].color = c;
        v[i].tex_coord.x = 0;
        v[i].tex_coord.y = 0;
        lua_pop(L, 2);
    }

    int ni = (n - 2) * 3;
    int* idx = lua_newuserdatauv(L, sizeof *idx * (size_t)ni, 0);
    for (int i = 0; i < n - 2; i++)
    {
        idx[i * 3] = 0;
        idx[i * 3 + 1] = i + 1;
        idx[i * 3 + 2] = i + 2;
    }

    SDL_RenderGeometry(app.renderer, NULL, v, n, idx, ni);
    return 0;
}

static int l_clip(lua_State* L)
{
    SDL_Rect r = {
        (int)luaL_checknumber(L, 1),
        (int)luaL_checknumber(L, 2),
        (int)luaL_checknumber(L, 3),
        (int)luaL_checknumber(L, 4),
    };

    if (r.w < 0)
    {
        r.w = 0;
    }

    if (r.h < 0)
    {
        r.h = 0;
    }

    if (nclips == CLIP_DEPTH)
    {
        return luaL_error(L, "clip stack overflow");
    }

    if (nclips > 0)
    {
        SDL_Rect out;
        if (!SDL_GetRectIntersection(&clips[nclips - 1], &r, &out))
        {
            out = (SDL_Rect){ r.x, r.y, 0, 0 };
        }

        r = out;
    }

    clips[nclips] = r;
    nclips++;
    SDL_SetRenderClipRect(app.renderer, &r);
    return 0;
}

static int l_unclip(lua_State* L)
{
    (void)L;
    if (nclips > 0)
    {
        nclips--;
    }

    SDL_SetRenderClipRect(app.renderer, nclips ? &clips[nclips - 1] : NULL);
    return 0;
}

void api_gfx_open(lua_State* L)
{
    static const luaL_Reg fns[] = {
        { "rect", l_rect },
        { "outline", l_outline },
        { "line", l_line },
        { "poly", l_poly },
        { "text", l_text },
        { "measure", l_measure },
        { "fit", l_fit },
        { "line_height", l_line_height },
        { "clip", l_clip },
        { "unclip", l_unclip },
        { NULL, NULL },
    };

    luaL_newlib(L, fns);
    lua_setfield(L, -2, "gfx");
}

void api_gfx_shutdown(lua_State* L)
{
    (void)L;
    (void)default_font_ref;
}

void api_gfx_begin_frame(void)
{
    nclips = 0;
    SDL_SetRenderClipRect(app.renderer, NULL);
}
