#ifndef ORPHEUS_API_H
#define ORPHEUS_API_H

#include <SDL3/SDL.h>
#include <lua.h>
#include <lauxlib.h>

void api_gfx_open(lua_State* L);
void api_window_open(lua_State* L);
void api_audio_open(lua_State* L);
void api_sys_open(lua_State* L);

void api_gfx_shutdown(lua_State* L);
void api_gfx_begin_frame(void);
SDL_FColor api_check_color(lua_State* L, int idx);

#endif
