/* Pitex embedded preview engine — reference-counted byte buffer.
 * Pitex-authored (PolyForm Shield 1.0.0). */
#include "tbuf.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

tbuf *tbuf_new(size_t cap)
{
  tbuf *b = calloc(1, sizeof *b);
  if (!b)
    abort();
  b->cap = cap ? cap : 16;
  b->data = malloc(b->cap);
  if (!b->data)
    abort();
  b->refs = 1;
  return b;
}

tbuf *tbuf_from_copy(const void *data, size_t len)
{
  tbuf *b = tbuf_new(len + 1);
  if (len)
    memcpy(b->data, data, len);
  b->len = len;
  return b;
}

tbuf *tbuf_read_file(const char *path)
{
  FILE *f = fopen(path, "rb");
  if (!f)
    return NULL;
  tbuf *b = tbuf_new(4096);
  for (;;)
  {
    if (b->len == b->cap)
    {
      b->cap *= 2;
      b->data = realloc(b->data, b->cap);
      if (!b->data)
        abort();
    }
    size_t n = fread(b->data + b->len, 1, b->cap - b->len, f);
    if (n == 0)
      break;
    b->len += n;
  }
  int err = ferror(f);
  fclose(f);
  if (err)
  {
    tbuf_drop(b);
    return NULL;
  }
  return b;
}

tbuf *tbuf_keep(tbuf *b)
{
  if (b)
    b->refs++;
  return b;
}

void tbuf_drop(tbuf *b)
{
  if (!b || --b->refs > 0)
    return;
  free(b->data);
  free(b);
}

void tbuf_append(tbuf *b, const void *data, size_t len)
{
  if (b->len + len > b->cap)
  {
    size_t cap = b->cap;
    while (cap < b->len + len)
      cap *= 2;
    b->data = realloc(b->data, cap);
    if (!b->data)
      abort();
    b->cap = cap;
  }
  memcpy(b->data + b->len, data, len);
  b->len += len;
}
