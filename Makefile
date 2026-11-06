CC = gcc
PREFIX ?= /usr/local
PKGS = sdl3 sdl3-image glib-2.0 gio-2.0 lua5.4 fontconfig

CFLAGS ?= -O2 -g
CPPFLAGS_ALL = -std=c99 -D_DEFAULT_SOURCE -Ivendor $(shell pkg-config --cflags $(PKGS))
WARN = -Wall -Wextra -Wpedantic -Wshadow -Wno-overlength-strings
LDLIBS += $(shell pkg-config --libs $(PKGS)) -lm -lpthread -ldl

BUILD = build
SRC = $(wildcard src/*.c)
OBJ = $(patsubst src/%.c,$(BUILD)/%.o,$(SRC))

all: orpheus

orpheus: $(OBJ)
	$(CC) $(CFLAGS) $(LDFLAGS) -o $@ $^ $(LDLIBS)

$(BUILD):
	mkdir -p $@

$(BUILD)/vendor_impl.o: src/vendor_impl.c | $(BUILD)
	$(CC) $(CPPFLAGS_ALL) $(CFLAGS) -w -c -o $@ $<

$(BUILD)/%.o: src/%.c $(wildcard src/*.h) | $(BUILD)
	$(CC) $(CPPFLAGS_ALL) $(CFLAGS) $(WARN) -c -o $@ $<

DEVENV = 	XDG_CONFIG_HOME=$(CURDIR)/dev/config XDG_DATA_HOME=$(CURDIR)/dev/data \
			XDG_CACHE_HOME=$(CURDIR)/dev/cache

run: orpheus
	$(DEVENV) ./orpheus $(ARGS)

clean:
	rm -rf $(BUILD) orpheus

.PHONY: all run clean
