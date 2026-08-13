#include "../headers/eq_state.h"

static void eq_state_reset_window(EqState* state)
{
	state->window_valid = 0;
	memset(state->window, 0, sizeof(state->window));
}

static bool eq_state_compute_bars(EqState* state)
{
	int output_size = (FFT_SIZE / 2) + 1;

	kiss_fftr(state->cfg, state->window, state->freqdata);

	for (int i = 0; i < output_size; i++)
	{
		float real = state->freqdata[i].r;
		float imag = state->freqdata[i].i;
        state->magnitude[i] = sqrtf((real * real) + (imag * imag));
	}

	for (int i = 0; i < BAR_COUNT; i++)
	{
		int start_bin = (int)roundf(state->boundaries[i]);
		int end_bin = (int)roundf(state->boundaries[i + 1]);

		if (start_bin > end_bin)
		{
			state->bars[i] = 0.0f;
			continue;
		}

		float max = 0.0f;
		for (int j = start_bin; j < end_bin && j < output_size; j++)
		{
			max = fmaxf(max, state->magnitude[j]);
		}

		state->bars[i] = max;
	}

	float max_val = 0.0f;
	for (int i = 0; i < BAR_COUNT; i++)
	{
		max_val = fmaxf(max_val, state->bars[i]);
	}

	if (max_val > 0.0f)
	{
		for (int i = 0; i < BAR_COUNT; i++)
		{
			state->bars[i] /= max_val;
		}
	}

	return true;
}

void eq_state_uninit(EqState* state)
{
    if (state->decoder_active)
    {
        ma_decoder_uninit(&state->decoder);
        state->decoder_active = false;
    }

    if (state->cfg)
    {
        kiss_fftr_free(state->cfg);
        state->cfg = NULL;
    }

    free(state->freqdata);
    free(state->magnitude);
    free(state->bars);
    free(state->boundaries);

    memset(state, 0, sizeof(EqState));
}

bool eq_state_init(EqState* state, const char* path)
{
	memset(state, 0, sizeof(EqState));

	ma_decoder_config config = ma_decoder_config_init(ma_format_f32, 1, 0);
    ma_result result = ma_decoder_init_file(path, &config, &state->decoder);

	if (result != MA_SUCCESS)
	{
		return false;
	}

    state->decoder_active = true;

    ma_uint32 sample_rate = 0;
    result = ma_decoder_get_data_format(&state->decoder, NULL, NULL, &sample_rate, NULL, 0);

    if (result != MA_SUCCESS || sample_rate == 0)
    {
        eq_state_uninit(state);
        return false;
    }

    state->step = sample_rate / 10;
    if (state->step == 0)
    {
        state->step = 1;
    }
    if (state->step > FFT_SIZE)
    {
        state->step = FFT_SIZE;
    }

    state->cfg = kiss_fftr_alloc(FFT_SIZE, 0, NULL, NULL);
    if (!state->cfg)
    {
        eq_state_uninit(state);
        return false;
    }

    int output_size = (FFT_SIZE / 2) + 1;

    state->freqdata = malloc(output_size * sizeof(kiss_fft_cpx));
    if (!state->freqdata)
    {
        eq_state_uninit(state);
        return false;
    }

    state->magnitude = malloc(output_size * sizeof(float));
    if (!state->magnitude)
    {
        eq_state_uninit(state);
        return false;
    }

    state->bars = malloc(BAR_COUNT * sizeof(float));
    if (!state->bars)
    {
        eq_state_uninit(state);
        return false;
    }

    state->boundaries = malloc((BAR_COUNT + 1) * sizeof(float));
    if (!state->boundaries)
    {
        eq_state_uninit(state);
        return false;
    }

    float low = 1.0f;
    float high = (float)(output_size - 1);
    for (int i = 0; i <= BAR_COUNT; i++)
    {
        float fraction = (float)i / BAR_COUNT;
        state->boundaries[i] = low * powf(high / low, fraction);
    }

    eq_state_reset_window(state);
    return true;
}

