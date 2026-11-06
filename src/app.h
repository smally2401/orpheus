#ifndef ORPHEUS_APP_H
#define ORPHEUS_APP_H

#include <SDL3/SDL.h>
#include <lua5.4/lua.h>
#include <stdbool.h>

#define ORPHEUS_VERSION "0.1.0"

typedef struct
{
    SDL_Window* window;
    SDL_Renderer* renderer;
    lua_State* L;
    bool running;
    int exit_code;
    int fps;
    int idle_fps;
    const char* config_file;
    const char* runtime_dir;
} App;

extern App app;

#endif
