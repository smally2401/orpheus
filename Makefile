CC = gcc
CFLAGS = -Wall -Wextra -MMD -MP

TARGET = target/chronos
OBJDIR = target/object

SRCS = $(shell find src -name '*.c')
OBJS = $(SRCS:src/%.c=$(OBJDIR)/%.o)
DEPS = $(OBJS:.o=.d)

all: $(TARGET)

$(TARGET): $(OBJS)
	@mkdir -p target
	$(CC) $^ -o $@

$(OBJDIR)/%.o: src/%.c
	@mkdir -p $(dir $@)
	$(CC) $(CFLAGS) -c $< -o $@

-include $(DEPS)

run: all
	./$(TARGET)

clean:
	rm -rf target

.PHONY: all run clean
