#include "api.h"

#include "app.h"
#include "glib.h"

#include <gio/gio.h>
#include <glib/gstdio.h>

#include <string.h>

static int l_checksum(lua_State* L)
{
    static const char* names[] = {
        "md5",
        "sha1",
        "sha256",
        "sha512",
        NULL,
    };

    static const GChecksumType types[] = {
        G_CHECKSUM_MD5,
        G_CHECKSUM_SHA1,
        G_CHECKSUM_SHA256,
        G_CHECKSUM_SHA512,
    };

    size_t n;
    const char* s = luaL_checklstring(L, 1, &n);
    int t = luaL_checkoption(L, 2, "md5", names);
    gchar* h = g_compute_checksum_for_data(types[t], (const guchar*)s, n);
    lua_pushstring(L, h);
    g_free(h);
    return 1;
}

static int l_time(lua_State* L)
{
    lua_pushnumber(L, (double)SDL_GetTicksNS() / 1e9);
    return 1;
}

static int l_unix_time(lua_State* L)
{
    lua_pushnumber(L, (double)g_get_real_time() / 1e6);
    return 1;
}

static int l_quit(lua_State* L)
{
    app.exit_code = (int)luaL_optinteger(L, 1, 0);
    app.running = false;
    return 0;
}

void api_sys_open(lua_State* L)
{
    static const luaL_Reg fns[] = {
        { "checksum", l_checksum },
        { "time", l_time },
        { "unix_time", l_unix_time },
        { "quit", l_quit },
        { NULL, NULL },
    };

    luaL_setfuncs(L, fns, 0);
}
