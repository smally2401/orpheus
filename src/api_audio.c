#include "api.h"

#include "audio.h"
#include "lauxlib.h"
#include "lua.h"

static int l_load(lua_State* L)
{
    const char* err = NULL;
    if (!audio_load(luaL_checkstring(L, 1), &err))
    {
        lua_pushnil(L);
        lua_pushstring(L, err ? err : "unknown error");
        return 2;
    }

    lua_pushboolean(L, 1);
    return 1;
}

static int l_unload(lua_State* L)
{
    (void)L;
    audio_unload();
    return 0;
}

static int l_play(lua_State* L)
{
    (void)L;
    audio_play();
    return 0;
}

static int l_pause(lua_State* L)
{
    (void)L;
    audio_pause();
    return 0;
}

static int l_stop(lua_State* L)
{
    (void)L;
    audio_stop();
    return 0;
}

static int l_seek(lua_State* L)
{
    lua_pushboolean(L, audio_seek(luaL_checknumber(L, 1)));
    return 1;
}

static int l_position(lua_State* L)
{
    lua_pushnumber(L, audio_position());
    return 1;
}

static int l_duration(lua_State* L)
{
    lua_pushnumber(L, audio_duration());
    return 1;
}

static int l_volume(lua_State* L)
{
    if (!lua_isnoneornil(L, 1))
    {
        audio_set_volume((float)luaL_checknumber(L, 1));
    }

    lua_pushnumber(L, audio_volume());
    return 1;
}

static int l_state(lua_State* L)
{
    static const char* names[] = { "stopped", "playing", "paused" };
    lua_pushstring(L, names[audio_state()]);
    return 1;
}

static int l_path(lua_State* L)
{
    const char* p = audio_path();
    if (p)
    {
        lua_pushstring(L, p);
    }
    else
    {
        lua_pushnil(L);
    }

    return 1;
}

void api_audio_open(lua_State* L)
{
    static const luaL_Reg fns[] = {
        { "load", l_load },
        { "unload", l_unload },
        { "play", l_play },
        { "pause", l_pause },
        { "stop", l_stop },
        { "seek", l_seek },
        { "position", l_position },
        { "duration", l_duration },
        { "volume", l_volume },
        { "state", l_state },
        { "path", l_path },
        { NULL, NULL },
    };

    luaL_newlib(L, fns);
    lua_setfield(L, -2, "audio");
}
