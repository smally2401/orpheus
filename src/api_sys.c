#include "api.h"

#include "app.h"
#include "glib.h"
#include "lauxlib.h"
#include "lua.h"

#include <gio/gio.h>
#include <glib/gstdio.h>

#include <string.h>

static gint cmp_str(gconstpointer a, gconstpointer b)
{
    return strcmp(*(char* const *)a, *(char* const *)b);
}

static GPtrArray* sorted_entries(const char* dir)
{
    GDir* d = g_dir_open(dir, 0, NULL);
    if (!d)
    {
        return NULL;
    }

    GPtrArray* names = g_ptr_array_new_with_free_func(g_free);
    const char* n;
    while ((n = g_dir_read_name(d)))
    {
        g_ptr_array_add(names, g_strdup(n));
    }

    g_dir_close(d);
    g_ptr_array_sort(names, cmp_str);
    return names;
}

static bool ext_ok(lua_State* L, int exts, const char* name)
{
    if (!exts)
    {
        return true;
    }

    const char* dot = strrchr(name, '.');
    if (!dot)
    {
        return false;
    }

    char* e = g_ascii_strdown(dot + 1, -1);
    lua_getfield(L, exts, e);
    bool ok = lua_toboolean(L, -1);
    lua_pop(L, 1);
    g_free(e);
    return ok;
}

static void scan(lua_State* L, const char* dir, int exts, int out, int depth)
{
    if (depth > 32)
    {
        return;
    }

    GPtrArray* names = sorted_entries(dir);
    if (!names)
    {
        return;
    }

    for (guint i = 0; i < names->len; i++)
    {
        const char* name = names->pdata[i];
        if (name[0] == '.')
        {
            continue;
        }

        char* p = g_build_filename(dir, name, NULL);
        if (g_file_test(p, G_FILE_TEST_IS_DIR))
        {
            scan(L, p, exts, out, depth + 1);
        }
        else if (ext_ok(L, exts, name))
        {
            lua_pushstring(L, p);
            lua_rawseti(L, out, (lua_Integer)lua_rawlen(L, out) + 1);
        }

        g_free(p);
    }

    g_ptr_array_free(names, TRUE);
}

static int l_scan(lua_State* L)
{
    const char* dir = luaL_checkstring(L, 1);
    int exts = 0;
    if (lua_istable(L, 2))
    {
        lua_newtable(L);
        exts = lua_gettop(L);
        lua_Integer n = (lua_Integer)lua_rawlen(L, 2);
        for (lua_Integer i = 1; i <= n; i++)
        {
            lua_rawgeti(L, 2, i);
            char* e = g_ascii_strdown(luaL_checkstring(L, -1), -1);
            lua_pushboolean(L, 1);
            lua_setfield(L, exts, e[0] == '.' ? e + 1 : e);
            g_free(e);
            lua_pop(L, 1);
        }
    }

    lua_newtable(L);
    scan(L, dir, exts, lua_gettop(L), 0);
    return 1;
}

static int l_list(lua_State* L)
{
    const char* dir = luaL_checkstring(L, 1);
    GPtrArray* names = sorted_entries(dir);
    if (!names)
    {
        lua_pushnil(L);
        return 1;
    }

    lua_createtable(L, (int)names->len, 0);
    for (guint i = 0; i < names->len; i++)
    {
        char* p = g_build_filename(dir, names->pdata[i], NULL);
        lua_createtable(L, 0, 2);
        lua_pushstring(L, names->pdata[i]);
        lua_setfield(L, -2, "name");
        lua_pushboolean(L, g_file_test(p, G_FILE_TEST_IS_DIR));
        lua_setfield(L, -2, "dir");
        lua_rawseti(L, -2, (lua_Integer)i + 1);
        g_free(p);
    }

    g_ptr_array_free(names, TRUE);
    return 1;
}

static int l_stat(lua_State* L)
{
    GStatBuf st;
    if (g_stat(luaL_checkstring(L, 1), &st) != 0)
    {
        lua_pushnil(L);
        return 1;
    }

    lua_createtable(L, 0, 3);
    lua_pushinteger(L, (lua_Integer)st.st_size);
    lua_setfield(L, -2, "size");
    lua_pushinteger(L, (lua_Integer)st.st_mtime);
    lua_setfield(L, -2, "mtime");
    lua_pushboolean(L, S_ISDIR(st.st_mode));
    lua_setfield(L, -2, "dir");
    return 1;
}

static int l_mkdir(lua_State* L)
{
    lua_pushboolean(L, g_mkdir_with_parents(luaL_checkstring(L, 1), 0755) == 0);
    return 1;
}

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
    static const luaL_Reg fs[] = {
        { "scan", l_scan },
        { "list", l_list },
        { "stat", l_stat },
        { "mkdir", l_mkdir },
        { NULL, NULL },
    };

    static const luaL_Reg fns[] = {
        { "checksum", l_checksum },
        { "time", l_time },
        { "unix_time", l_unix_time },
        { "quit", l_quit },
        { NULL, NULL },
    };

    luaL_newlib(L, fs);
    lua_setfield(L, -2, "fs");
    luaL_setfuncs(L, fns, 0);
}
