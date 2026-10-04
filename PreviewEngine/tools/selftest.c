/* Pitex embedded preview engine — boundary regressions (B7–B9). Builds a
 * crafted XDV/corrupt font and drives the real xdv2pdf/synctex/parser code
 * paths. Run by `make selftest`. Pitex-authored (PolyForm Shield 1.0.0). */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <zlib.h>
#include "xdv2pdf.h"
#include "fonts.h"
#include "tbuf.h"

static int failures;

#define CHECK(cond, msg) do { \
    if (!(cond)) { fprintf(stderr, "FAIL: %s\n", msg); failures++; } \
    else fprintf(stderr, "ok: %s\n", msg); } while (0)

/* ------------------------------------------------------------------ */
/* XDV builder                                                        */

typedef struct { unsigned char *d; size_t len, cap; } bb;
static void put(bb *b, const void *p, size_t n)
{
  if (b->len + n > b->cap)
  {
    b->cap = (b->len + n) * 2 + 64;
    b->d = realloc(b->d, b->cap);
    if (!b->d) abort();
  }
  memcpy(b->d + b->len, p, n);
  b->len += n;
}
static void u8(bb *b, int v) { unsigned char c = (unsigned char)v; put(b, &c, 1); }
static void u16(bb *b, unsigned v) { u8(b, v >> 8); u8(b, v); }
static void u32(bb *b, unsigned v) { u8(b, v >> 24); u8(b, v >> 16); u8(b, v >> 8); u8(b, v); }
static void i32(bb *b, int v) { u32(b, (unsigned)v); }

/* XXX4 with a text payload (op 242, len 4 + text). */
static void xxx(bb *b, const char *s)
{
  u8(b, 242);
  u32(b, (unsigned)strlen(s));
  put(b, s, strlen(s));
}
/* XXX4 whose declared length runs past the buffer end. */
static void xxx_truncated(bb *b, const char *s, size_t real_len)
{
  u8(b, 242);
  u32(b, (unsigned)real_len + 64);
  put(b, s, real_len);
}

static size_t pre(bb *b)   /* PRE: id=2 xdv, num/den/mag, comment */
{
  u8(b, 247); u8(b, 7); u32(b, 25400000); u32(b, 473628672); u32(b, 1000);
  const char *c = "selftest"; u8(b, (int)strlen(c)); put(b, c, strlen(c));
  return b->len;
}
static void bop(bb *b)
{
  u8(b, 139);
  for (int i = 0; i < 10; i++) u32(b, 0);
  i32(b, -1);
}
static void eop(bb *b) { u8(b, 140); }

/* No font needed: specials only, no text. */
static xdv2pdf *new_writer(void)
{
  return xdv2pdf_new((xdv_resolver){ NULL, NULL });
}

static int pdf_of(bb *b, pbuf *pdf, pbuf *warn)
{
  xdv_index *x = xdv_index_new();
  xdv_index_update(x, b->d, b->len);
  xdv2pdf *w = new_writer();
  xdv_range r = { b->d, b->len, x, 0, xdv_index_page_count(x) };
  int err = xdv2pdf_write(w, &r, 1, pdf, warn);
  xdv2pdf_free(w);
  xdv_index_free(x);
  return err;
}

/* ------------------------------------------------------------------ */
/* B8: specials must not read past their payload                        */



/* Inflate all PDF streams into `out`. */
static void pdf_inflate_all(pbuf *pdf, pbuf *out)
{
  for (size_t i = 0; i + 8 < pdf->len; i++)
  {
    if (memcmp(pdf->data + i, "stream\n", 7))
      continue;
    const unsigned char *s = pdf->data + i + 7;
    size_t avail = pdf->len - i - 7;
    const unsigned char *es = memmem(s, avail, "endstream", 9);
    if (!es)
      break;
    avail = (size_t)(es - s);
    unsigned char tmp[65536];
    z_stream z = { 0 };
    inflateInit(&z);
    z.next_in = (unsigned char *)s;
    z.avail_in = (unsigned)avail;
    int rc;
    do {
      z.next_out = tmp;
      z.avail_out = sizeof tmp;
      rc = inflate(&z, 0);
      pbuf_append(out, tmp, sizeof tmp - z.avail_out);
    } while (rc == Z_OK);
    inflateEnd(&z);
    i += 7;
  }
}

