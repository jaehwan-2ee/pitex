/* Pitex embedded preview engine — reference-counted byte buffer.
 * Replaces MuPDF's fz_buffer in the TeXpresso-derived session code (the
 * rollback log keeps extra references to file buffers and truncates their
 * length on rollback, exactly the semantics fz_buffer provided there).
 * Pitex-authored (AGPL-3.0-or-later). */
#ifndef PITEX_TBUF_H
#define PITEX_TBUF_H

#include <stddef.h>

typedef struct {
  unsigned char *data;
  size_t len, cap;
  int refs;
} tbuf;

tbuf *tbuf_new(size_t cap);
tbuf *tbuf_from_copy(const void *data, size_t len);
/* NULL if the file cannot be read. */
tbuf *tbuf_read_file(const char *path);
tbuf *tbuf_keep(tbuf *b);
void tbuf_drop(tbuf *b);
void tbuf_append(tbuf *b, const void *data, size_t len);

#endif
