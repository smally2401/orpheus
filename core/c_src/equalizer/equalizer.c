#include "kiss_fft.h"
#include "kiss_fftr.h"
#include "../audio/miniaudio.h"
#include <unistd.h>

#define FFT_SIZE 8192
#define BAR_COUNT 32

float* miniaudio_equalizer(const char* path, double position_seconds)
{
    ma_decoder decoder;
    ma_decoder_config config = ma_decoder_config_init(ma_format_f32, 1, 0);
    ma_result result = ma_decoder_init_file(path, &config, &decoder);

    if (result != MA_SUCCESS)
    {
        return NULL;
    }

    ma_uint32 sample_rate;
    result = ma_decoder_get_data_format(&decoder, NULL, NULL, &sample_rate, NULL, 0);

    if (result != MA_SUCCESS)
    {
        ma_decoder_uninit(&decoder);
        return NULL;
    }

    ma_uint64 target_frame = (ma_uint64)(position_seconds * sample_rate);
    result = ma_decoder_seek_to_pcm_frame(&decoder, target_frame);

    printf("%i\n", sample_rate);
    printf("%llu\n", target_frame);

    if (result != MA_SUCCESS)
    {
        ma_decoder_uninit(&decoder);
        return NULL;
    }

    float* buffer = malloc(FFT_SIZE * sizeof(float));

    if (!buffer)
    {
        ma_decoder_uninit(&decoder);
        return NULL;
    }

    ma_uint64 frames_read;
    result = ma_decoder_read_pcm_frames(&decoder, buffer, FFT_SIZE, &frames_read);

    if (frames_read != FFT_SIZE)
    {
        puts("seeked too close to the end");
        free(buffer);
        ma_decoder_uninit(&decoder);
        return NULL;
    }

    ma_decoder_uninit(&decoder);
    
    kiss_fftr_cfg cfg = kiss_fftr_alloc(FFT_SIZE, 0, NULL, NULL);

    if (!cfg)
    {
        free(buffer);
        return NULL;
    }

    int output_size = (FFT_SIZE / 2) + 1;
    kiss_fft_cpx* freqdata = malloc(output_size * sizeof(kiss_fft_cpx));

    if (!freqdata)
    {
        free(buffer);
        kiss_fftr_free(cfg);
        return NULL;
    }

    kiss_fftr(cfg, buffer, freqdata);
    float* magnitude = malloc(output_size * sizeof(float));

    if (!magnitude)
    {
        free(buffer);
        kiss_fftr_free(cfg);
        free(freqdata);
        return NULL;
    }

    for (int i = 0; i < output_size; i++)
    {
        float real = freqdata[i].r;
        float imag = freqdata[i].i;
        magnitude[i] = sqrtf((real * real) + (imag * imag));
    }

    free(buffer);
    kiss_fftr_free(cfg);
    free(freqdata);

    float low = 1.0f;
    float high = (float)(output_size - 1);
    float* boundaries = malloc((BAR_COUNT + 1) * sizeof(float));

    if (!boundaries)
    {
        free(magnitude);
        return NULL;
    }

    for (int i = 0; i <= BAR_COUNT; i++)
    {
        float fraction = (float)i / BAR_COUNT;
        boundaries[i] = low * powf(high / low, fraction);
    }

    float* bars = malloc(BAR_COUNT * sizeof(float));

    if (!bars)
    {
        free(magnitude);
        free(boundaries);
        return NULL;
    }

    for (int i = 0; i < BAR_COUNT; i++)
    {
        int start_bin = (int)roundf(boundaries[i]);
        int end_bin = (int)roundf(boundaries[i + 1]);

        if (start_bin == end_bin)
        {
            bars[i] = 0.0f;
            continue;
        }

        float max = 0.0f;
        for (int j = start_bin; j < end_bin; j++)
        {
            max = fmaxf(max, magnitude[j]);
        }

        bars[i] = max;
    }

    free(magnitude);
    free(boundaries);

    float max = 0.0f;
    for (int i = 0; i < BAR_COUNT; i++)
    {
        max = fmaxf(max, bars[i]);
    }

    if (max > 0.0f)
    {
        for (int i = 0; i < BAR_COUNT; i++)
        {
            bars[i] /= max;
        }
    }

    return bars;
}

