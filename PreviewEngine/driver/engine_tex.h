/*
 * MIT License
 *
 * Copyright (c) 2023 Frédéric Bour <frederic.bour@lakaban.net>
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to
 * deal in the Software without restriction, including without limitation the
 * rights to use, copy, modify, merge, publish, distribute, sublicense, and/or
 * sell copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in
 * all copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
 * FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS
 * IN THE SOFTWARE.
 */

/* Pitex modifications (2026): derived from TeXpresso src/frontend/engine.h
 * (txp_engine class). The generic engine class, MuPDF display-list rendering
 * and SDL/editor coupling were removed; only the TeX engine remains, exposed
 * as plain functions to the Pitex preview driver. */

#ifndef PITEX_ENGINE_TEX_H
#define PITEX_ENGINE_TEX_H

#include <stdbool.h>
#include "state.h"
#include "xdv.h"

typedef enum {
  DOC_RUNNING,
  DOC_TERMINATED
} txp_engine_status;

typedef struct tex_engine tex_engine;

tex_engine *txp_engine_new(const char *engine_path,
                           const char *inclusion_path, const char *tex_name);
void txp_engine_free(tex_engine *self);

/* Process at most one engine query (waiting up to 10 ms for it). Returns
 * true if a query was answered. */
bool txp_engine_step(tex_engine *self, bool restart_if_needed);

void txp_engine_begin_changes(tex_engine *self);
void txp_engine_detect_changes(tex_engine *self);
void txp_engine_scan_file(tex_engine *self, fileentry_t *e);
bool txp_engine_end_changes(tex_engine *self);
void txp_engine_notify_file_changes(tex_engine *self, fileentry_t *entry, int offset);
fileentry_t *txp_engine_find_file(tex_engine *self, const char *path);

txp_engine_status txp_engine_get_status(tex_engine *self);
bool txp_engine_aux_dirty(tex_engine *self);
bool txp_engine_is_finishing(tex_engine *self);
void txp_engine_start_finishing(tex_engine *self);
/* Returns true if a convergence rerun was started. */
bool txp_engine_finish_convergence(tex_engine *self);

/* Pitex additions */
xdv_index *txp_engine_xdv(tex_engine *self);
tbuf *txp_engine_document(tex_engine *self); /* XDV bytes, may be NULL */
tbuf *txp_engine_synctex(tex_engine *self);
tbuf *txp_engine_log(tex_engine *self);
filesystem_t *txp_engine_fs(tex_engine *self);
/* File descriptor of the running engine process, or -1. */
int txp_engine_fd(tex_engine *self);
/* Milliseconds since the running process last sent a query. */
int txp_engine_idle_ms(tex_engine *self);
/* Kill the running process (watchdog / stuck detection). */
void txp_engine_kill_running(tex_engine *self);
/* Drop every snapshot and restart from a freshly exec'd engine. */
void txp_engine_restart(tex_engine *self);
/* True if the process currently running was produced by fork() (a
 * checkpoint child) rather than exec. */
bool txp_engine_running_forked(tex_engine *self);
/* Exit status of the last reaped engine process: -1 unknown, otherwise
 * the raw waitpid status. */
int txp_engine_last_status(tex_engine *self);
/* True once after a checkpoint child reported a platform-font barrier; the
 * caller then restarts the engine (txp_engine_restart). */
bool txp_engine_take_barrier(tex_engine *self);

#endif
