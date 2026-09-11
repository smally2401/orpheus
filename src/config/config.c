#include "config.h"
#include "glib.h"
#include "lauxlib.h"
#include "lstate.h"
#include "lua.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32

#include <direct.h>
#define orph_mkdir(path) _mkdir(path)

#else

#include <sys/stat.h>
#define orph_mkdir(path) mkdir(path, 0755)

#endif

static char* get_config_path(void)
{
#ifdef _WIN32
	const char* base = getenv("APPDATA");
	const char* suffix1 = "\\orpheus";
	const char* suffix2 = "\\config.lua";

	if (!base)
	{
		return NULL;
	}
#else
	const char* base = getenv("XDG_CONFIG_HOME");
	char fallback[512];

	if (!base)
	{
		const char* home = getenv("HOME");
		if (!home)
		{
			return NULL;
		}

		snprintf(fallback, sizeof(fallback), "%s/.config", home);
		base = fallback;
		orph_mkdir(base);
	}

	const char* suffix1 = "/orpheus";
	const char* suffix2 = "/config.lua";
#endif

	size_t dir_len = strlen(base) + strlen(suffix1) + 1;
	size_t path_len = dir_len + strlen(suffix2);

	char* config_dir = malloc(dir_len);
	if (!config_dir)
	{
		return NULL;
	}

	char* config_path = malloc(path_len);
	if (!config_path)
	{
		free(config_dir);
		return NULL;
	}

	snprintf(config_dir, dir_len, "%s%s", base, suffix1);
	orph_mkdir(config_dir);
	snprintf(config_path, path_len, "%s%s", config_dir, suffix2);
	free(config_dir);

	return config_path;
}

static int lua_push_playlist(lua_State* lstate)
{
	lua_getfield(lstate, LUA_REGISTRYINDEX, "orpheus_backend");
	OrpheusBackend* backend = (OrpheusBackend*)lua_touserdata(lstate, -1);
	lua_pop(lstate, 1);

	const char* name = lua_tostring(lstate, 1);
	if (!name)
	{
		return luaL_error(lstate, "Missing 'name' argument in 'push_playlist'");
	}

	if (!lua_istable(lstate, 2))
	{
		return luaL_error(
		    lstate,
		    "Expected string table 'paths' argument in 'push_playlist'");
	}

	Playlist* playlist = g_malloc(sizeof(Playlist));
	playlist->name = g_strdup(name);
	playlist->songs = g_ptr_array_new();

	lua_Unsigned len = lua_rawlen(lstate, 2);
	for (lua_Unsigned i = 1; i <= len; i++)
	{
		lua_rawgeti(lstate, 2, (int)i);
		if (!lua_isstring(lstate, -1))
		{
			lua_pop(lstate, 1);

			g_free(playlist->name);
			g_ptr_array_free(playlist->songs, FALSE);
			g_free(playlist);

			return luaL_error(
			    lstate,
			    "Expected string in table 'paths' argument in 'push_playlist'");
		}

		const char* path = lua_tostring(lstate, -1);
		Song* song = song_from_path(backend, path);
		if (song)
		{
			g_ptr_array_add(playlist->songs, song);
		}

		lua_pop(lstate, 1);
	}

	g_ptr_array_add(backend->playlists, playlist);
	return 0;
}

void load_lua(OrpheusBackend* backend)
{
	lua_State* lstate = luaL_newstate();
	luaL_openlibs(lstate);

	lua_pushlightuserdata(lstate, backend);
	lua_setfield(lstate, LUA_REGISTRYINDEX, "orpheus_backend");

	lua_newtable(lstate);
	lua_pushcfunction(lstate, lua_push_playlist);
	lua_setfield(lstate, -2, "push_playlist");
	lua_setglobal(lstate, "orpheus");

	char* config_path = get_config_path();
	if (luaL_dofile(lstate, config_path) != LUA_OK)
	{
		const char* err = lua_tostring(lstate, -1);
		fprintf(stderr, "Lua error: %s\n", err);
		lua_pop(lstate, 1);
	}

	free(config_path);
	lua_close(lstate);
}
