#ifndef UTILS_H
#define UTILS_H

#define LOG_ERR(fmt, ...)                                                      \
	fprintf(stderr, "\033[31m[ERROR]\033[0m " fmt "\n", ##__VA_ARGS__)

char* orph_strdup(const char* str);
const char* get_extension(const char* path);
char* expand_tilde(const char* path);
char* get_default_music_dir(void);

#endif
