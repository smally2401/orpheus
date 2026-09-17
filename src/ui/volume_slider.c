#include "volume_slider.h"

#include <stdio.h>

void display_volume_slider(OrpheusBackend* backend, struct nk_context* context)
{
	static float pos = 1.0F;
	char pos_str[8];
	snprintf(pos_str, sizeof(pos_str), "%.2f", pos);

	nk_bool changed = nk_slider_float(context, 0.0F, &pos, 1.0F, 0.01F);
	nk_label(context, pos_str, NK_TEXT_CENTERED);

	if (changed)
	{
		backend_set_volume(backend, pos);
	}
}