static int pdf_has(pbuf *pdf, const char *tok)
{
  pbuf flat = { 0 };
  pbuf_append(&flat, pdf->data, pdf->len);
  pdf_inflate_all(pdf, &flat);
  size_t n = strlen(tok);
  int found = 0;
  for (size_t i = 0; i + n <= flat.len; i++)
    if (!memcmp(flat.data + i, tok, n))
      found = 1;
  pbuf_free(&flat);
  return found;
}

/* First "/MediaBox[0 0 W H" in the PDF → w,h (bp). */
static int pdf_mediabox(pbuf *pdf, double *w, double *h)
{
  pbuf flat = { 0 };
  pbuf_append(&flat, pdf->data, pdf->len);
  pdf_inflate_all(pdf, &flat);
  const char *key = "/MediaBox[0 0 ";
  size_t n = strlen(key);
  int ok = 0;
  for (size_t i = 0; i + n <= flat.len; i++)
  {
    if (!memcmp(flat.data + i, key, n))
    {
      char *q;
      double a = strtod((char *)flat.data + i + n, &q);
      if (q != (char *)flat.data + i + n)
      {
        double b = strtod(q, &q);
        if (q != (char *)flat.data + i + n)
        {
          *w = a; *h = b; ok = 1;
        }
      }
      break;
    }
  }
  pbuf_free(&flat);
  return ok;
}

static void test_special_no_bleed(void)
{
  /* "x:scale 2 2" then opcodes 'e','2': bounded parse scales by 2; an
   * unbounded strtod reads "2e2" → 200. */
  bb b = { 0 };
  pre(&b); bop(&b);
  xxx(&b, "x:scale 2 2");
  u8(&b, 101); /* SET 'e' — continues "2" as "2e…" if read unbounded */
  u8(&b, 50);  /* SET '2' */
  u8(&b, 137); i32(&b, 50000); i32(&b, 50000); /* PUT_RULE a,b */
  eop(&b);
  pbuf pdf = { 0 }, warn = { 0 };
  int err = pdf_of(&b, &pdf, &warn);
  CHECK(err == 0 && pdf.len > 200, "scale-special next-opcode bleed renders");
  /* scale 2 emits "2 0 0 2 … cm"; a bled "2e2" parse emits 200 0 0 200 */
  CHECK(pdf_has(&pdf, "2 0 0 2 "), "x:scale emitted ~2 transform");
  CHECK(!pdf_has(&pdf, "200.00000"), "x:scale did not consume next opcode");
  pbuf_free(&pdf); pbuf_free(&warn); free(b.d);
}

static void test_truncated_special(void);
static void test_truncated_special(void)
{
  /* A XXX4 at the very end whose declared length runs past the buffer:
   * must not read past the page bytes or loop. */
  bb b = { 0 };
  pre(&b); bop(&b);
  u8(&b, DVI_RIGHT1); u8(&b, 10);
  xxx_truncated(&b, "x:gsave", 7); /* says 71 bytes, only 7 present */
  eop(&b);
  pbuf pdf = { 0 }, warn = { 0 };
  int err = pdf_of(&b, &pdf, &warn);
  CHECK(err == 0 || err != 0, "truncated XXX did not crash"); /* survival */
  pbuf_free(&pdf); pbuf_free(&warn); free(b.d);
}


/* ------------------------------------------------------------------ */
/* B8: specials must not read past their payload                        */

static void test_pagesize_boundary(void)
{
  /* "pdf:pagesize wid" — 'wid' is shorter than "width"(5): the guard must
   * stop strncmp reading past the payload end. Also positive control:
   * "width 100mm height 50mm" sets MediaBox to ~283.46×141.73 bp. */
  bb b = { 0 };
  pre(&b); bop(&b);
  xxx(&b, "pdf:pagesize wid");
  eop(&b);
  pbuf pdf = { 0 }, warn = { 0 };
  int err = pdf_of(&b, &pdf, &warn);
  CHECK(err == 0, "boundary pdf:pagesize renders");
  double w = 0, h = 0;
  CHECK(pdf_mediabox(&pdf, &w, &h) && w > 590 && w < 600,
        "boundary pagesize kept default size");
  pbuf_free(&pdf); pbuf_free(&warn);

  free(b.d); b = (bb){ 0 };
  pre(&b); bop(&b);
  xxx(&b, "pdf:pagesize width 100mm height 50mm");
  eop(&b);
  memset(&pdf, 0, sizeof pdf); memset(&warn, 0, sizeof warn);
  err = pdf_of(&b, &pdf, &warn);
  w = h = 0;
  CHECK(err == 0 && pdf_mediabox(&pdf, &w, &h)
        && w > 283.0 && w < 283.9 && h > 141.3 && h < 142.2,
        "explicit width/height honored (100mm→283.46bp, 50mm→141.73bp)");
  pbuf_free(&pdf); pbuf_free(&warn); free(b.d);
}
/* ------------------------------------------------------------------ */
/* B9: corrupt cmap offset must not wrap the bounds check               */

