#include "position_slider.h"

#include <stdio.h>

void format_time(int time, char* out, size_t buf_size)
{
	int mins = time / 60;
	int secs = time % 60;

	snprintf(out, buf_size, "%i:%02i", mins, secs);
}

void display_position_slider(OrpheusBackend* backend,
                             struct nk_context* context)
{
	static bool dragging = false;

	static int pos = 0;
	int dur = (int)backend_get_duration_seconds(backend);

	const size_t buf_size = 32;
	char pos_str[buf_size];
	char dur_str[buf_size];
	format_time(pos, pos_str, buf_size);
	format_time(dur, dur_str, buf_size);

	nk_label(context, pos_str, NK_TEXT_CENTERED);
	nk_bool changed = nk_slider_int(context, 0, &pos, dur, 1);
	nk_label(context, dur_str, NK_TEXT_CENTERED);

	if (changed)
	{
		dragging = true;
	}

	if (dragging && !nk_input_is_mouse_down(&context->input, NK_BUTTON_LEFT))
	{
		dragging = false;
		backend_seek(backend, (float)pos);
	}

	if (!dragging)
	{
		pos = (int)backend_get_position_seconds(backend);
	}
}
