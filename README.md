# Orpheus

A local-first music player written in C99 with a UI powered by
[Nuklear](https://github.com/Immediate-Mode-UI/Nuklear) and
[SDL3](https://github.com/libsdl-org/SDL), and Lua scripting for
configuration and playback customization.

> **Status:** very early, actively developed. Expect rough edges.

<!-- todo: screenshots -->

## Features

- Local library scanning with tag-based metadata and cover art
- Automatic album grouping, sorted by title and track number
- Native audio playback via 
  [miniaudio](https://github.com/mackron/miniaudio)

## Building and running

Requires `gcc`, `make` and `pkg-config`.

Dependencies: SDL3, SDL3_image, glib-2.0.

```sh
make
./target/release/orpheus
```

## Platform support

| Platform | Support |
| --- | --- |
| Linux | Working: primary development platform |
| Windows | Testing planned before `v0.1` |
