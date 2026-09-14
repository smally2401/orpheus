#ifndef UI_VIEW_H
#define UI_VIEW_H

#include "../../vendor/nuklear.h"

#include "../backend/backend.h"
#include "ui_state.h"

void display_current_view(OrpheusBackend* backend, UiState* state,
                          struct nk_context* context);

#endif
