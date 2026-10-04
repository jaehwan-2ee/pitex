/* Pitex embedded preview engine — growable byte buffer.
 * Pitex-authored (PolyForm Shield 1.0.0, see repository LICENSE). */
#ifndef PITEX_BUF_H
#define PITEX_BUF_H

#include <stdarg.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  unsigned char *data;
  size_t len, cap;
} pbuf;

static inline void pbuf_reserve(pbuf *b, size_t extra)
{
  if (b->len + extra <= b->cap)
    return;
  size_t cap = b->cap ? b->cap : 256;
  while (cap < b->len + extra)
    cap *= 2;
  unsigned char *p = realloc(b->data, cap);
  if (!p)
    abort();
  b->data = p;
  b->cap = cap;
}

static inline void pbuf_append(pbuf *b, const void *data, size_t len)
{
  if (!len)
    return;
  pbuf_reserve(b, len);
  memcpy(b->data + b->len, data, len);
  b->len += len;
}

static inline void pbuf_putc(pbuf *b, int c)
{
  pbuf_reserve(b, 1);
  b->data[b->len++] = (unsigned char)c;
}

static inline void pbuf_puts(pbuf *b, const char *s) { pbuf_append(b, s, strlen(s)); }

void pbuf_printf(pbuf *b, const char *fmt, ...)
#if defined(__GNUC__)
    __attribute__((format(printf, 2, 3)))
#endif
    ;

static inline void pbuf_free(pbuf *b)
{
  free(b->data);
  b->data = NULL;
  b->len = b->cap = 0;
}

static inline void pbuf_clear(pbuf *b) { b->len = 0; }

/* Append a real number in compact PDF syntax (no exponent). */
void pbuf_real(pbuf *b, double v);

/* zlib helpers (deflate at `level`, inflate with a 1 GiB output cap). */
int pbuf_deflate(pbuf *out, const unsigned char *data, size_t len, int level);
int pbuf_inflate(pbuf *out, const unsigned char *data, size_t len);

#endif
