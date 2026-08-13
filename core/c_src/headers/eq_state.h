#ifndef EQ_STATE_H
#define EQ_STATE_H

#include <stdbool.h>
#include "../audio/miniaudio.h"
#include "../equalizer/kiss_fft.h"
#include "../equalizer/kiss_fftr.h"

#define FFT_SIZE 2048
#define BAR_COUNT 32

typedef struct
{
	ma_decoder decoder;
	bool decoder_active;

	// Sliding sample window for the FFT
	float window[FFT_SIZE];
	// 0..FFT_SIZE
	ma_uint64 window_valid;

	// How many frames to read per tick
	ma_uint64 step;

	// Persistent FFT allocations
	kiss_fftr_cfg cfg;
	kiss_fft_cpx* freqdata;
	float* magnitude;
	float* bars;
	float* boundaries;
} EqState;

bool eq_state_init(EqState* state, const char* path);
bool eq_state_init_win(EqState* state, const wchar_t* path);
bool eq_state_tick(EqState* state, float** out_bars, int* out_bar_count);
bool eq_state_seek(EqState* state, double seconds);
void eq_state_uninit(EqState* state);

#endif