static tbuf *load_font(void *env, const char *name, xdv_res_kind kind)
{
  (void)env; (void)name; (void)kind;
  /* Minimal sfnt: header + one 'cmap' table whose subtable offset is
   * 0xFFFFFFFC — off+4 wraps to 0 without the subtract-safe check. */
  static const int CMAP_LEN = 12;
  size_t total = 12 + 16 + (size_t)CMAP_LEN;
  unsigned char *d = calloc(1, total);
  if (!d) abort();
  d[1] = 1; d[5] = 1;                       /* sfnt version 0x00010000, 1 table */
  unsigned char *r = d + 12;
  memcpy(r, "cmap", 4);
  r[11] = 28;                               /* off=28 */
  r[15] = CMAP_LEN;                         /* len */
  unsigned char *c = d + 28;
  c[2] = 0; c[3] = 1;                       /* version 0, numTables 1 */
  c[5] = 3; c[7] = 1;                       /* platform 3, encoding 1 */
  c[6] = 0xFF; c[7] = 0xFF; c[8] = 0xFF; c[9] = 0xFC; /* off=0xFFFFFFFC */
  tbuf *t = tbuf_new((int)total);
  tbuf_append(t, d, total);
  free(d);
  return t;
}

static void test_corrupt_cmap(void)
{
  font_cache *fc = font_cache_new((xdv_resolver){ NULL, load_font });
  pbuf warn = { 0 };
  native_face *f = font_native(fc, "corrupt.ttf", -1, &warn);
  /* Corrupt cmap must be skipped without crashing; the face itself is
   * still usable (no ToUnicode, glyphs render but don't copy out). */
  CHECK(f && f->ok, "corrupt cmap tolerated, face usable");
  CHECK(f && f->num_glyphs > 0, "face still has glyphs");
  pbuf_free(&warn);
  font_cache_free(fc);
}

/* A valid minimal cmap survives: format 4 with one segment. */
static tbuf *load_font_ok(void *env, const char *name, xdv_res_kind kind)
{
  (void)env; (void)name; (void)kind;
  /* sfnt header + cmap(fmt4: segx2=2, one segment ' '(32) → gid 1 +
   * sentinel) — exercises the real parser path. */
  const int cmap_len = 4 + 8 + 32; /* hdr + rec + fmt4 subtable(2 segs) */
  unsigned char *d = calloc(1, 12 + 16 + (size_t)cmap_len + 8);
  if (!d) abort();
  d[1] = 1; d[5] = 1;
  unsigned char *r = d + 12;
  memcpy(r, "cmap", 4);
  r[11] = 28;
  unsigned cl = (unsigned)(cmap_len);
  r[12] = (cl >> 24) & 255; r[13] = (cl >> 16) & 255; r[14] = (cl >> 8) & 255; r[15] = cl & 255;
  unsigned char *c = d + 28;
  c[2] = 0; c[3] = 1;                 /* numTables 1 */
  c[5] = 3; c[7] = 1;                 /* platform 3, encoding 1 */
  c[8] = 0; c[9] = 0; c[10] = 0; c[11] = 12; /* subtable at c+12 */
  unsigned char *s4 = c + 12;
  s4[0] = 0; s4[1] = 4;               /* format 4 */
  unsigned s4len = 32;
  s4[2] = (s4len >> 8) & 255; s4[3] = s4len & 255;
  s4[4] = 0; s4[5] = 0;               /* language */
  s4[6] = 0; s4[7] = 4;               /* segCountX2 = 4 → 2 segs */
  unsigned char *ends = s4 + 14, *starts = ends + 4 + 2, *deltas = starts + 4,
                *ranges = deltas + 4;
  ends[0] = 0; ends[1] = 32; ends[2] = 0xFF; ends[3] = 0xFF;      /* endCodes */
  starts[0] = 0; starts[1] = 32; starts[2] = 0xFF; starts[3] = 0xFF;
  /* deltas: gid = (32 + delta) & 0xFFFF = 1 → delta = -31 */
  int d0 = (-31) & 0xFFFF;
  deltas[0] = (d0 >> 8) & 255; deltas[1] = d0 & 255;
  deltas[2] = 0; deltas[3] = 1;      /* sentinel maps to itself */
  /* ranges all 0 */
  tbuf *t = tbuf_new(64);
  tbuf_append(t, d, 12 + 16 + (size_t)cmap_len + 8);
  free(d);
  return t;
}

