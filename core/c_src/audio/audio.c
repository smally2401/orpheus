// C-side implementation backing `MiniAudioPlayer` in `audio_player.rs`.
// Wraps a single `ma_engine` + `ma_sound` pair: only one sound is ever
// loaded at a time (see `miniaudio_load_file`), matching how
// `LocalBackend::load_track` uses it: always stop then append, never
// queuing multiple sounds concurrently.

#define MINIAUDIO_IMPLEMENTATION
#include "miniaudio.h"
#include "../headers/eq_state.h"

typedef struct
{
	ma_engine engine;
	ma_sound sound;
	// Only meaningful once a sound has been loaded via
	// `miniaudio_load_file`/`_win`, `sound` itself is otherwise
	// uninitialized.
	bool has_sound;
	// Mirrors "paused" as reported to Rust via `miniaudio_is_paused`.
	// Set by both `miniaudio_pause` and `miniaudio_stop`.
	bool paused;
	// Persistent equalizer decoder + FFT state. Tied to the lifetime of
	// the currently loaded sound.
	EqState eq;
} MiniAudioPlayerC;

// Allocates a player and initializes its `ma_engine` on the default
// audio device. Returns NULL on allocation failure or engine init
// failure (caller side asserts non-null, see `MiniAudioPlayer::new`).
MiniAudioPlayerC* miniaudio_create(void)
{
	MiniAudioPlayerC* player = (MiniAudioPlayerC*)calloc(1, sizeof(MiniAudioPlayerC));

	if (!player)
	{
		return NULL;
	}

	ma_result result = ma_engine_init(NULL, &player->engine);

	if (result != MA_SUCCESS)
	{
		free(player);
		return NULL;
	}

	return player;
}

// Tears down the current sound (if any), the equalizer state, and the
// enfine, then frees the player itself
void miniaudio_destroy(MiniAudioPlayerC* player)
{
	if (!player)
	{
		return;
	}

	eq_state_uninit(&player->eq);

	if (player->has_sound)
	{
		ma_sound_uninit(&player->sound);
	}

	ma_engine_uninit(&player->engine);
	free(player);
}

// Loads `path` as the player's active sound, replacing whatever was
// previously loaded (uninits the old `ma_sound` and old `EqState`
// first, if any). Uses flag `0`, i.e. streaming decode rather than
// up-front full decode.
bool miniaudio_load_file(MiniAudioPlayerC* player, const char* path)
{
	if (!player)
	{
		return false;
	}

	if (player->has_sound)
	{
		ma_sound_uninit(&player->sound);
		player->has_sound = false;
	}

	eq_state_uninit(&player->eq);
	ma_result result = ma_sound_init_from_file(&player->engine, path, 0, NULL, NULL, &player->sound);

	if (result == MA_SUCCESS)
	{
		player->has_sound = true;
		player->paused = false;
		eq_state_init(&player->eq, path);
		return true;
	}

	return false;
}

// Windows counterpart to `miniaudio_load_file`, taking a wide
// (UTF-16) path via `ma_sound_init_from_file_w`.
bool miniaudio_load_file_win(MiniAudioPlayerC* player, const wchar_t* path)
{
	if (!player)
	{
		return false;
	}

	if (player->has_sound)
	{
		ma_sound_uninit(&player->sound);
		player->has_sound = false;
	}

	eq_state_uninit(&player->eq);
	ma_result result = ma_sound_init_from_file_w(&player->engine, path, 0, NULL, NULL, &player->sound);

	if (result == MA_SUCCESS)
	{
		player->has_sound = true;
		player->paused = false;
		eq_state_init_win(&player->eq, path);
		return true;
	}

	return false;
}

// Resumes/starts playback of the current sound from wherever its
// cursor currently is. No-op if nothing is loaded.
void miniaudio_play(MiniAudioPlayerC* player)
{
	if (player && player->has_sound)
	{
		ma_sound_start(&player->sound);
		player->paused = false;
	}
}

// Halts playback in place: position is left untouched, so a
// subsequent `miniaudio_play` resumes from where it stopped. Distinct
// from `miniaudio_stop`, which additionally resets position to zero.
void miniaudio_pause(MiniAudioPlayerC* player)
{
	if (player && player->has_sound)
	{
		ma_sound_stop(&player->sound);
		player->paused = true;
	}
}

