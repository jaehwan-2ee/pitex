/* Pitex embedded preview engine — raster image header parsing.
 * Independently written from the PNG (ISO/IEC 15948), JPEG/JFIF/Exif and
 * BMP format specifications. Pitex-authored (AGPL-3.0-or-later). */
#ifndef PITEX_IMGINFO_H
#define PITEX_IMGINFO_H

#include <stddef.h>

typedef enum { PITEX_IMG_NONE, PITEX_IMG_PNG, PITEX_IMG_JPEG, PITEX_IMG_BMP } pitex_img_kind;

typedef struct {
  pitex_img_kind kind;
  unsigned width, height;
  /* Horizontal/vertical resolution in dots per inch; 72 when the file does
   * not record one (the dvipdfmx convention XeTeX sizes pictures with). */
  double xdpi, ydpi;
} pitex_img_info;

pitex_img_kind pitex_img_sniff(const unsigned char *data, size_t len);
/* Returns 0 on success. */
int pitex_img_info_read(const unsigned char *data, size_t len, pitex_img_info *out);

#endif
