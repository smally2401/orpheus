CC = gcc
BASE_CFLAGS = -MMD -MP
MODE ?= release

SDL_CFLAGS = $(shell pkg-config --cflags sdl3 glib-2.0)
SDL_LIBS   = $(shell pkg-config --libs sdl3 glib-2.0)

TARGET_NAME = orpheus
BUILD_DIR = target

ifeq ($(MODE),debug)
	CFLAGS = -Wall -Wextra -Wpedantic $(BASE_CFLAGS) -g -O0 -DDEBUG \
			 -fsanitize=address,undefined -fno-omit-frame-pointer
	LDFLAGS = -fsanitize=address,undefined
else
	CFLAGS = $(BASE_CFLAGS) -O2 -DNDEBUG
	LDFLAGS = -flto
endif

TARGET = $(BUILD_DIR)/$(MODE)/$(TARGET_NAME)
OBJDIR = $(BUILD_DIR)/$(MODE)/object

SRCS = $(shell find src -name '*.c')
OBJS = $(SRCS:src/%.c=$(OBJDIR)/%.o)
DEPS = $(OBJS:.o=.d)

all: $(TARGET)

$(TARGET): $(OBJS)
	@mkdir -p $(dir $@)
	$(CC) $^ -o $@ $(SDL_LIBS) $(LDFLAGS) -lm

$(OBJDIR)/%.o: src/%.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) $(SDL_CFLAGS) -c $< -o $@

-include $(DEPS)

debug:
	$(MAKE) MODE=debug

release:
	$(MAKE) MODE=release

run:
	$(MAKE) MODE=$(MODE)
	./$(TARGET)

clean:
	rm -rf $(BUILD_DIR)

.PHONY: all run clean
