#include "script.h"

#include "app.h"
#include "api.h"
#include "util.h"

#include <glib.h>
#include <lauxlib.h>
#include <lua.h>
#include <lualib.h>

#include <stdarg.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

static void* l_alloc(void* ud, void* ptr, size_t osize, size_t nsize)
{
    (void)ud;
    (void)osize;
    if (nsize == 0)
    {
        free(ptr);
        return NULL;
    }

    void* p = realloc(ptr, nsize);
    if (!p)
    {
        die("out of memory (lua, %zu bytes)", nsize);
    }

    return p;
}

static int traceback(lua_State* L)
{
    const char* msg = lua_tostring(L, 1);
    if (!msg)
    {
        msg = lua_pushfstring(L, "(error object is a %s value)", luaL_typename(L, 1));
    }

    luaL_traceback(L, L, msg, 1);
    return 1;
}

bool script_pcall(lua_State* L, int nargs, int nres)
{
    int base = lua_gettop(L) - nargs;
    lua_pushcfunction(L, traceback);
    lua_insert(L, base);
    int r = lua_pcall(L, nargs, nres, base);
    if (r != LUA_OK)
    {
        log_msg("lua: %s", lua_tostring(L, -1));
        lua_pop(L, 1);
        return false;
    }

    return true;
}

static int embedded_searcher(lua_State* L)
{
    const char* name = luaL_checkstring(L, 1);
    if (strncmp(name, "orpheus.", 8) != 0)
    {
        lua_pushfstring(L, "no embedded module '%s'", name);
        return 1;
    }

    const char* sub = name + 8;
    if (app.runtime_dir)
    {
        char* file = g_strdup_printf("%s/%s.lua", app.runtime_dir, sub);
        bool exists = g_file_test(file, G_FILE_TEST_IS_REGULAR);
        if (exists)
        {
            if (luaL_loadfile(L, file) != LUA_OK)
            {
                g_free(file);
                return lua_error(L);
            }

            lua_pushstring(L, file);
            g_free(file);
            return 2;
        }

        g_free(file);
    }

    lua_pushfstring(L, "no embedded module '%s'", name);
    return 1;
}

static void set_path_field(lua_State* L, const char* key, const char* a, const char* b)
{
    char* p = g_build_filename(a, b, NULL);
    lua_pushstring(L, p);
    lua_setfield(L, -2, key);
    g_free(p);
}

static void push_paths(lua_State* L)
{
    lua_newtable(L);
    set_path_field(L, "config", g_get_user_config_dir(), "orpheus");
    set_path_field(L, "data", g_get_user_data_dir(), "orpheus");
    set_path_field(L, "cache", g_get_user_cache_dir(), "orpheus");
    const char* music = g_get_user_special_dir(G_USER_DIRECTORY_MUSIC);
    if (music)
    {
        lua_pushstring(L, music);
        lua_setfield(L, -2, "music");
    }
    else
    {
        set_path_field(L, "music", g_get_home_dir(), "Music");
    }

    lua_pushstring(L, g_get_home_dir());
    lua_setfield(L, -2, "home");
}

static void setup_package_path(lua_State* L)
{
    lua_getglobal(L, "package");
    char* cfg = g_build_filename(g_get_user_config_dir(), "orpheus", NULL);
    lua_getfield(L, -1, "path");
    lua_pushfstring(L, "%s/?.lua;%s/?/init.lua;%s", cfg, cfg, lua_tostring(L, -1));
    lua_setfield(L, -3, "path");
    lua_pop(L, 1);
    g_free(cfg);

    lua_getfield(L, -1, "searchers");
    lua_pushcfunction(L, embedded_searcher);
    lua_rawseti(L, -2, (lua_Integer)lua_rawlen(L, -2) + 1);
    lua_pop(L, 2);
}

void script_init(int nfiles, char** files)
{
    lua_State* L = lua_newstate(l_alloc, NULL);
    if (!L)
    {
        die("could not create lua state");
    }

    app.L = L;
    luaL_openlibs(L);
    setup_package_path(L);

    lua_newtable(L);
    api_gfx_open(L);
    api_window_open(L);
    api_audio_open(L);
    api_sys_open(L);
    push_paths(L);
    lua_setfield(L, -2, "paths");
    lua_pushstring(L, ORPHEUS_VERSION);
    lua_setfield(L, -2, "version");
    if (app.config_file)
    {
        lua_pushstring(L, app.config_file);
        lua_setfield(L, -2, "config_file");
    }

    lua_createtable(L, nfiles, 0);
    for (int i = 0; i < nfiles; i++)
    {
        char* abs = g_canonicalize_filename(files[i], NULL);
        lua_pushstring(L, abs);
        g_free(abs);
        lua_rawseti(L, -2, i + 1);
    }

    lua_setfield(L, -2, "args");
    lua_setglobal(L, "orpheus");
    lua_getglobal(L, "require");
    lua_pushstring(L, "orpheus.boot");
    if (!script_pcall(L, 1, 0))
    {
        die("the lua runtime failed to start");
    }
}

void script_event(const char* name, const char* fmt, ...)
{
    lua_State* L = app.L;
    if (!L)
    {
        return;
    }

    lua_getglobal(L, "orpheus");
    lua_getfield(L, -1, "_event");
    lua_remove(L, -2);
    if (!lua_isfunction(L, -1))
    {
        lua_pop(L, 1);
        return;
    }

    lua_pushstring(L, name);
    int n = 1;
    va_list ap;
    va_start(ap, fmt);
    for (const char* f = fmt; f && *f; f++, n++)
    {
        switch (*f)
        {
        case 'i': 
            lua_pushinteger(L, va_arg(ap, int));
            break;
        case 'n':
            lua_pushnumber(L, va_arg(ap, double));
            break;
        case 's':
            lua_pushstring(L, va_arg(ap, const char*));
            break;
        case 'b':
            lua_pushboolean(L, va_arg(ap, int));
            break;
        default:
            lua_pushnil(L);
            break;
        }
    }

    va_end(ap);
    script_pcall(L, n, 0);
}

void script_shutdown(void)
{
    if (!app.L)
    {
        return;
    }

    script_event("quit", "");
    api_gfx_shutdown(app.L);
    lua_close(app.L);
    app.L = NULL;
}