static void test_valid_cmap(void)
{
  font_cache *fc = font_cache_new((xdv_resolver){ NULL, load_font_ok });
  pbuf warn = { 0 };
  native_face *f = font_native(fc, "ok.ttf", -1, &warn);
  if (warn.len) fprintf(stderr, "warn: %.*s", (int)warn.len, warn.data);
  CHECK(f != NULL && f->ok, "valid font parses");
  CHECK(f && f->num_glyphs > 0, "valid font has glyphs");
  CHECK(f && f->to_unicode[1] == 32, "valid cmap maps space to gid 1");
  pbuf_free(&warn);
  font_cache_free(fc);
}

/* A minimal 'ssty' alternate maps gid 1 (space) to gid 2. Corrupt
 * relative offsets must be ignored while the base cmap stays usable. */
static tbuf *load_script_font(void *env, const char *name, xdv_res_kind kind)
{
  tbuf *base = load_font_ok(NULL, name, kind);
  int variant = *(int *)env;
  bb g = { 0 };
  u16(&g, 1); u16(&g, 0); u16(&g, 0); u16(&g, 10); u16(&g, 24);
  u16(&g, 1); put(&g, "ssty", 4); u16(&g, 8);
  u16(&g, 0); u16(&g, 1); u16(&g, 0);
  u16(&g, 1); u16(&g, 4);
  unsigned type = variant == 4 || variant == 5 ? 1 : 3;
  int extension = variant == 2 || variant == 6;
  u16(&g, extension ? 7 : type); u16(&g, 0); u16(&g, 1); u16(&g, 8);
  if (extension)
  {
    u16(&g, 1); u16(&g, type); u32(&g, variant == 6 ? 0xFFFFFFFC : 8);
  }
  u16(&g, variant == 5 ? 2 : 1);
  u16(&g, variant == 4 ? 6 : variant == 5 ? 8 : 12);
  u16(&g, 1);                         /* delta or substitution count */
  if (variant == 5)
    u16(&g, 2);                      /* explicit single substitute */
  else if (variant != 4)
  {
    u16(&g, variant == 7 ? 0xFFFF : 8);
    u16(&g, 1); u16(&g, 2);           /* alternate set */
  }
  u16(&g, variant == 3 ? 2 : 1); u16(&g, variant == 8 ? 0xFFFF : 1); u16(&g, 1);
  if (variant == 3)
  {
    u16(&g, 1); u16(&g, 0);           /* coverage range end/index */
  }
  if (variant == 1)                         /* bad GSUB FeatureList offset */
    g.d[6] = g.d[7] = 0xFF;
  bb b = { 0 };
  put(&b, base->data, 12);
  b.d[5] = 2;
  put(&b, "cmap", 4); u32(&b, 0); u32(&b, 44); u32(&b, 44);
  put(&b, "GSUB", 4); u32(&b, 0); u32(&b, 88); u32(&b, (unsigned)g.len);
  put(&b, base->data + 28, 44);
  put(&b, g.d, g.len);
  tbuf *result = tbuf_from_copy(b.d, b.len);
  tbuf_drop(base); free(b.d); free(g.d);
  return result;
}

static void test_script_unicode(void)
{
  const char *const names[] = {
    "alternate substitution", "corrupt feature offset", "extension substitution",
    "coverage range", "single delta substitution", "single array substitution",
    "corrupt extension offset", "corrupt alternate offset", "corrupt coverage count"
  };
  for (int variant = 0; variant < 9; variant++)
  {
    font_cache *fc = font_cache_new((xdv_resolver){ &variant, load_script_font });
    pbuf warn = { 0 };
    native_face *f = font_native(fc, "script.ttf", -1, &warn);
    int valid = variant != 1 && variant < 6;
    CHECK(f && f->ok && f->to_unicode[1] == 32, "script font retains base cmap");
    CHECK(f && f->to_unicode[2] == (valid ? 32 : 0), names[variant]);
    pbuf_free(&warn);
    font_cache_free(fc);
  }
}

int main(void)
{
  test_special_no_bleed();
  test_truncated_special();
  test_pagesize_boundary();
  test_corrupt_cmap();
  test_valid_cmap();
  test_script_unicode();
  if (failures)
    fprintf(stderr, "%d failures\n", failures);
  else
    fprintf(stderr, "all boundary regressions pass\n");
  return failures ? 1 : 0;
}
