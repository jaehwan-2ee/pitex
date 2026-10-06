/* Pitex embedded preview engine — raster images to PDF image XObjects.
 * Independently written from the PNG, JPEG/JFIF and BMP specifications.
 * Pitex-authored (AGPL-3.0-or-later). */
#ifndef PITEX_IMAGES_H
#define PITEX_IMAGES_H

#include <stdbool.h>
#include "pitex_buf.h"

typedef struct {
  bool ok;
  int width, height;
  double xdpi, ydpi;
  char dict[192];   /* /Width../Height../ColorSpace../BitsPerComponent..[/Decode..] */
  bool dct;         /* data is JPEG (DCTDecode); otherwise Flate-compressed */
  pbuf data;
  pbuf smask;       /* Flate-compressed 8-bit alpha, empty when opaque */
} pdf_image;

/* Returns 0 on success; unsupported variants append a warning. */
int image_encode(const unsigned char *data, size_t len, const char *name, pdf_image *out, pbuf *warnings);
void image_free(pdf_image *img);

#endif
