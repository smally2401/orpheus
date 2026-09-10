#include "config.h"

#include <lua5.4/lauxlib.h>
#include <lua5.4/lua.h>
#include <lua5.4/lualib.h>
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

static int touch_file(const char* path)
{
	FILE* f = fopen(path, "a");
	if (!f)
	{
		return -1;
	}

	fclose(f);
	return 0;
}

void load_lua(void)
{
	lua_State* lstate = luaL_newstate();
	luaL_openlibs(lstate);
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
