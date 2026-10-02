/* Pitex embedded preview engine — PDF object writer.
 * Written from the PDF 1.7 specification. Pitex-authored (PolyForm Shield). */
#ifndef PITEX_PDFW_H
#define PITEX_PDFW_H

#include <stdbool.h>
#include <stddef.h>
#include "pdfread.h"
#include "pitex_buf.h"

typedef struct pdfw pdfw;

pdfw *pdfw_new(pbuf *out);
void pdfw_free(pdfw *w);
pbuf *pdfw_out(pdfw *w);

int pdfw_alloc(pdfw *w);
/* Start "n 0 obj" at the current output position; end with pdfw_end. */
void pdfw_begin(pdfw *w, int num);
void pdfw_end(pdfw *w);
/* Write a complete stream object. `dict_body` is the dictionary content
 * without << >> and without /Length (and /Filter when compress is set). */
void pdfw_stream(pdfw *w, int num, const char *dict_body, const void *data, size_t len, bool compress);
/* Stream whose data is already Flate-compressed. */
void pdfw_stream_deflated(pdfw *w, int num, const char *dict_body, const void *data, size_t len);
/* Finish with xref table and trailer. */
void pdfw_finish(pdfw *w, int root, int info);

/* Name / string literal helpers. */
void pdfw_name(pbuf *b, const char *name);
void pdfw_hexstring(pbuf *b, const unsigned char *s, size_t len);

/* Deep copy of objects from an input PDF. Indirect references are
 * remapped to newly allocated objects which are written by
 * pdfw_import_flush. Dictionary keys that would pull in unrelated parts of
 * the source document (Parent, PieceInfo, Metadata, ...) are dropped. */
typedef struct pdfw_import pdfw_import;
pdfw_import *pdfw_import_begin(pdfw *w, pr_doc *src);
void pdfw_import_value(pdfw_import *imp, pbuf *b, pr_obj *obj);
void pdfw_import_flush(pdfw_import *imp);
void pdfw_import_end(pdfw_import *imp);

#endif
