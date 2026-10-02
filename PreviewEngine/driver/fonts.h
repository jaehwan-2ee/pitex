/* Pitex embedded preview engine — font file readers for PDF embedding.
 * Independently written from the OpenType/TrueType (ISO/IEC 14496-22),
 * TFM/VF (Knuth: tftopl, vftovp), Adobe Type 1 (PFB/PFA) and pdfTeX font
 * map format descriptions. Pitex-authored (PolyForm Shield 1.0.0). */
#ifndef PITEX_FONTS_H
#define PITEX_FONTS_H

#include <stdbool.h>
#include <stdint.h>
#include "pitex_buf.h"
#include "tbuf.h"
#include "xdv2pdf.h"

typedef struct font_cache font_cache;

font_cache *font_cache_new(xdv_resolver resolver);
void font_cache_free(font_cache *fc);

/* OpenType/TrueType face (one member of a collection). */
typedef struct {
  bool ok;
  bool cff;               /* CFF outlines ('OTTO') */
  unsigned char *sfnt;    /* single-face sfnt bytes (owned) */
  size_t sfnt_len;
  int units_per_em;
  int num_glyphs;
  int num_hmetrics;
  const unsigned char *hmtx; /* into sfnt */
  size_t hmtx_len;
  uint32_t *to_unicode;   /* gid -> first code point, 0 if none */
  double bbox[4];         /* 1000-unit glyph space */
  double ascent, descent, cap_height, italic_angle;
  char psname[128];
  pbuf deflated;          /* compressed sfnt for FontFile2/3, filled lazily */
} native_face;

/* NULL-free: returns a face with ok=false (and a warning emitted once) when
 * the file cannot be used. */
native_face *font_native(font_cache *fc, const char *name, int index, pbuf *warnings);
int native_advance(const native_face *f, int gid); /* font units */

typedef struct {
  bool ok;
  int bc, ec;
  uint32_t checksum;
  int32_t design;          /* fix_word */
  int32_t width[256];      /* fix_word relative to design size */
  bool exists[256];
} tfm_font;

tfm_font *font_tfm(font_cache *fc, const char *name);

typedef struct {
  char *tfm, *psname, *fontfile, *encfile;
  double slant, extend;
} map_entry;

const map_entry *font_map_lookup(font_cache *fc, const char *tfm);

typedef struct {
  bool ok;
  unsigned char *data;     /* cleartext + binary + trailer segments */
  size_t len1, len2, len3;
  double bbox[4];
  char fontname[128];
  pbuf deflated;
} type1_font;

type1_font *font_type1(font_cache *fc, const char *file, pbuf *warnings);

typedef struct {
  bool ok;
  char *glyph[256];
} enc_vector;

enc_vector *font_enc(font_cache *fc, const char *file);

typedef struct {
  int32_t k;
  int32_t scaled_fix;       /* relative to the VF design size */
  char *name;
} vf_fontdef;

typedef struct {
  const unsigned char *packet;
  uint32_t len;
  int32_t width_fix;
  bool exists;
} vf_char;

typedef struct {
  bool ok;
  int nfonts;
  vf_fontdef *fonts;
  vf_char chars[256];
} vf_font;

/* NULL when no virtual font exists for `name`. */
vf_font *font_vf(font_cache *fc, const char *name);

#endif
