CC = gcc
BASE_CFLAGS = -Wall -Wextra -MMD -MP
SDL_FLAGS = $(shell pkg-config --cflags --libs sdl3)

TARGET_NAME = orpheus
BUILD_DIR = target

MODE ?= release

ifeq ($(MODE),debug)
	CFLAGS = $(BASE_CFLAGS) -g -O0 -DDEBUG
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
	@mkdir -p target
	$(CC) $^ -o $@ $(SDL_FLAGS) -lm $(LDFLAGS)

$(OBJDIR)/%.o: src/%.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c $< -o $@

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
