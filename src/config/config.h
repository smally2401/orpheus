#ifndef CONFIG_H
#define CONFIG_H

#include <glib.h>

#include <lua.h>
#include <lauxlib.h>
#include <lualib.h>

#include "../backend.h"

typedef enum
{
	CMD_PUSH_PLAYLIST,
} CommandType;

typedef struct
{
	CommandType type;
	union
	{
		Playlist* playlist;
	} data;
} Command;

void load_lua(OrpheusBackend* backend);

#endif
