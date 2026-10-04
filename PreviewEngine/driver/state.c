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

/* Pitex modifications (2026): MuPDF fz_buffer/fz_context/fz_try replaced by
 * the Pitex tbuf type and plain allocation; rollback semantics unchanged. */

#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include "state.h"

/* Rollback log */

#undef LOG
#define LOG 0

enum log_action {
  LOG_ENTRY = 0x42,
  LOG_CELL,
  LOG_OVERWRITE
};

struct log_s
{
  mark_t snap;
  tbuf *data;
};

log_t *log_new(void)
{
  log_t *log = calloc(1, sizeof(log_t));
  if (!log)
    abort();
  log->data = tbuf_new(512);
  log->snap = 1;
  uint8_t zero = 0;
  tbuf_append(log->data, &zero, 1);
  return log;
}

void log_free(log_t *log)
{
  tbuf_drop(log->data);
  free(log);
}

#define PUSH_VALUE(buf, val) \
  tbuf_append(buf, &(val), sizeof(val))

static void pop_value(tbuf *buf, void *val, size_t len)
{
  if (buf->len < len) abort();
  buf->len -= len;
  memcpy(val, buf->data + buf->len, len);
}

#define POP_VALUE(buf, val) \
  pop_value(buf, &(val), sizeof(val))

static void push_action(tbuf *buf, enum log_action action)
{
  uint8_t b = action;
  PUSH_VALUE(buf, b);
}

void log_fileentry(log_t *log, fileentry_t *entry)
{
  if (entry->saved.snap != log->snap)
  {
    if (LOG) fprintf(stderr, "push LOG_ENTRY %s\n", entry->path);
    if (entry->saved.data)
    {
      tbuf_keep(entry->saved.data);
      PUSH_VALUE(log->data, entry->saved.data->len);
    }
    PUSH_VALUE(log->data, entry->saved);
    PUSH_VALUE(log->data, entry);
    push_action(log->data, LOG_ENTRY);
    entry->saved.snap = log->snap;
  }
}

void log_filecell(log_t *log, filecell_t *cell)
{
  if (cell->snap != log->snap)
  {
    if (LOG) fprintf(stderr, "push LOG_CELL\n");
    PUSH_VALUE(log->data, *cell);
    PUSH_VALUE(log->data, cell);
    push_action(log->data, LOG_CELL);
    cell->snap = log->snap;
  }
}

struct overwrite_data {
  tbuf *buf;
  int start, len;
};

void log_overwrite(log_t *log, tbuf *buf, int start, int len)
{
  if (LOG) fprintf(stderr, "push LOG_OVERWRITE\n");
  tbuf_keep(buf);
  tbuf_append(log->data, buf->data + start, len);
  struct overwrite_data data = {
    .buf = buf,
    .start = start,
    .len = len,
  };
  PUSH_VALUE(log->data, data);
  push_action(log->data, LOG_OVERWRITE);
}

static enum log_action pop_action(tbuf *buf)
{
  uint8_t b;
  POP_VALUE(buf, b);
  return b;
}

static void log_pop(log_t *log)
{
  switch (pop_action(log->data))
  {
    case LOG_ENTRY:
    {
      fileentry_t *entry;
      POP_VALUE(log->data, entry);
      if (LOG) fprintf(stderr, "pop LOG_ENTRY %s\n", entry->path);
      if (entry->saved.data)
        tbuf_drop(entry->saved.data);
      POP_VALUE(log->data, entry->saved);
      if (entry->saved.data)
      {
        POP_VALUE(log->data, entry->saved.data->len);
      }
      break;
    }
    case LOG_CELL:
    {
      if (LOG) fprintf(stderr, "pop LOG_CELL\n");
      filecell_t *cell;
      POP_VALUE(log->data, cell);
      POP_VALUE(log->data, *cell);
      break;
    }
    case LOG_OVERWRITE:
    {
      if (LOG) fprintf(stderr, "pop LOG_OVERWRITE\n");
      struct overwrite_data data;
      POP_VALUE(log->data, data);
      pop_value(log->data, data.buf->data + data.start, data.len);
      tbuf_drop(data.buf);
      break;
    }
    default:
      abort();
  }
}

mark_t log_snapshot(log_t *log)
{
  return (log->snap = log->data->len);
}

void log_rollback(log_t *log, mark_t mark)
{
  if (mark > log->snap) abort();

  while (log->data->len > (size_t)mark)
    log_pop(log);

  if ((size_t)mark != log->data->len)
  {
    fprintf(stderr, "[fatal] rollback: mark=%d len =%d\n", mark, (int)log->data->len);
    abort();
  }

  log->snap = mark;
}

/* State */

void state_init(state_t *st)
{
  memset(st, 0, sizeof(state_t));
}

static bool
same_time(struct timespec a, struct timespec b)
{
  return (a.tv_sec == b.tv_sec) && (a.tv_nsec == b.tv_nsec);
}

#ifndef __APPLE__
# define st_time(a) st_##a##tim
#else
# define st_time(a) st_##a##timespec
#endif

bool stat_same(struct stat *st1, struct stat *st2)
{
  return st1->st_dev     == st2->st_dev &&
         st1->st_ino     == st2->st_ino &&
         st1->st_mode    == st2->st_mode &&
         st1->st_nlink   == st2->st_nlink &&
         st1->st_uid     == st2->st_uid &&
         st1->st_gid     == st2->st_gid &&
         st1->st_rdev    == st2->st_rdev &&
         st1->st_size    == st2->st_size &&
         st1->st_blksize == st2->st_blksize &&
         st1->st_blocks  == st2->st_blocks &&
         same_time(st1->st_time(a), st2->st_time(a)) &&
         same_time(st1->st_time(m), st2->st_time(m)) &&
         same_time(st1->st_time(c), st2->st_time(c));
}
