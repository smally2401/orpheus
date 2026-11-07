#ifndef ORPHEUS_SCRIPT_H
#define ORPHEUS_SCRIPT_H

#include <lua.h>

#include <stdbool.h>

void script_init(int nfiles, char** files);
void script_shutdown(void);
void script_event(const char* name, const char* fmt, ...);
bool script_pcall(lua_State* L, int nargs, int nres);

#endif