float* miniaudio_equalizer_win(const wchar_t* path, double position_seconds)
{
    ma_decoder decoder;
    ma_decoder_config config = ma_decoder_config_init(ma_format_f32, 1, 0);
    ma_result result = ma_decoder_init_file_w(path, &config, &decoder);

    if (result != MA_SUCCESS)
    {
        return NULL;
    }

    ma_uint32 sample_rate;
    result = ma_decoder_get_data_format(&decoder, NULL, NULL, &sample_rate, NULL, 0);

    if (result != MA_SUCCESS)
    {
        ma_decoder_uninit(&decoder);
        return NULL;
    }

    ma_uint64 target_frame = (ma_uint64)(position_seconds * sample_rate);
    result = ma_decoder_seek_to_pcm_frame(&decoder, target_frame);

    printf("%i\n", sample_rate);
    printf("%llu\n", target_frame);

    if (result != MA_SUCCESS)
    {
        ma_decoder_uninit(&decoder);
        return NULL;
    }

    float* buffer = malloc(FFT_SIZE * sizeof(float));

    if (!buffer)
    {
        ma_decoder_uninit(&decoder);
        return NULL;
    }

    ma_uint64 frames_read;
    result = ma_decoder_read_pcm_frames(&decoder, buffer, FFT_SIZE, &frames_read);

    if (frames_read != FFT_SIZE)
    {
        puts("seeked too close to the end");
        free(buffer);
        ma_decoder_uninit(&decoder);
        return NULL;
    }

    ma_decoder_uninit(&decoder);
    
    kiss_fftr_cfg cfg = kiss_fftr_alloc(FFT_SIZE, 0, NULL, NULL);

    if (!cfg)
    {
        free(buffer);
        return NULL;
    }

    int output_size = (FFT_SIZE / 2) + 1;
    kiss_fft_cpx* freqdata = malloc(output_size * sizeof(kiss_fft_cpx));

    if (!freqdata)
    {
        free(buffer);
        kiss_fftr_free(cfg);
        return NULL;
    }

    kiss_fftr(cfg, buffer, freqdata);
    float* magnitude = malloc(output_size * sizeof(float));

    if (!magnitude)
    {
        free(buffer);
        kiss_fftr_free(cfg);
        free(freqdata);
        return NULL;
    }

    for (int i = 0; i < output_size; i++)
    {
        float real = freqdata[i].r;
        float imag = freqdata[i].i;
        magnitude[i] = sqrtf((real * real) + (imag * imag));
    }

    free(buffer);
    kiss_fftr_free(cfg);
    free(freqdata);

    float low = 1.0f;
    float high = (float)(output_size - 1);
    float* boundaries = malloc((BAR_COUNT + 1) * sizeof(float));

    if (!boundaries)
    {
        free(magnitude);
        return NULL;
    }

    for (int i = 0; i <= BAR_COUNT; i++)
    {
        float fraction = (float)i / BAR_COUNT;
        boundaries[i] = low * powf(high / low, fraction);
    }

    float* bars = malloc(BAR_COUNT * sizeof(float));

    if (!bars)
    {
        free(magnitude);
        free(boundaries);
        return NULL;
    }

    for (int i = 0; i < BAR_COUNT; i++)
    {
        int start_bin = (int)roundf(boundaries[i]);
        int end_bin = (int)roundf(boundaries[i + 1]);

        if (start_bin == end_bin)
        {
            bars[i] = 0.0f;
            continue;
        }

        float max = 0.0f;
        for (int j = start_bin; j < end_bin; j++)
        {
            max = fmaxf(max, magnitude[j]);
        }

        bars[i] = max;
    }

    free(magnitude);
    free(boundaries);

    float max = 0.0f;
    for (int i = 0; i < BAR_COUNT; i++)
    {
        max = fmaxf(max, bars[i]);
    }

    if (max > 0.0f)
    {
        for (int i = 0; i < BAR_COUNT; i++)
        {
            bars[i] /= max;
        }
    }

    return bars;
}

void miniaudio_free_equalizer(float* equalizer)
{
    free(equalizer);
}