bool eq_state_init_win(EqState* state, const wchar_t* path)
{
	memset(state, 0, sizeof(EqState));

	ma_decoder_config config = ma_decoder_config_init(ma_format_f32, 1, 0);
    ma_result result = ma_decoder_init_file_w(path, &config, &state->decoder);

	if (result != MA_SUCCESS)
	{
		return false;
	}

    state->decoder_active = true;

    ma_uint32 sample_rate = 0;
    result = ma_decoder_get_data_format(&state->decoder, NULL, NULL, &sample_rate, NULL, 0);

    if (result != MA_SUCCESS || sample_rate == 0)
    {
        eq_state_uninit(state);
        return false;
    }

    state->step = sample_rate / 10;
    if (state->step == 0)
    {
        state->step = 1;
    }
    if (state->step > FFT_SIZE)
    {
        state->step = FFT_SIZE;
    }

    state->cfg = kiss_fftr_alloc(FFT_SIZE, 0, NULL, NULL);
    if (!state->cfg)
    {
        eq_state_uninit(state);
        return false;
    }

    int output_size = (FFT_SIZE / 2) + 1;

    state->freqdata = malloc(output_size * sizeof(kiss_fft_cpx));
    if (!state->freqdata)
    {
        eq_state_uninit(state);
        return false;
    }

    state->magnitude = malloc(output_size * sizeof(float));
    if (!state->magnitude)
    {
        eq_state_uninit(state);
        return false;
    }

    state->bars = malloc(BAR_COUNT * sizeof(float));
    if (!state->bars)
    {
        eq_state_uninit(state);
        return false;
    }

    state->boundaries = malloc((BAR_COUNT + 1) * sizeof(float));
    if (!state->boundaries)
    {
        eq_state_uninit(state);
        return false;
    }

    float low = 1.0f;
    float high = (float)(output_size - 1);
    for (int i = 0; i <= BAR_COUNT; i++)
    {
        float fraction = (float)i / BAR_COUNT;
        state->boundaries[i] = low * powf(high / low, fraction);
    }

    eq_state_reset_window(state);
    return true;
}

// Reads the next chunk of frames sequentially, advances the sliding window,
// and runs the FFT. Returns false if the decoder isn't active, the window
// isn't full yet, or we've hit the end of the file with no data.
//
// On success, *out_bars points to state->bars (BAR_COUNT floats). The caller
// must NOT free this pointer, it is owned by EqState.
bool eq_state_tick(EqState* state, float** out_bars, int* out_bar_count)
{
    *out_bars = NULL;
    *out_bar_count = 0;

    if (!state->decoder_active)
    {
        return false;
    }

    ma_uint64 to_read = state->step;
    float* read_dest = NULL;

    if (state->window_valid < FFT_SIZE)
    {
        // First few ticks: fill the window from the left
        ma_uint64 space = FFT_SIZE - state->window_valid;
        if (to_read > space)
        {
            to_read = space;
        }
        read_dest = state->window + state->window_valid;
    }
    else
    {
        // Window is full: shift left by step, append new frames at the end
        memmove(state->window, state->window + state->step, (FFT_SIZE - state->step) * sizeof(float));
        read_dest = state->window + (FFT_SIZE - state->step);
    }

    ma_uint64 frames_read = 0;
    ma_result result = ma_decoder_read_pcm_frames(&state->decoder, read_dest, to_read, &frames_read);

    if (frames_read == 0)
    {
        if (result != MA_SUCCESS && result != MA_AT_END)
        {
            return false;
        }

        if (state->window_valid == 0)
        {
            return false;
        }
    }

    if (state->window_valid < FFT_SIZE)
    {
        state->window_valid += frames_read;
        if (state->window_valid > FFT_SIZE)
        {
            state->window_valid = FFT_SIZE;
        }
    }

    if (state->window_valid < FFT_SIZE)
    {
        return false;
    }

    eq_state_compute_bars(state);

    *out_bars = state->bars;
    *out_bar_count = BAR_COUNT;
    return true;
}

// Seeks the decoder to an absolute position in seconds and clears the window
// so the next tick refills it. This is the ONLY place we ever seek, and it
// should only be called on user scrub or resume from pause.
bool eq_state_seek(EqState* state, double seconds)
{
    if (!state->decoder_active)
    {
        return false;
    }

    ma_uint32 sample_rate = 0;
    ma_result result = ma_decoder_get_data_format(&state->decoder, NULL, NULL, &sample_rate, NULL, 0);

    if (result != MA_SUCCESS || sample_rate == 0)
    {
        return false;
    }

    ma_uint64 target_frame = (ma_uint64)(seconds * (double)sample_rate);
    result = ma_decoder_seek_to_pcm_frame(&state->decoder, target_frame);

    if (result != MA_SUCCESS)
    {
        return false;
    }

    eq_state_reset_window(state);
    return true;
}