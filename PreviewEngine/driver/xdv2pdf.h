/* Pitex embedded preview engine — XDV to PDF conversion.
 * Independently written from the DVI/XDV format descriptions and the PDF
 * 1.7 specification (no xdvipdfmx/dvipdfmx or MuPDF code).
 * Pitex-authored (AGPL-3.0-or-later). */
#ifndef PITEX_XDV2PDF_H
#define PITEX_XDV2PDF_H

#include <stddef.h>
#include "pitex_buf.h"
#include "tbuf.h"
#include "xdv.h"

typedef enum {
  RES_NATIVE_FONT, /* OpenType/TrueType file named in an XDV font def */
  RES_TFM,
  RES_VF,
  RES_ENC,
  RES_MAP,
  RES_TYPE1,       /* .pfb/.pfa named in a font map */
  RES_IMAGE        /* picture named in a pdf:image special */
} xdv_res_kind;

typedef struct {
  void *env;
  /* New reference to the file's current contents, or NULL. */
  tbuf *(*load)(void *env, const char *name, xdv_res_kind kind);
} xdv_resolver;

/* Pages [first, first + count) of one XDV buffer. */
typedef struct {
  const unsigned char *data;
  size_t len;
  const xdv_index *index;
  int first, count;
} xdv_range;

typedef struct xdv2pdf xdv2pdf;

xdv2pdf *xdv2pdf_new(xdv_resolver resolver);
void xdv2pdf_free(xdv2pdf *w);

/* Write a complete PDF containing the given page ranges in order. Problems
 * that only degrade the preview (missing font, unsupported special) are
 * appended to `warnings`, one per line. Returns 0 on success. */
int xdv2pdf_write(xdv2pdf *w, const xdv_range *ranges, int nranges, pbuf *pdf, pbuf *warnings);

#endif
