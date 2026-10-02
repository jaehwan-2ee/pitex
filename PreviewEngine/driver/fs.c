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

/* Pitex modifications (2026): MuPDF allocation helpers replaced by calloc;
 * fz_buffer replaced by tbuf. Hash table logic unchanged. */

#include <stdlib.h>
#include <string.h>
#include "state.h"

static unsigned long
sdbm_hash(const unsigned char *str)
{
  unsigned long hash = 0;
  int c;

  while ((c = *str++))
    hash = c + (hash << 6) + (hash << 16) - hash;

  return hash * 2654435761;
}

#define str_hash sdbm_hash

typedef struct
{
  unsigned long hash;
  fileentry_t *entry;
} tablecell;

struct filesystem_s
{
  int count, cap;
  tablecell *table;
};

static void *xcalloc(size_t n, size_t size)
{
  void *p = calloc(n, size);
  if (!p)
    abort();
  return p;
}

static const char *normalize_path(const char *path)
{
  if (path[0] == '.' && path[1] == '/')
  {
    path += 2;
    while (*path == '/')
      path += 1;
  }
  return path;
}

filesystem_t *filesystem_new(void)
{
  filesystem_t *fs = xcalloc(1, sizeof(filesystem_t));
  fs->cap = 64;
  fs->table = xcalloc(64, sizeof(tablecell));
  return fs;
}

void filesystem_free(filesystem_t *fs)
{
  int cap = fs->cap;
  for (int i = 0; i < cap; ++i)
  {
    fileentry_t *e = fs->table[i].entry;
    if (!e)
      continue;
    tbuf_drop(e->fs_data);
    tbuf_drop(e->edit_data);
    tbuf_drop(e->saved.data);
    free((void *)e->path);
    free(fs->table[i].entry);
  }
  free(fs->table);
  free(fs);
}

static tablecell *table_get(int cap, tablecell *table, const char *path)
{
  unsigned long mask = cap - 1;
  unsigned long hash = str_hash((const unsigned char*)path);

  int index = hash & mask;

  while (table[index].entry)
  {
    if (table[index].hash == hash && strcmp(table[index].entry->path, path) == 0)
      break;
    index = (index + 1) & mask;
  }
  table[index].hash = hash;
  return &table[index];
}

static tablecell *table_resize(int oldcap, tablecell *oldtab, int newcap)
{
  tablecell *newtab = xcalloc(newcap, sizeof(tablecell));
  int mask = newcap - 1;
  for (int i = 0; i < oldcap; ++i)
  {
    if (!oldtab[i].entry)
      continue;
    tablecell cell = oldtab[i];
    int index = cell.hash & mask;
    while (newtab[index].entry)
    {
      if ((cell.hash & mask) < (newtab[index].hash & mask))
      {
        tablecell tmp = newtab[index];
        newtab[index] = cell;
        cell = tmp;
      }
      index = (index + 1) & mask;
    }
    newtab[index] = cell;
  }
  return newtab;
}

fileentry_t *filesystem_lookup(filesystem_t *fs, const char *path)
{
  return table_get(fs->cap, fs->table, normalize_path(path))->entry;
}

fileentry_t *filesystem_lookup_or_create(filesystem_t *fs, const char *path)
{
  path = normalize_path(path);
  tablecell *cell = table_get(fs->cap, fs->table, path);
  fileentry_t *entry = cell->entry;

  if (entry != NULL) return entry;

  entry = xcalloc(1, sizeof(fileentry_t));
  entry->path = strdup(path);
  if (!entry->path)
    abort();
  entry->saved.level = FILE_NONE;
  entry->seen = -1;
  entry->pic_cache.type = -1;
  entry->fs_stat.st_ino = 0;
  entry->debug_rollback_invalidation = -1;
  cell->entry = entry;

  fs->count += 1;
  if (fs->count * 4 >= fs->cap * 3)
  {
    int newcap = fs->cap * 2;
    tablecell *newtab = table_resize(fs->cap, fs->table, newcap);
    free(fs->table);
    fs->cap = newcap;
    fs->table = newtab;
  }

  return entry;
}

fileentry_t *filesystem_scan(filesystem_t *fs, int *index)
{
  while (*index < fs->cap)
  {
    int i = *index;
    *index += 1;
    if (fs->table[i].entry)
      return fs->table[i].entry;
  }
  return NULL;
}
