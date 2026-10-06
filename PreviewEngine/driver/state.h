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

/* Pitex modifications (2026): MuPDF fz_buffer/fz_context replaced by the
 * Pitex tbuf type; no functional change to the VFS/rollback model. */

#ifndef STATE_H
#define STATE_H

#include <stdbool.h>
#include <sys/stat.h>
#include "sprotocol.h"
#include "tbuf.h"

#define MAX_FILES 1024

enum accesslevel {
  FILE_NONE,
  FILE_READ,
  FILE_WRITE
};

typedef int mark_t;

typedef struct fileentry_s {
  const char *path;

  // Cache of filesystem state
  struct stat fs_stat;
  tbuf *fs_data;

  // Cached picture information
  struct pic_cache pic_cache;

  // State of the file in the text editor (or NULL if unedited)
  tbuf *edit_data;
  bool promised;
  bool edit_data_from_convergence;

  // State observed and/or produced by TeX process
  struct {
    tbuf *data;
    enum accesslevel level;
    mark_t snap;
  } saved;

  int seen;
  int debug_rollback_invalidation;
} fileentry_t;

typedef struct filecell_s {
  mark_t snap;
  fileentry_t *entry;
} filecell_t;

typedef struct {
  filecell_t table[MAX_FILES];
  filecell_t stdout, document, synctex, log;
} state_t;

void state_init(state_t *st);

typedef struct filesystem_s filesystem_t;
typedef struct log_s log_t;

filesystem_t *filesystem_new(void);
void filesystem_free(filesystem_t *fs);
fileentry_t *filesystem_lookup_or_create(filesystem_t *fs, const char *path);
fileentry_t *filesystem_lookup(filesystem_t *fs, const char *path);
fileentry_t *filesystem_scan(filesystem_t *fs, int *index);

log_t *log_new(void);
void log_free(log_t *log);
mark_t log_snapshot(log_t *log);
void log_rollback(log_t *log, mark_t snapshot);
void log_fileentry(log_t *log, fileentry_t *entry);
void log_filecell(log_t *log, filecell_t *cell);
void log_overwrite(log_t *log, tbuf *buf, int start, int len);

bool stat_same(struct stat *st1, struct stat *st2);

#endif /*!STATE_H*/