// Halts playback and resets position to the start of the track. Used
// internally by `load_track` before swapping in a new sound.
void miniaudio_stop(MiniAudioPlayerC* player)
{
	if (player && player->has_sound)
	{
		ma_sound_stop(&player->sound);
		ma_sound_seek_to_pcm_frame(&player->sound, 0);
		player->paused = true;
		eq_state_uninit(&player->eq);
	}
}

// Seeks to an absolute position in seconds, converting to a PCM frame
// index using the engine's sample rate. Also re-syncs the equalizer
// decoder so the visualizer doesn't drift from the audio.
bool miniaudio_seek_seconds(MiniAudioPlayerC* player, double seconds)
{
	if (!player || !player->has_sound)
	{
		return false;
	}

	ma_uint32 sample_rate = ma_engine_get_sample_rate(&player->engine);
	ma_uint64 frame = (ma_uint64)(seconds * (double)sample_rate);

	bool sound_ok = ma_sound_seek_to_pcm_frame(&player->sound, frame) == MA_SUCCESS;
	eq_state_seek(&player->eq, seconds);

	return sound_ok;
}

// Current playback position in seconds, derived from the sound's PCM
// cursor and the engine's sample rate. Returns 0.0 if nothing is
// loaded.
double miniaudio_get_position_seconds(MiniAudioPlayerC* player)
{
	if (!player || !player->has_sound)
	{
		return 0.0;
	}

	ma_uint64 cursor = 0;
	ma_sound_get_cursor_in_pcm_frames(&player->sound, &cursor);
	ma_uint32 sample_rate = ma_engine_get_sample_rate(&player->engine);

	return (double)cursor / (double)sample_rate;
}

// True if the current sound has played to its end or if nothing is
// loaded at all.
bool miniaudio_is_empty(MiniAudioPlayerC* player)
{
	if (!player || !player->has_sound)
	{
		return true;
	}

	return ma_sound_at_end(&player->sound) != 0;
}

// Reports the `paused` flag tracked on this struct. A NULL player
// counts as paused.
bool miniaudio_is_paused(MiniAudioPlayerC* player)
{
	if (!player || player->paused)
	{
		return true;
	}

	return false;
}

// Engine-level volume (0.0-1.0).
float miniaudio_volume(MiniAudioPlayerC* player)
{
	if (!player)
	{
		return 0.0f;
	}

	return ma_engine_get_volume(&player->engine);
}

void miniaudio_set_volume(MiniAudioPlayerC* player, float volume)
{
	if (!player)
	{
		return;
	}

	ma_engine_set_volume(&player->engine, volume);
}

bool miniaudio_equalizer_tick(MiniAudioPlayerC* player, float** out_bars, int* out_bar_count)
{
	if (!player)
	{
		return false;
	}

	return eq_state_tick(&player->eq, out_bars, out_bar_count);
}

float* miniaudio_waveform(const char* path, ma_uint64 bucket_count, bool use_rms, ma_uint64* out_bucket_count)
{
	*out_bucket_count = 0;
	if (bucket_count == 0)
	{
		return NULL;
	}

	ma_decoder decoder;
	ma_decoder_config config = ma_decoder_config_init(ma_format_f32, 1, 0);
	ma_result result = ma_decoder_init_file(path, &config, &decoder);

	if (result != MA_SUCCESS)
	{
		return NULL;
	}

	ma_uint64 total_frames;
	result = ma_decoder_get_length_in_pcm_frames(&decoder, &total_frames);

	if (result != MA_SUCCESS)
	{
		ma_decoder_uninit(&decoder);
		return NULL;
	}

	float* buffer = malloc(total_frames * sizeof(float));
	ma_uint64 frames_read;

	if (!buffer)
	{
		ma_decoder_uninit(&decoder);
		return NULL;
	}

	result = ma_decoder_read_pcm_frames(&decoder, buffer, total_frames, &frames_read);

	if (frames_read == 0 || (result != MA_SUCCESS && result != MA_AT_END))
	{
		free(buffer);
		ma_decoder_uninit(&decoder);
		return NULL;
	}

	if (bucket_count > frames_read)
	{
		bucket_count = frames_read;
	}

	ma_uint64 frames_per_bucket = frames_read / bucket_count;
	float* output_buckets = malloc(bucket_count * sizeof(float));

	if (!output_buckets)
	{
		free(buffer);
		ma_decoder_uninit(&decoder);
		return NULL;
	}

	for (ma_uint64 bucket_index = 0; bucket_index < bucket_count; bucket_index++)
	{		
		ma_uint64 start_frame = bucket_index * frames_per_bucket;
		ma_uint64 end_frame =
			bucket_index == bucket_count - 1 ? frames_read : start_frame + frames_per_bucket;

		if (!use_rms)
		{
			float sample = 0.0f;
			for (ma_uint64 sample_index = start_frame; sample_index < end_frame; sample_index++)
			{
				sample = fmaxf(sample, fabsf(buffer[sample_index]));
			}

			output_buckets[bucket_index] = sample;
		}
		else
		{
			float sum_of_squares = 0.0f;
			for (ma_uint64 sample_index = start_frame; sample_index < end_frame; sample_index++)
			{
				sum_of_squares += buffer[sample_index] * buffer[sample_index];
			}

			ma_uint64 count = end_frame - start_frame;
			output_buckets[bucket_index] = sqrtf(sum_of_squares / (float)count);
		}
	}

	float max = 0.0f;
	for (ma_uint64 i = 0; i < bucket_count; i++)
	{
		max = fmaxf(max, output_buckets[i]);
	}

	if (max > 0.0f)
	{
		for (ma_uint64 i = 0; i < bucket_count; i++)
		{
			output_buckets[i] /= max;
		}
	}

	free(buffer);
	ma_decoder_uninit(&decoder);

	*out_bucket_count = bucket_count;
	return output_buckets;
}

