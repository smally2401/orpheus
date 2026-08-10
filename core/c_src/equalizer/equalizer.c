#include "kiss_fft.h"
#include "kiss_fftr.h"
#include "../audio/miniaudio.h"

#define FFT_SIZE 1024
#define FRAME_AMOUNT 500000

float* miniaudio_equalizer(const char* path)
{
    ma_decoder decoder;
    ma_decoder_config config = ma_decoder_config_init(ma_format_f32, 1, 0);
    ma_result result = ma_decoder_init_file(path, &config, &decoder);

    if (result != MA_SUCCESS)
    {
        return NULL;
    }

    result = ma_decoder_seek_to_pcm_frame(&decoder, FRAME_AMOUNT);

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
        printf("%f\n", magnitude[i]);
    }

    free(buffer);
    kiss_fftr_free(cfg);
    free(freqdata);

    return magnitude;
}