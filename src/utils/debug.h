#ifndef DEBUG_H
#define DEBUG_H
#ifdef DEBUG

#define DEBUG_PRINT(...) printf(__VA_ARGS__)

#define DEBUG_SONG_CREATE(var, path)                                                               \
	do                                                                                             \
	{                                                                                              \
		Song* var = song_create(path);                                                             \
		song_free(var);                                                                            \
	} while (0)

#else

#define DEBUG_PRINT(...) ((void)0)
#define DEBUG_SONG_CREATE(var, path) ((void)0)

#endif
#endif