float* miniaudio_waveform_win(const wchar_t* path, ma_uint64 bucket_count, bool use_rms, ma_uint64* out_bucket_count)
{
	if (bucket_count == 0)
	{
		return NULL;
	}

	ma_decoder decoder;
	ma_decoder_config config = ma_decoder_config_init(ma_format_f32, 1, 0);
	ma_result result = ma_decoder_init_file_w(path, &config, &decoder);

	if (result != MA_SUCCESS)
	{
		return NULL;
	}

	ma_uint64 total_frames;
	result = ma_decoder_get_length_in_pcm_frames(&decoder, &total_frames);

	if (result != MA_SUCCESS)
	{
		ma_decoder_uninit(&decoder);
		return NULL;
	}

	float* buffer = malloc(total_frames * sizeof(float));
	ma_uint64 frames_read;

	if (!buffer)
	{
		ma_decoder_uninit(&decoder);
		return NULL;
	}

	result = ma_decoder_read_pcm_frames(&decoder, buffer, total_frames, &frames_read);

	if (frames_read == 0 || (result != MA_SUCCESS && result != MA_AT_END))
	{
		free(buffer);
		ma_decoder_uninit(&decoder);
		return NULL;
	}

	if (bucket_count > frames_read)
	{
		bucket_count = frames_read;
	}

	ma_uint64 frames_per_bucket = frames_read / bucket_count;
	float* output_buckets = malloc(bucket_count * sizeof(float));

	if (!output_buckets)
	{
		free(buffer);
		ma_decoder_uninit(&decoder);
		return NULL;
	}

	for (ma_uint64 i = 0; i < bucket_count; i++)
	{		
		ma_uint64 start_frame = i * frames_per_bucket;
		ma_uint64 end_frame =
			i == bucket_count - 1 ? frames_read : start_frame + frames_per_bucket;

		if (!use_rms)
		{
			float sample = 0.0f;
			for (ma_uint64 j = start_frame; j < end_frame; j++)
			{
				sample = fmaxf(sample, fabsf(buffer[j]));
			}

			output_buckets[i] = sample;
		}
		else
		{
			float sum_of_squares = 0.0f;
			for (ma_uint64 j = start_frame; j < end_frame; j++)
			{
				sum_of_squares += buffer[j] * buffer[j];
			}

			ma_uint64 count = end_frame - start_frame;
			output_buckets[i] = sqrtf(sum_of_squares / (float)count);
		}
	}

	float max = 0.0f;
	for (ma_uint64 i = 0; i < bucket_count; i++)
	{
		max = fmaxf(max, output_buckets[i]);
	}

	if (max > 0.0f)
	{
		for (ma_uint64 i = 0; i < bucket_count; i++)
		{
			output_buckets[i] /= max;
		}
	}

	free(buffer);
	ma_decoder_uninit(&decoder);

	*out_bucket_count = bucket_count;
	return output_buckets;
}

void miniaudio_free_waveform(float* waveform)
{
	free(waveform);
}