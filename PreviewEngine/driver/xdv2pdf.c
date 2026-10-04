/* Pitex embedded preview engine — XDV to PDF conversion.
 *
 * Interprets DVI/XDV pages (Knuth's DVI format, XeTeX native-font opcodes
 * 252-254, virtual fonts) and the dvipdfmx-style specials emitted by the
 * LaTeX xetex drivers (graphics-def xetex.def, pgfsys-dvipdfmx/xetex.def),
 * producing a self-contained PDF. Independently written from the published
 * format descriptions and driver sources' documented special syntax; no
 * xdvipdfmx/dvipdfmx or MuPDF code. Pitex-authored (PolyForm Shield 1.0.0).
 *
 * ponytail: fonts are embedded whole (no subsetting) and compressed once
 * per session; subset them if very large CJK fonts make publishing slow. */
#include "xdv2pdf.h"

#include <ctype.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "fonts.h"
#include "images.h"
#include "pdfread.h"
#include "pdfw.h"

#define MAX_VF_DEPTH 8
#define MAX_STACK 1024
#define MAX_COLORSTACKS 16

/* ------------------------------------------------------------------ */
/* Small helpers                                                       */

typedef struct { double a, b, c, d, e, f; } mat;

static uint32_t be(const unsigned char *p, int n)
{
  uint32_t v = 0;
  for (int i = 0; i < n; i++)
    v = (v << 8) | p[i];
  return v;
}

static int32_t sbe(const unsigned char *p, int n)
{
  uint32_t v = be(p, n);
  if (n < 4 && (v & (1u << (8 * n - 1))))
    v |= ~0u << (8 * n);
  return (int32_t)v;
}

static char *xstrndup(const char *s, size_t n)
{
  char *r = malloc(n + 1);
  if (!r)
    abort();
  memcpy(r, s, n);
  r[n] = 0;
  return r;
}

/* ------------------------------------------------------------------ */
/* Fonts as used by a DVI stream                                       */

typedef enum { FS_MISSING, FS_NATIVE, FS_TYPE1, FS_VF } fs_kind;

typedef struct font_table font_table;

typedef struct {
  int32_t k;
  fs_kind kind;
  double size; /* DVI units of the stream that defined it */
  char *name;
  tfm_font *tfm;
  const map_entry *map;
  type1_font *t1;
  enc_vector *enc;
  vf_font *vf;
  font_table *vf_local;
  int vf_default; /* index in vf_local of the first local font */
  native_face *nf;
  bool colored;
  double rgba[4];
  double extend, slant, embolden;
  int pdf_font; /* per-publish index, -1 until used */
  int publish_epoch;
} font_slot;

struct font_table {
  int n, cap;
  font_slot **slots;
};

static font_slot *table_find(font_table *t, int32_t k)
{
  for (int i = t->n - 1; i >= 0; i--)
    if (t->slots[i]->k == k)
      return t->slots[i];
  return NULL;
}

static void table_add(font_table *t, font_slot *s)
{
  if (t->n == t->cap)
  {
    t->cap = t->cap ? t->cap * 2 : 16;
    t->slots = realloc(t->slots, sizeof(font_slot *) * t->cap);
    if (!t->slots)
      abort();
  }
  t->slots[t->n++] = s;
}

static void slot_free(font_slot *s);

static void table_free(font_table *t)
{
  if (!t)
    return;
  for (int i = 0; i < t->n; i++)
    slot_free(t->slots[i]);
  free(t->slots);
  free(t);
}

static void slot_free(font_slot *s)
{
  free(s->name);
  table_free(s->vf_local);
  free(s);
}

/* ------------------------------------------------------------------ */
/* Per-publish PDF resources                                           */

typedef struct {
  bool native;
  native_face *nf;
  tfm_font *tfm;
  type1_font *t1;
  enc_vector *enc;
  int obj;
  unsigned char *used; /* native: bitmap by gid; type1: by code */
  int nused;
} pdf_font;

typedef struct {
  char *key;
  char *value;
} kv;

typedef struct {
  kv *items;
  int n, cap;
} kvlist;

static void kv_set(kvlist *l, const char *key, size_t klen, const char *val, size_t vlen)
{
  for (int i = 0; i < l->n; i++)
    if (strlen(l->items[i].key) == klen && memcmp(l->items[i].key, key, klen) == 0)
    {
      free(l->items[i].value);
      l->items[i].value = xstrndup(val, vlen);
      return;
    }
  if (l->n == l->cap)
  {
    l->cap = l->cap ? l->cap * 2 : 8;
    l->items = realloc(l->items, sizeof(kv) * l->cap);
    if (!l->items)
      abort();
  }
  l->items[l->n].key = xstrndup(key, klen);
  l->items[l->n].value = xstrndup(val, vlen);
  l->n++;
}

static void kv_free(kvlist *l)
{
  for (int i = 0; i < l->n; i++)
  {
    free(l->items[i].key);
    free(l->items[i].value);
  }
  free(l->items);
  memset(l, 0, sizeof *l);
}

enum { CAT_EXTGSTATE, CAT_COLORSPACE, CAT_PATTERN, CAT_SHADING, CAT_XOBJECT, CAT_FONT, CAT_PROPERTIES, NCAT };
static const char *const cat_names[NCAT] = { "ExtGState", "ColorSpace", "Pattern", "Shading",
                                             "XObject", "Font", "Properties" };

typedef struct {
  kvlist cat[NCAT];
  char *cat_ref[NCAT]; /* "N 0 R" when a special set the whole category */
} resources;

static void resources_free(resources *r)
{
  for (int i = 0; i < NCAT; i++)
  {
    kv_free(&r->cat[i]);
    free(r->cat_ref[i]);
  }
  memset(r, 0, sizeof *r);
}

typedef enum { NO_UNDEF, NO_DICT, NO_ARRAY, NO_RAW, NO_STREAM, NO_FORM } named_kind;

typedef struct {
  char *name;
  int obj;
  named_kind kind;
  kvlist dict;       /* NO_DICT (and stream dict) */
  pbuf array;        /* NO_ARRAY items, NO_RAW text, NO_STREAM data */
  bool written;
} named_obj;

typedef struct {
  char *ops[64];
  int depth;
  char *init;
  bool page;
} colorstack;

typedef struct cached_image {
  struct cached_image *next;
  tbuf *file;
  pdf_image img;
  bool pdf;          /* PDF figure */
  pr_doc *doc;
} cached_image;

struct xdv2pdf {
  xdv_resolver res;
  font_cache *fonts;
  cached_image *images;
  int epoch;
};

/* One output XObject per (image, page) per publish. */
typedef struct {
  cached_image *ci;
  int page;
  int box;
  int obj;
  char name[16];
  double w, h;            /* raster natural size (bp) */
  double bbox[4];         /* PDF: rotated box */
} xobj_use;

/* Capture target (page content or a pdf:bxobj form). */
typedef struct {
  pbuf content;
  resources res;
  named_obj *form;       /* NULL for page content */
  double bbox[4];
  double ox, oy;          /* origin in page coordinates */
  int q_base;
  bool saved_render;
} target;

typedef struct {
  xdv2pdf *w;
  pdfw *pw;
  pbuf *warnings;
  /* output-wide */
  pdf_font *pfonts;
  int npfonts, cappfonts;
  xobj_use *xobjs;
  int nxobjs, capxobjs;
  named_obj *named;
  int nnamed, capnamed;
  resources page_res;
  int font_dict_obj, xobject_dict_obj, resources_obj;
  /* stream state */
  double conv;            /* bp per DVI unit */
  double page_w, page_h, default_w, default_h;
  double ox, oy;          /* bcontent offset */
  double off_stack[64][2];
  int off_depth;
  target targets[8];
  int ntargets;
  int q_depth;
  bool render;            /* false while prescanning earlier pages */
  colorstack cs[MAX_COLORSTACKS];
  char *old_colors[64];   /* legacy "color push" stack */
  int old_depth;
  double background[3];
  bool has_background;
  /* text state */
  bool in_bt, in_tj, pos_valid;
  int t_font;
  double t_size, t_a, t_c, t_x, t_y;
  /* warnings dedupe */
  char warned[64][48];
  int nwarned;
} conv_ctx;

static target *cur(conv_ctx *c) { return &c->targets[c->ntargets - 1]; }

static void warn_once(conv_ctx *c, const char *key, const char *fmt, const char *arg)
{
  for (int i = 0; i < c->nwarned; i++)
    if (strncmp(c->warned[i], key, sizeof c->warned[i] - 1) == 0)
      return;
  if (c->nwarned < 64)
    snprintf(c->warned[c->nwarned++], sizeof c->warned[0], "%s", key);
  pbuf_printf(c->warnings, fmt, arg);
  pbuf_putc(c->warnings, '\n');
}

/* ------------------------------------------------------------------ */
/* Content emission                                                    */

static void emit_num(pbuf *b, double v)
{
  pbuf_real(b, v);
  pbuf_putc(b, ' ');
}

static void text_end(conv_ctx *c)
{
  pbuf *o = &cur(c)->content;
  if (c->in_tj)
    pbuf_puts(o, "]TJ\n");
  if (c->in_bt)
    pbuf_puts(o, "ET\n");
  c->in_tj = c->in_bt = c->pos_valid = false;
  c->t_font = -1;
}

static void emit_raw(conv_ctx *c, const char *s, size_t n)
{
  if (!c->render)
    return;
  text_end(c);
  pbuf *o = &cur(c)->content;
  pbuf_append(o, s, n);
  pbuf_putc(o, '\n');
}

static void emit_q(conv_ctx *c)
{
  if (!c->render)
    return;
  text_end(c);
  pbuf_puts(&cur(c)->content, "q\n");
  c->q_depth++;
}

static void emit_Q(conv_ctx *c)
{
  if (!c->render)
    return;
  text_end(c);
  if (c->q_depth > cur(c)->q_base)
  {
    pbuf_puts(&cur(c)->content, "Q\n");
    c->q_depth--;
  }
}

static void emit_cm(conv_ctx *c, mat m)
{
  if (!c->render)
    return;
  text_end(c);
  pbuf *o = &cur(c)->content;
  emit_num(o, m.a);
  emit_num(o, m.b);
  emit_num(o, m.c);
  emit_num(o, m.d);
  emit_num(o, m.e);
  emit_num(o, m.f);
  pbuf_puts(o, "cm\n");
}

static mat mat_mul(mat x, mat y) /* x then y */
{
  return (mat){ x.a * y.a + x.b * y.c, x.a * y.b + x.b * y.d, x.c * y.a + x.d * y.c,
                x.c * y.b + x.d * y.d, x.e * y.a + x.f * y.c + y.e, x.e * y.b + x.f * y.d + y.f };
}

static mat mat_translate(double x, double y) { return (mat){ 1, 0, 0, 1, x, y }; }

/* Transform about point (x, y). */
static mat mat_about(mat m, double x, double y)
{
  return mat_mul(mat_mul(mat_translate(-x, -y), m), mat_translate(x, y));
}

/* Page coordinates of a DVI position (relative to active offsets). */
static double X(conv_ctx *c, double h) { return 72.0 + h * c->conv - c->ox; }
static double Y(conv_ctx *c, double v) { return c->page_h - 72.0 - v * c->conv - c->oy; }

static void emit_rule(conv_ctx *c, double h, double v, double width, double height)
{
  if (!c->render || width <= 0 || height <= 0)
    return;
  text_end(c);
  pbuf *o = &cur(c)->content;
  emit_num(o, X(c, h));
  emit_num(o, Y(c, v));
  emit_num(o, width * c->conv);
  emit_num(o, height * c->conv);
  pbuf_puts(o, "re f\n");
}

static void emit_glyph(conv_ctx *c, int pf, double size, double a, double sl, double x, double y,
                       const unsigned char *code, int nbytes, double width_units)
{
  pbuf *o = &cur(c)->content;
  if (!c->in_bt)
  {
    pbuf_puts(o, "BT\n");
    c->in_bt = true;
    c->t_font = -1;
    c->pos_valid = false;
  }
  if (c->t_font != pf || fabs(c->t_size - size) > 1e-6)
  {
    if (c->in_tj)
    {
      pbuf_puts(o, "]TJ\n");
      c->in_tj = false;
    }
    pbuf_printf(o, "/F%d ", pf + 1);
    emit_num(o, size);
    pbuf_puts(o, "Tf\n");
    c->t_font = pf;
    c->t_size = size;
    c->pos_valid = false;
  }
  if (fabs(c->t_a - a) > 1e-9 || fabs(c->t_c - sl) > 1e-9)
    c->pos_valid = false;
  if (c->pos_valid && c->in_tj && fabs(y - c->t_y) < 1e-4)
  {
    double adj = -(x - c->t_x) * 1000.0 / (size * a);
    if (fabs(adj) > 0.01)
    {
      pbuf_real(o, adj);
    }
  }
  else
  {
    if (c->in_tj)
      pbuf_puts(o, "]TJ\n");
    emit_num(o, a);
    pbuf_puts(o, "0 ");
    emit_num(o, sl);
    pbuf_puts(o, "1 ");
    emit_num(o, x);
    emit_num(o, y);
    pbuf_puts(o, "Tm[");
    c->in_tj = true;
    c->t_a = a;
    c->t_c = sl;
  }
  static const char hex[] = "0123456789ABCDEF";
  pbuf_putc(o, '<');
  for (int i = 0; i < nbytes; i++)
  {
    pbuf_putc(o, hex[code[i] >> 4]);
    pbuf_putc(o, hex[code[i] & 15]);
  }
  pbuf_putc(o, '>');
  c->t_x = x + width_units / 1000.0 * size * a;
  c->t_y = y;
  c->pos_valid = true;
}

/* ------------------------------------------------------------------ */
/* PDF font registry                                                   */

static int pdf_font_for(conv_ctx *c, font_slot *s)
{
  if (s->publish_epoch == c->w->epoch && s->pdf_font >= 0)
    return s->pdf_font;
  for (int i = 0; i < c->npfonts; i++)
  {
    pdf_font *p = &c->pfonts[i];
    if (s->kind == FS_NATIVE ? (p->native && p->nf == s->nf)
                             : (!p->native && p->t1 == s->t1 && p->enc == s->enc && p->tfm == s->tfm))
    {
      s->pdf_font = i;
      s->publish_epoch = c->w->epoch;
      return i;
    }
  }
  if (c->npfonts == c->cappfonts)
  {
    c->cappfonts = c->cappfonts ? c->cappfonts * 2 : 16;
    c->pfonts = realloc(c->pfonts, sizeof(pdf_font) * c->cappfonts);
    if (!c->pfonts)
      abort();
  }
  pdf_font *p = &c->pfonts[c->npfonts];
  memset(p, 0, sizeof *p);
  p->native = s->kind == FS_NATIVE;
  p->nf = s->nf;
  p->t1 = s->t1;
  p->enc = s->enc;
  p->tfm = s->tfm;
  p->obj = pdfw_alloc(c->pw);
  p->nused = p->native ? s->nf->num_glyphs : 256;
  p->used = calloc((size_t)p->nused + 1, 1);
  if (!p->used)
    abort();
  s->pdf_font = c->npfonts;
  s->publish_epoch = c->w->epoch;
  return c->npfonts++;
}

/* ------------------------------------------------------------------ */
/* Font definitions                                                    */

static font_slot *new_slot(int32_t k)
{
  font_slot *s = calloc(1, sizeof *s);
  if (!s)
    abort();
  s->k = k;
  s->extend = 1;
  s->pdf_font = -1;
  return s;
}

static void setup_tfm_slot(conv_ctx *c, font_slot *s)
{
  static int vf_depth; /* virtual fonts may (wrongly) refer to themselves */
  vf_font *vf = vf_depth < MAX_VF_DEPTH ? font_vf(c->w->fonts, s->name) : NULL;
  s->tfm = font_tfm(c->w->fonts, s->name);
  if (vf && vf->ok)
  {
    s->kind = FS_VF;
    s->vf = vf;
    s->vf_local = calloc(1, sizeof(font_table));
    if (!s->vf_local)
      abort();
    vf_depth++;
    for (int i = 0; i < vf->nfonts; i++)
    {
      font_slot *l = new_slot(vf->fonts[i].k);
      l->name = strdup(vf->fonts[i].name);
      l->size = (double)vf->fonts[i].scaled_fix * s->size / 1048576.0;
      setup_tfm_slot(c, l);
      table_add(s->vf_local, l);
    }
    vf_depth--;
    return;
  }
  const map_entry *m = font_map_lookup(c->w->fonts, s->name);
  if (!m || !m->fontfile)
  {
    s->kind = FS_MISSING;
    warn_once(c, s->name, "no Type 1 font mapped for TeX font %s (glyphs omitted)", s->name);
    return;
  }
  s->map = m;
  s->t1 = font_type1(c->w->fonts, m->fontfile, c->warnings);
  if (!s->t1->ok || !s->tfm->ok)
  {
    s->kind = FS_MISSING;
    if (!s->tfm->ok)
      warn_once(c, s->name, "TFM not found: %s", s->name);
    return;
  }
  s->enc = m->encfile ? font_enc(c->w->fonts, m->encfile) : NULL;
  if (s->enc && !s->enc->ok)
    s->enc = NULL;
  s->extend = m->extend > 0 ? m->extend : 1;
  s->slant = m->slant;
  s->kind = FS_TYPE1;
}

static void define_tfm_font(conv_ctx *c, font_table *t, const unsigned char *p, int k_bytes)
{
  int32_t k = sbe(p + 1, k_bytes);
  const unsigned char *q = p + 1 + k_bytes;
  int32_t s = (int32_t)be(q + 4, 4);
  int a = q[12], l = q[13];
  if (table_find(t, k))
    return;
  font_slot *slot = new_slot(k);
  slot->name = xstrndup((const char *)q + 14 + a, (size_t)l);
  slot->size = s;
  setup_tfm_slot(c, slot);
  table_add(t, slot);
}

static void define_native_font(conv_ctx *c, font_table *t, const unsigned char *p)
{
  int32_t k = (int32_t)be(p + 1, 4);
  if (table_find(t, k))
    return;
  int32_t size = (int32_t)be(p + 5, 4);
  unsigned flags = be(p + 9, 2);
  int nl = p[11];
  const unsigned char *q = p + 12 + nl;
  uint32_t index = be(q, 4);
  q += 4;
  font_slot *s = new_slot(k);
  s->name = xstrndup((const char *)p + 12, (size_t)nl);
  s->size = size;
  if (flags & 0x0200)
  {
    uint32_t rgba = be(q, 4);
    q += 4;
    s->colored = true;
    s->rgba[0] = (rgba >> 24) / 255.0;
    s->rgba[1] = ((rgba >> 16) & 255) / 255.0;
    s->rgba[2] = ((rgba >> 8) & 255) / 255.0;
    s->rgba[3] = (rgba & 255) / 255.0;
  }
  if (flags & 0x1000)
  {
    s->extend = (int32_t)be(q, 4) / 65536.0;
    q += 4;
  }
  if (flags & 0x2000)
  {
    s->slant = (int32_t)be(q, 4) / 65536.0;
    q += 4;
  }
  if (flags & 0x4000)
  {
    s->embolden = (int32_t)be(q, 4) / 65536.0;
    q += 4;
  }
  if (flags & 0x0100)
    warn_once(c, "vertical", "vertical native fonts are previewed horizontally%s", "");
  s->nf = font_native(c->w->fonts, s->name, (int)index, c->warnings);
  s->kind = s->nf->ok ? FS_NATIVE : FS_MISSING;
  table_add(t, s);
}

/* ------------------------------------------------------------------ */
/* Glyph drawing                                                       */

static void run_packet(conv_ctx *c, font_slot *vfslot, const unsigned char *p, size_t len, double h, double v,
                       int depth);

static void draw_char(conv_ctx *c, font_slot *s, unsigned code, double h, double v, int depth)
{
  if (!c->render || code > 255)
    return;
  switch (s->kind)
  {
    case FS_TYPE1:
    {
      if (!s->tfm->exists[code])
        return;
      int pf = pdf_font_for(c, s);
      c->pfonts[pf].used[code] = 1;
      double size = s->size * c->conv;
      double wunits = s->tfm->width[code] / 1048.576; /* fix_word * 1000 / 2^20 */
      unsigned char b = (unsigned char)code;
      emit_glyph(c, pf, size, s->extend, s->slant, X(c, h), Y(c, v), &b, 1, wunits);
      break;
    }
    case FS_VF:
    {
      if (depth >= MAX_VF_DEPTH || !s->vf->chars[code].exists)
        return;
      run_packet(c, s, s->vf->chars[code].packet, s->vf->chars[code].len, h, v, depth + 1);
      break;
    }
    default:
      break;
  }
}

static double char_width(font_slot *s, unsigned code)
{
  if (code > 255)
    return 0;
  if (s->tfm && s->tfm->ok && s->tfm->exists[code])
    return (double)s->tfm->width[code] * s->size / 1048576.0;
  if (s->kind == FS_VF && s->vf->chars[code].exists)
    return (double)s->vf->chars[code].width_fix * s->size / 1048576.0;
  return 0;
}

static void begin_native_style(conv_ctx *c, font_slot *s)
{
  if (!s->colored && s->embolden == 0)
    return;
  text_end(c);
  pbuf *o = &cur(c)->content;
  pbuf_puts(o, "q ");
  if (s->colored)
  {
    emit_num(o, s->rgba[0]);
    emit_num(o, s->rgba[1]);
    emit_num(o, s->rgba[2]);
    pbuf_puts(o, "rg ");
    emit_num(o, s->rgba[0]);
    emit_num(o, s->rgba[1]);
    emit_num(o, s->rgba[2]);
    pbuf_puts(o, "RG ");
  }
  if (s->embolden != 0)
  {
    emit_num(o, fabs(s->embolden));
    pbuf_puts(o, "w 2 Tr");
  }
  pbuf_putc(o, '\n');
}

static void end_native_style(conv_ctx *c, font_slot *s)
{
  if (!s->colored && s->embolden == 0)
    return;
  text_end(c);
  pbuf_puts(&cur(c)->content, "0 Tr Q\n");
}

static void draw_native_glyphs(conv_ctx *c, font_slot *s, double h, double v, const unsigned char *xy,
                               const unsigned char *ids, int n)
{
  if (!c->render || s->kind != FS_NATIVE)
    return;
  int pf = pdf_font_for(c, s);
  pdf_font *p = &c->pfonts[pf];
  double size = s->size * c->conv;
  begin_native_style(c, s);
  for (int i = 0; i < n; i++)
  {
    int32_t gx = (int32_t)be(xy + 8 * i, 4), gy = (int32_t)be(xy + 8 * i + 4, 4);
    unsigned gid = be(ids + 2 * i, 2);
    if ((int)gid < p->nused)
      p->used[gid] = 1;
    double wunits = (double)native_advance(s->nf, (int)gid) * 1000.0 / s->nf->units_per_em;
    unsigned char code[2] = { (unsigned char)(gid >> 8), (unsigned char)gid };
    emit_glyph(c, pf, size, s->extend, s->slant, X(c, h + gx), Y(c, v + gy), code, 2, wunits);
  }
  end_native_style(c, s);
}

/* ------------------------------------------------------------------ */
/* Specials                                                            */

static named_obj *named_get(conv_ctx *c, const char *name, size_t len, bool create)
{
  for (int i = 0; i < c->nnamed; i++)
    if (strlen(c->named[i].name) == len && memcmp(c->named[i].name, name, len) == 0)
      return &c->named[i];
  if (!create)
    return NULL;
  if (c->nnamed == c->capnamed)
  {
    c->capnamed = c->capnamed ? c->capnamed * 2 : 16;
    c->named = realloc(c->named, sizeof(named_obj) * c->capnamed);
    if (!c->named)
      abort();
  }
  named_obj *o = &c->named[c->nnamed++];
  memset(o, 0, sizeof *o);
  o->name = xstrndup(name, len);
  o->obj = pdfw_alloc(c->pw);
  return o;
}

static const char *skip_ws(const char *p, const char *end)
{
  while (p < end && isspace((unsigned char)*p))
    p++;
  return p;
}

/* Skip one PDF value; returns pointer after it. */
static const char *skip_value(const char *p, const char *end)
{
  p = skip_ws(p, end);
  if (p >= end)
    return p;
  if (*p == '(')
  {
    int depth = 0;
    for (; p < end; p++)
    {
      if (*p == '\\')
      {
        p++;
        continue;
      }
      if (*p == '(')
        depth++;
      else if (*p == ')' && --depth == 0)
        return p + 1;
    }
    return end;
  }
  if (*p == '<' && p + 1 < end && p[1] == '<')
  {
    p += 2;
    for (;;)
    {
      p = skip_ws(p, end);
      if (p >= end)
        return end;
      if (*p == '>' && p + 1 < end && p[1] == '>')
        return p + 2;
      const char *q = skip_value(p, end);
      if (q == p)
        return end;
      p = q;
    }
  }
  if (*p == '<')
  {
    const char *q = memchr(p, '>', (size_t)(end - p));
    return q ? q + 1 : end;
  }
  if (*p == '[')
  {
    p++;
    for (;;)
    {
      p = skip_ws(p, end);
      if (p >= end)
        return end;
      if (*p == ']')
        return p + 1;
      const char *q = skip_value(p, end);
      if (q == p)
        return end;
      p = q;
    }
  }
  if (*p == '/')
    p++;
  const char *s = p;
  while (p < end && !isspace((unsigned char)*p) && !strchr("()<>[]{}/%", *p))
    p++;
  if (p == s && p < end && *p != '/')
    p++; /* stray delimiter */
  /* "num gen R" reference */
  const char *t = skip_ws(p, end);
  if (isdigit((unsigned char)*s))
  {
    const char *u = t;
    while (u < end && isdigit((unsigned char)*u))
      u++;
    if (u > t)
    {
      const char *r = skip_ws(u, end);
      if (r < end && *r == 'R' && (r + 1 == end || !isalnum((unsigned char)r[1])))
        return r + 1;
    }
  }
  return p;
}

/* Replace @name tokens (outside strings) by indirect references. */
static char *subst_names(conv_ctx *c, const char *s, size_t n, double cpx, double cpy)
{
  pbuf b = { 0 };
  const char *end = s + n;
  for (const char *p = s; p < end;)
  {
    if (*p == '(')
    {
      const char *q = skip_value(p, end);
      pbuf_append(&b, p, (size_t)(q - p));
      p = q;
      continue;
    }
    /* "@name" references start a token; "@" inside a PDF name such as
     * /pgf@CA0.5 is part of that name. */
    char prev = p > s ? p[-1] : ' ';
    if (*p == '@' && (isspace((unsigned char)prev) || strchr("[]<>{}", prev)))
    {
      const char *q = p + 1;
      while (q < end && (isalnum((unsigned char)*q) || strchr("_.-@:", *q)))
        q++;
      size_t len = (size_t)(q - p);
      if (len == 5 && !memcmp(p, "@xpos", 5))
        pbuf_real(&b, cpx);
      else if (len == 5 && !memcmp(p, "@ypos", 5))
        pbuf_real(&b, cpy);
      else if (len > 1)
        pbuf_printf(&b, "%d 0 R", named_get(c, p, len, true)->obj);
      p = q;
      continue;
    }
    pbuf_putc(&b, *p++);
  }
  pbuf_putc(&b, 0);
  return (char *)b.data;
}

/* Merge "<< /K v ... >>" entries into kv list. */
static void merge_dict(kvlist *l, const char *s)
{
  const char *end = s + strlen(s);
  const char *p = skip_ws(s, end);
  if (p + 1 >= end || p[0] != '<' || p[1] != '<')
    return;
  p += 2;
  for (;;)
  {
    p = skip_ws(p, end);
    if (p >= end || (*p == '>' && p + 1 < end && p[1] == '>'))
      break;
    if (*p != '/')
    {
      const char *q = skip_value(p, end);
      if (q == p)
        break;
      p = q;
      continue;
    }
    const char *k = p + 1, *ke = k;
    while (ke < end && !isspace((unsigned char)*ke) && !strchr("()<>[]{}/%", *ke))
      ke++;
    const char *vs = skip_ws(ke, end), *ve = skip_value(vs, end);
    kv_set(l, k, (size_t)(ke - k), vs, (size_t)(ve - vs));
    p = ve;
  }
}

static void merge_resources(resources *r, const char *dict)
{
  kvlist tmp = { 0 };
  merge_dict(&tmp, dict);
  for (int i = 0; i < tmp.n; i++)
  {
    int cat = -1;
    for (int j = 0; j < NCAT; j++)
      if (!strcmp(tmp.items[i].key, cat_names[j]))
        cat = j;
    if (cat < 0)
      continue;
    const char *v = tmp.items[i].value;
    if (v[0] == '<')
      merge_dict(&r->cat[cat], v);
    else
    {
      free(r->cat_ref[cat]);
      r->cat_ref[cat] = strdup(v);
    }
  }
  kv_free(&tmp);
}

static double parse_dimen(const char **pp, const char *end, bool *ok)
{
  const char *p = skip_ws(*pp, end);
  char *q;
  double v = strtod(p, &q);
  *ok = q != p;
  p = skip_ws(q, end);
  static const struct { const char *u; double f; } units[] = {
    { "truept", 72.0 / 72.27 }, { "pt", 72.0 / 72.27 }, { "bp", 1 }, { "in", 72 }, { "cm", 72 / 2.54 },
    { "mm", 72 / 25.4 }, { "pc", 12 * 72.0 / 72.27 }, { "dd", 1238.0 / 1157 * 72.0 / 72.27 },
    { "cc", 12 * 1238.0 / 1157 * 72.0 / 72.27 }, { "sp", 72.0 / 72.27 / 65536 } };
  for (size_t i = 0; i < sizeof units / sizeof units[0]; i++)
  {
    size_t n = strlen(units[i].u);
    if ((size_t)(end - p) >= n && !strncmp(p, units[i].u, n))
    {
      *pp = p + n;
      return v * units[i].f;
    }
  }
  *pp = q;
  return v;
}

static bool parse_paper(const char *s, const char *end, double *w, double *h)
{
  bool ok1, ok2;
  const char *p = s;
  *w = parse_dimen(&p, end, &ok1);
  p = skip_ws(p, end);
  if (p < end && *p == ',')
    p++;
  *h = parse_dimen(&p, end, &ok2);
  return ok1 && ok2 && *w > 0 && *h > 0;
}

/* Scan a page's specials for its media size before rendering it. */
static void page_size_prescan(conv_ctx *c, const unsigned char *p, const unsigned char *end)
{
  double w = c->default_w, h = c->default_h;
  while (p < end)
  {
    long n = xdv_insn_length(p, end);
    if (n <= 0)
      break;
    if (p[0] >= DVI_XXX1 && p[0] < DVI_XXX1 + 4)
    {
      int k = p[0] - DVI_XXX1 + 1;
      const char *s = (const char *)p + 1 + k, *se = (const char *)p + n;
      if ((size_t)(se - s) > 10 && !strncmp(s, "papersize=", 10))
      {
        double pw, ph;
        if (parse_paper(s + 10, se, &pw, &ph))
        {
          c->default_w = w = pw;
          c->default_h = h = ph;
        }
      }
      else if ((size_t)(se - s) > 13 && !strncmp(s, "pdf:pagesize ", 13))
      {
        const char *q = s + 13;
        q = skip_ws(q, se);
        if ((size_t)(se - q) >= 7 && !strncmp(q, "default", 7))
          w = c->default_w, h = c->default_h;
        else
        {
          double pw = 0, ph = 0;
          while (q < se)
          {
            q = skip_ws(q, se);
            bool ok;
            if ((size_t)(se - q) >= 5 && !strncmp(q, "width", 5))
            {
              q += 5;
              pw = parse_dimen(&q, se, &ok);
            }
            else if ((size_t)(se - q) >= 6 && !strncmp(q, "height", 6))
            {
              q += 6;
              ph = parse_dimen(&q, se, &ok);
            }
            else
            {
              while (q < se && !isspace((unsigned char)*q))
                q++;
            }
          }
          if (pw > 0 && ph > 0)
            w = pw, h = ph;
        }
      }
    }
    p += n;
  }
  c->page_w = w;
  c->page_h = h;
}

static void set_colorstack_ops(conv_ctx *c, const char *ops)
{
  if (ops && *ops)
    emit_raw(c, ops, strlen(ops));
}

static char *paren_content(const char *p, const char *end)
{
  p = skip_ws(p, end);
  if (p >= end || *p != '(')
    return strdup("");
  const char *q = skip_value(p, end);
  if (q - p < 2)
    return strdup("");
  /* literal string content: operators contain no escapes in practice */
  return xstrndup(p + 1, (size_t)(q - p - 2));
}

static char *color_spec_ops(const char *s, const char *end)
{
  /* "rgb r g b", "cmyk c m y k", "gray g", "hsb h s b", or bare numbers */
  s = skip_ws(s, end);
  double v[4] = { 0 };
  int n = 0;
  const char *model = s;
  while (s < end && isalpha((unsigned char)*s))
    s++;
  size_t mlen = (size_t)(s - model);
  while (n < 4)
  {
    s = skip_ws(s, end);
    if (s < end && (*s == '[' || *s == ']'))
    {
      s++;
      continue;
    }
    char *q;
    double x = strtod(s, &q);
    if (q == s)
      break;
    v[n++] = x;
    s = q;
  }
  char buf[160];
  if ((mlen == 3 && !strncmp(model, "rgb", 3)) || (mlen == 0 && n == 3))
    snprintf(buf, sizeof buf, "%g %g %g rg %g %g %g RG", v[0], v[1], v[2], v[0], v[1], v[2]);
  else if ((mlen == 4 && !strncmp(model, "cmyk", 4)) || (mlen == 0 && n == 4))
    snprintf(buf, sizeof buf, "%g %g %g %g k %g %g %g %g K", v[0], v[1], v[2], v[3], v[0], v[1], v[2], v[3]);
  else if ((mlen == 4 && !strncmp(model, "gray", 4)) || (mlen == 0 && n == 1))
    snprintf(buf, sizeof buf, "%g g %g G", v[0], v[0]);
  else if (mlen == 3 && !strncmp(model, "hsb", 3))
  {
    double h = v[0] * 6, sat = v[1], br = v[2], f = h - floor(h), r, g, b;
    double p = br * (1 - sat), q = br * (1 - sat * f), t = br * (1 - sat * (1 - f));
    switch (((int)floor(h)) % 6)
    {
      case 0: r = br; g = t; b = p; break;
      case 1: r = q; g = br; b = p; break;
      case 2: r = p; g = br; b = t; break;
      case 3: r = p; g = q; b = br; break;
      case 4: r = t; g = p; b = br; break;
      default: r = br; g = p; b = q; break;
    }
    snprintf(buf, sizeof buf, "%g %g %g rg %g %g %g RG", r, g, b, r, g, b);
  }
  else if (mlen == 5 && !strncmp(model, "Black", 5))
    snprintf(buf, sizeof buf, "0 g 0 G");
  else
    return NULL;
  return strdup(buf);
}

static void do_pdfcolorstack(conv_ctx *c, const char *s, const char *end, bool init)
{
  char *q;
  long n = strtol(s, &q, 10);
  if (q == s || n < 0 || n >= MAX_COLORSTACKS)
    return;
  colorstack *st = &c->cs[n];
  const char *p = skip_ws(q, end);
  if (init)
  {
    st->page = false;
    for (;;)
    {
      p = skip_ws(p, end);
      if (!strncmp(p, "page", 4))
      {
        st->page = true;
        p += 4;
      }
      else if (!strncmp(p, "direct", 6))
        p += 6;
      else
        break;
    }
    free(st->init);
    st->init = paren_content(p, end);
    for (int i = 0; i < st->depth; i++)
      free(st->ops[i]);
    st->depth = 0;
    return;
  }
  if (!strncmp(p, "push", 4))
  {
    char *ops = paren_content(p + 4, end);
    if (st->depth < 64)
      st->ops[st->depth++] = ops;
    else
      free(ops);
    set_colorstack_ops(c, st->depth ? st->ops[st->depth - 1] : NULL);
  }
  else if (!strncmp(p, "pop", 3))
  {
    if (st->depth > 0)
      free(st->ops[--st->depth]);
    set_colorstack_ops(c, st->depth ? st->ops[st->depth - 1] : st->init);
  }
  else if (!strncmp(p, "set", 3))
  {
    char *ops = paren_content(p + 3, end);
    if (st->depth > 0)
    {
      free(st->ops[st->depth - 1]);
      st->ops[st->depth - 1] = ops;
    }
    else
    {
      free(st->init);
      st->init = ops;
    }
    set_colorstack_ops(c, st->depth ? st->ops[st->depth - 1] : st->init);
  }
  else if (!strncmp(p, "current", 7))
    set_colorstack_ops(c, st->depth ? st->ops[st->depth - 1] : st->init);
}

static void old_color_push(conv_ctx *c, char *ops)
{
  if (!ops)
    return;
  if (c->old_depth < 64)
    c->old_colors[c->old_depth++] = ops;
  else
    free(ops);
  set_colorstack_ops(c, ops);
}

static void old_color_pop(conv_ctx *c)
{
  if (c->old_depth > 0)
    free(c->old_colors[--c->old_depth]);
  set_colorstack_ops(c, c->old_depth ? c->old_colors[c->old_depth - 1] : "0 g 0 G");
}

static cached_image *get_image(conv_ctx *c, const char *file)
{
  tbuf *data = c->w->res.load(c->w->res.env, file, RES_IMAGE);
  if (!data)
  {
    warn_once(c, file, "image not found: %s", file);
    return NULL;
  }
  for (cached_image *ci = c->w->images; ci; ci = ci->next)
    if (ci->file == data)
    {
      tbuf_drop(data);
      return ci;
    }
  cached_image *ci = calloc(1, sizeof *ci);
  if (!ci)
    abort();
  ci->file = data; /* keeps the pointer unique while cached */
  if (data->len >= 5 && !memcmp(data->data, "%PDF-", 5))
  {
    ci->pdf = true;
    ci->doc = pr_open(data->data, data->len);
    if (!ci->doc)
      warn_once(c, file, "unreadable PDF figure: %s", file);
    else if (pr_is_encrypted(ci->doc))
    {
      warn_once(c, file, "encrypted PDF figures are not previewed: %s", file);
      pr_close(ci->doc);
      ci->doc = NULL;
    }
  }
  else
    image_encode(data->data, data->len, file, &ci->img, c->warnings);
  ci->next = c->w->images;
  c->w->images = ci;
  return ci;
}

static xobj_use *use_xobject(conv_ctx *c, cached_image *ci, int page, int box)
{
  for (int i = 0; i < c->nxobjs; i++)
    if (c->xobjs[i].ci == ci && c->xobjs[i].page == page && c->xobjs[i].box == box)
      return &c->xobjs[i];
  if (ci->pdf ? !ci->doc : !ci->img.ok)
    return NULL;
  xobj_use u = { ci, page, box, 0, "", 0, 0, { 0 } };
  if (ci->pdf)
  {
    pr_page_info info;
    if (!pr_page(ci->doc, pr_normalize_page(ci->doc, page) - 1, box ? box : PR_BOX_CROP, &info))
      return NULL;
    double m[6];
    pr_rotation_matrix(info.rotate, m);
    pr_transform_box(m, info.box, u.bbox);
  }
  else
  {
    u.w = ci->img.width * 72.0 / (ci->img.xdpi > 0 ? ci->img.xdpi : 72);
    u.h = ci->img.height * 72.0 / (ci->img.ydpi > 0 ? ci->img.ydpi : 72);
  }
  u.obj = pdfw_alloc(c->pw);
  snprintf(u.name, sizeof u.name, "Im%d", c->nxobjs + 1);
  if (c->nxobjs == c->capxobjs)
  {
    c->capxobjs = c->capxobjs ? c->capxobjs * 2 : 8;
    c->xobjs = realloc(c->xobjs, sizeof(xobj_use) * c->capxobjs);
    if (!c->xobjs)
      abort();
  }
  c->xobjs[c->nxobjs] = u;
  return &c->xobjs[c->nxobjs++];
}

static void do_image(conv_ctx *c, const char *s, const char *end, double cx, double cy)
{
  mat m = { 1, 0, 0, 1, 0, 0 };
  bool have_matrix = false, hide = false;
  double width = 0, height = 0;
  int page = 1, box = 0;
  char *file = NULL;
  const char *p = s;
  while (p < end)
  {
    p = skip_ws(p, end);
    if (p >= end)
      break;
    bool ok;
    if (*p == '(')
    {
      const char *q = skip_value(p, end);
      file = xstrndup(p + 1, (size_t)(q - p >= 2 ? q - p - 2 : 0));
      p = q;
      break;
    }
    if (*p == '@')
    {
      while (p < end && !isspace((unsigned char)*p))
        p++;
      continue;
    }
    if (!strncmp(p, "matrix", 6))
    {
      p += 6;
      double v[6];
      for (int i = 0; i < 6; i++)
      {
        char *q;
        v[i] = strtod(p, &q);
        p = q;
      }
      m = (mat){ v[0], v[1], v[2], v[3], v[4], v[5] };
      have_matrix = true;
    }
    else if (!strncmp(p, "page", 4) && !strncmp(p, "pagebox", 7))
    {
      p = skip_ws(p + 7, end);
      static const char *const boxes[] = { NULL, "cropbox", "mediabox", "bleedbox", "trimbox", "artbox" };
      for (int i = 1; i <= 5; i++)
        if (!strncmp(p, boxes[i], strlen(boxes[i])))
          box = i;
      while (p < end && !isspace((unsigned char)*p))
        p++;
    }
    else if (!strncmp(p, "page", 4))
    {
      char *q;
      page = (int)strtol(p + 4, &q, 10);
      p = q;
    }
    else if (!strncmp(p, "width", 5))
    {
      p += 5;
      width = parse_dimen(&p, end, &ok);
    }
    else if (!strncmp(p, "height", 6))
    {
      p += 6;
      height = parse_dimen(&p, end, &ok);
    }
    else if (!strncmp(p, "hide", 4))
    {
      hide = true;
      p += 4;
    }
    else
    {
      while (p < end && !isspace((unsigned char)*p))
        p++;
    }
  }
  if (!file || hide || !c->render)
  {
    free(file);
    return;
  }
  cached_image *ci = get_image(c, file);
  xobj_use *u = ci ? use_xobject(c, ci, page, box) : NULL;
  if (!u)
  {
    free(file);
    return;
  }
  free(file);
  mat place;
  double bw = ci->pdf ? u->bbox[2] - u->bbox[0] : u->w, bh = ci->pdf ? u->bbox[3] - u->bbox[1] : u->h;
  if (have_matrix)
    place = m;
  else
  {
    double sx = 1, sy = 1;
    if (width > 0 && height > 0)
      sx = width / bw, sy = height / bh;
    else if (width > 0)
      sx = sy = width / bw;
    else if (height > 0)
      sx = sy = height / bh;
    place = (mat){ sx, 0, 0, sy, 0, 0 };
    if (ci->pdf)
      place = mat_mul(mat_translate(-u->bbox[0], -u->bbox[1]), place);
  }
  if (!ci->pdf)
    place = mat_mul((mat){ u->w, 0, 0, u->h, 0, 0 }, place);
  emit_q(c);
  emit_cm(c, mat_mul(place, mat_translate(cx, cy)));
  pbuf_printf(&cur(c)->content, "/%s Do\n", u->name);
  emit_Q(c);
}

static bool parse_transform(const char *p, const char *end, mat *m)
{
  *m = (mat){ 1, 0, 0, 1, 0, 0 };
  bool any = false;
  while (p < end)
  {
    p = skip_ws(p, end);
    if (p >= end)
      break;
    char *q;
    if (!strncmp(p, "rotate", 6))
    {
      double a = strtod(p + 6, &q) * M_PI / 180.0;
      *m = mat_mul(*m, (mat){ cos(a), sin(a), -sin(a), cos(a), 0, 0 });
      p = q;
      any = true;
    }
    else if (!strncmp(p, "xscale", 6))
    {
      double s = strtod(p + 6, &q);
      *m = mat_mul(*m, (mat){ s, 0, 0, 1, 0, 0 });
      p = q;
      any = true;
    }
    else if (!strncmp(p, "yscale", 6))
    {
      double s = strtod(p + 6, &q);
      *m = mat_mul(*m, (mat){ 1, 0, 0, s, 0, 0 });
      p = q;
      any = true;
    }
    else if (!strncmp(p, "scale", 5))
    {
      double sx = strtod(p + 5, &q);
      const char *r = q;
      double sy = strtod(r, &q);
      if (q == r)
        sy = sx;
      *m = mat_mul(*m, (mat){ sx, 0, 0, sy, 0, 0 });
      p = q;
      any = true;
    }
    else if (!strncmp(p, "matrix", 6))
    {
      double v[6];
      p += 6;
      for (int i = 0; i < 6; i++)
      {
        v[i] = strtod(p, &q);
        p = q;
      }
      *m = mat_mul(*m, (mat){ v[0], v[1], v[2], v[3], v[4], v[5] });
      any = true;
    }
    else
    {
      while (p < end && !isspace((unsigned char)*p))
        p++;
    }
  }
  return any;
}

static void push_offset(conv_ctx *c, double x, double y)
{
  if (c->off_depth < 64)
  {
    c->off_stack[c->off_depth][0] = c->ox;
    c->off_stack[c->off_depth][1] = c->oy;
    c->off_depth++;
  }
  c->ox += x;
  c->oy += y;
}

static void pop_offset(conv_ctx *c)
{
  if (c->off_depth > 0)
  {
    c->off_depth--;
    c->ox = c->off_stack[c->off_depth][0];
    c->oy = c->off_stack[c->off_depth][1];
  }
}

static void write_form(conv_ctx *c, target *t, const char *extra);

static void do_special(conv_ctx *c, const char *s, size_t n, double h, double v);

/* The XDV length is the authority; parsers below use NUL-terminated
 * idioms (strtod, strncmp, sscanf). Copy the payload into a buffer with
 * 16 trailing NULs — longer than any fixed-length token probe — so no
 * scan can read past the payload or bleed into the next opcode. */
static void do_special_bounded(conv_ctx *c, const char *s, size_t n, double h, double v)
{
  char *copy = malloc(n + 16);
  if (!copy)
    abort();
  memcpy(copy, s, n);
  memset(copy + n, 0, 16);
  do_special(c, copy, n, h, v);
  free(copy);
}

static void do_special(conv_ctx *c, const char *s, size_t n, double h, double v)
{
  const char *end = s + n;
  double cx = X(c, h), cy = Y(c, v);
  s = skip_ws(s, end);
#define IS(pfx) ((size_t)(end - s) >= sizeof(pfx) - 1 && !strncmp(s, pfx, sizeof(pfx) - 1))
  if (IS("pdfcolorstackinit"))
    do_pdfcolorstack(c, s + 17, end, true);
  else if (IS("pdfcolorstack"))
    do_pdfcolorstack(c, skip_ws(s + 13, end), end, false);
  else if (IS("color push"))
    old_color_push(c, color_spec_ops(s + 10, end));
  else if (IS("color pop"))
    old_color_pop(c);
  else if (IS("color "))
  {
    char *ops = color_spec_ops(s + 6, end);
    if (ops)
    {
      set_colorstack_ops(c, ops);
      free(ops);
    }
  }
  else if (IS("background"))
  {
    char *ops = color_spec_ops(s + 10, end);
    if (ops)
    {
      double r = 1, g = 1, b = 1;
      if (sscanf(ops, "%lf %lf %lf rg", &r, &g, &b) == 3 || sscanf(ops, "%lf g", &r) == 1)
      {
        if (!strstr(ops, "rg"))
          g = b = r;
        c->background[0] = r;
        c->background[1] = g;
        c->background[2] = b;
        c->has_background = !(r == 1 && g == 1 && b == 1);
      }
      free(ops);
    }
  }
  else if (IS("x:gsave"))
    emit_q(c);
  else if (IS("x:grestore"))
    emit_Q(c);
  else if (IS("x:rotate"))
  {
    double a = strtod(s + 8, NULL) * M_PI / 180.0;
    emit_cm(c, mat_about((mat){ cos(a), sin(a), -sin(a), cos(a), 0, 0 }, cx, cy));
  }
  else if (IS("x:scale"))
  {
    char *q;
    double sx = strtod(s + 7, &q);
    double sy = strtod(q, NULL);
    emit_cm(c, mat_about((mat){ sx, 0, 0, sy, 0, 0 }, cx, cy));
  }
  else if (IS("pdf:") || IS("x:"))
  {
    const char *p = skip_ws(s + (IS("pdf:") ? 4 : 2), end);
    const char *cmd = p;
    /* "pdf:bann<<...>>" has no separator: the command is the letters */
    while (p < end && isalpha((unsigned char)*p))
      p++;
    size_t cl = (size_t)(p - cmd);
    const char *arg = skip_ws(p, end);
#define CMD(x) (cl == sizeof(x) - 1 && !strncmp(cmd, x, cl))
    if (CMD("pagesize"))
      ;
    else if (CMD("btrans") || CMD("bt"))
    {
      emit_q(c);
      mat m;
      if (parse_transform(arg, end, &m))
        emit_cm(c, mat_about(m, cx, cy));
    }
    else if (CMD("etrans") || CMD("et"))
      emit_Q(c);
    else if (CMD("bcontent"))
    {
      emit_q(c);
      emit_cm(c, mat_translate(cx, cy));
      push_offset(c, cx, cy);
    }
    else if (CMD("econtent"))
    {
      pop_offset(c);
      emit_Q(c);
    }
    else if (CMD("code") || CMD("direct"))
      emit_raw(c, arg, (size_t)(end - arg));
    else if (CMD("literal"))
    {
      if (!strncmp(arg, "direct", 6))
        emit_raw(c, arg + 6, (size_t)(end - arg - 6));
      else
      {
        emit_cm(c, mat_translate(cx, cy));
        emit_raw(c, arg, (size_t)(end - arg));
        emit_cm(c, mat_translate(-cx, -cy));
      }
    }
    else if (CMD("content"))
    {
      emit_q(c);
      emit_cm(c, mat_translate(cx, cy));
      emit_raw(c, arg, (size_t)(end - arg));
      emit_Q(c);
    }
    else if (CMD("bcolor") || CMD("bc"))
      old_color_push(c, color_spec_ops(arg, end));
    else if (CMD("ecolor") || CMD("ec"))
      old_color_pop(c);
    else if (CMD("scolor") || CMD("sc"))
    {
      char *ops = color_spec_ops(arg, end);
      if (ops)
      {
        set_colorstack_ops(c, ops);
        free(ops);
      }
    }
    else if (CMD("image"))
      do_image(c, arg, end, cx, cy);
    else if (CMD("obj"))
    {
      const char *q = arg;
      if (q < end && *q == '@')
      {
        const char *ne = q;
        while (ne < end && !isspace((unsigned char)*ne))
          ne++;
        named_obj *o = named_get(c, q, (size_t)(ne - q), true);
        char *val = subst_names(c, ne, (size_t)(end - ne), cx, cy);
        const char *vs = skip_ws(val, val + strlen(val));
        if (vs[0] == '<' && vs[1] == '<')
        {
          if (o->kind != NO_DICT)
          {
            kv_free(&o->dict);
            o->kind = NO_DICT;
          }
          merge_dict(&o->dict, vs);
        }
        else if (vs[0] == '[')
        {
          o->kind = NO_ARRAY;
          pbuf_clear(&o->array);
          pbuf_append(&o->array, vs + 1, (size_t)(skip_value(vs, vs + strlen(vs)) - vs - 2 > 0
                                                      ? skip_value(vs, vs + strlen(vs)) - vs - 2
                                                      : 0));
        }
        else
        {
          o->kind = NO_RAW;
          pbuf_clear(&o->array);
          pbuf_puts(&o->array, vs);
        }
        free(val);
      }
    }
    else if (CMD("put"))
    {
      const char *q = arg, *ne = q;
      while (ne < end && !isspace((unsigned char)*ne))
        ne++;
      char *val = subst_names(c, ne, (size_t)(end - ne), cx, cy);
      if ((size_t)(ne - q) == 10 && !strncmp(q, "@resources", 10))
        merge_resources(&cur(c)->res, val);
      else if (q < end && *q == '@' && strncmp(q, "@thispage", 9) && strncmp(q, "@prevpage", 9) &&
               strncmp(q, "@nextpage", 9))
      {
        named_obj *o = named_get(c, q, (size_t)(ne - q), true);
        const char *vs = skip_ws(val, val + strlen(val));
        if (o->kind == NO_UNDEF)
          o->kind = vs[0] == '[' ? NO_ARRAY : NO_DICT;
        if (o->kind == NO_DICT || o->kind == NO_STREAM || o->kind == NO_FORM)
          merge_dict(&o->dict, vs);
        else if (o->kind == NO_ARRAY && vs[0] == '[')
        {
          const char *ve = skip_value(vs, vs + strlen(vs));
          if (o->array.len)
            pbuf_putc(&o->array, ' ');
          if (ve - vs >= 2)
            pbuf_append(&o->array, vs + 1, (size_t)(ve - vs - 2));
        }
      }
      free(val);
    }
    else if (CMD("stream") || CMD("fstream"))
    {
      const char *q = arg, *ne = q;
      while (ne < end && !isspace((unsigned char)*ne))
        ne++;
      if (q < end && *q == '@' && CMD("stream"))
      {
        named_obj *o = named_get(c, q, (size_t)(ne - q), true);
        const char *ds = skip_ws(ne, end), *de = skip_value(ds, end);
        o->kind = NO_STREAM;
        pbuf_clear(&o->array);
        /* decode literal string escapes */
        for (const char *r = ds + 1; r < de - 1; r++)
        {
          if (*r == '\\' && r + 1 < de - 1)
          {
            r++;
            char e = *r;
            pbuf_putc(&o->array, e == 'n' ? '\n' : e == 'r' ? '\r' : e == 't' ? '\t' : e);
          }
          else
            pbuf_putc(&o->array, *r);
        }
        char *dict = subst_names(c, de, (size_t)(end - de), cx, cy);
        merge_dict(&o->dict, dict);
        free(dict);
      }
    }
    else if (CMD("bxobj") || CMD("beginxobj"))
    {
      if (c->ntargets >= 8)
        return;
      const char *q = arg, *ne = q;
      while (ne < end && !isspace((unsigned char)*ne))
        ne++;
      if (q >= end || *q != '@')
        return;
      named_obj *o = named_get(c, q, (size_t)(ne - q), true);
      o->kind = NO_FORM;
      double w = 0, hh = 0, d = 0, bbox[4] = { 0, 0, 0, 0 };
      bool have_bbox = false;
      const char *r = ne;
      while (r < end)
      {
        r = skip_ws(r, end);
        bool ok;
        if (!strncmp(r, "width", 5))
        {
          r += 5;
          w = parse_dimen(&r, end, &ok);
        }
        else if (!strncmp(r, "height", 6))
        {
          r += 6;
          hh = parse_dimen(&r, end, &ok);
        }
        else if (!strncmp(r, "depth", 5))
        {
          r += 5;
          d = parse_dimen(&r, end, &ok);
        }
        else if (!strncmp(r, "bbox", 4))
        {
          r += 4;
          char *z;
          for (int i = 0; i < 4; i++)
          {
            bbox[i] = strtod(r, &z);
            r = z;
          }
          have_bbox = true;
        }
        else
        {
          while (r < end && !isspace((unsigned char)*r))
            r++;
        }
      }
      if (!have_bbox)
      {
        bbox[0] = 0;
        bbox[1] = -d;
        bbox[2] = w;
        bbox[3] = hh;
      }
      text_end(c);
      target *t = &c->targets[c->ntargets++];
      memset(t, 0, sizeof *t);
      t->form = o;
      memcpy(t->bbox, bbox, sizeof bbox);
      t->ox = cx;
      t->oy = cy;
      t->q_base = c->q_depth;
      /* Forms may be defined on pages replayed without drawing (hybrid
       * previews) and used later: always record their content. */
      t->saved_render = c->render;
      c->render = true;
      push_offset(c, cx, cy);
    }
    else if (CMD("exobj") || CMD("endxobj"))
    {
      if (c->ntargets <= 1)
        return;
      text_end(c);
      while (c->q_depth > cur(c)->q_base)
        emit_Q(c);
      pop_offset(c);
      char *extra = subst_names(c, arg, (size_t)(end - arg), cx, cy);
      write_form(c, cur(c), extra);
      free(extra);
      pbuf_free(&cur(c)->content);
      resources_free(&cur(c)->res);
      c->render = cur(c)->saved_render;
      c->ntargets--;
    }
    else if (CMD("uxobj") || CMD("usexobj"))
    {
      const char *q = arg, *ne = q;
      while (ne < end && !isspace((unsigned char)*ne))
        ne++;
      named_obj *o = named_get(c, q, (size_t)(ne - q), false);
      if (!o || o->kind != NO_FORM || !c->render)
      {
        if (c->render)
          warn_once(c, "uxobj", "form XObject used before definition%s", "");
        return;
      }
      char name[32];
      snprintf(name, sizeof name, "X%d", o->obj);
      char ref[32];
      snprintf(ref, sizeof ref, "%d 0 R", o->obj);
      kv_set(&cur(c)->res.cat[CAT_XOBJECT], name, strlen(name), ref, strlen(ref));
      emit_q(c);
      emit_cm(c, mat_translate(cx, cy));
      pbuf_printf(&cur(c)->content, "/%s Do\n", name);
      emit_Q(c);
    }
    else if (CMD("ann") || CMD("bann") || CMD("eann") || CMD("dest") || CMD("outline") || CMD("out") ||
             CMD("docinfo") || CMD("docview") || CMD("close") || CMD("names") || CMD("tounicode") ||
             CMD("mapline") || CMD("mapfile") || CMD("fontmapline") || CMD("fontmapfile") ||
             CMD("minorversion") || CMD("majorversion") || CMD("encrypt") || CMD("link") || CMD("nolink") ||
             CMD("thread") || CMD("article") || CMD("bead") || CMD("bop") || CMD("eop") || CMD("font") ||
             CMD("pageresources") || CMD("backupfont") || CMD("setfillcolor") || CMD("setstrokecolor"))
      ; /* navigation/metadata: irrelevant for an editing preview */
    else if (c->render)
    {
      char tmp[48];
      snprintf(tmp, sizeof tmp, "%.*s", (int)(cl < 40 ? cl : 40), cmd);
      warn_once(c, tmp, "unsupported special ignored: %s", tmp);
    }
#undef CMD
  }
  else if (IS("papersize=") || IS("src:") || IS("header=") || IS("landscape") || IS("!") || IS("em:") ||
           IS("line:") || IS("html:") || IS("dvipdfmx:") || IS("dvips:"))
    ;
  else if (IS("ps:") || IS("PSfile") || IS("\" ") || IS("\"") || IS("psfile"))
  {
    if (c->render)
      warn_once(c, "postscript", "PostScript specials cannot be previewed%s", "");
  }
#undef IS
}

/* ------------------------------------------------------------------ */
/* DVI interpreter                                                     */

typedef struct {
  double h, v, w, x, y, z;
} dvi_regs;

typedef struct {
  font_table *fonts;
  font_slot *font;
  double scale;       /* multiplier from stream units to DVI units */
  dvi_regs r;
  dvi_regs stack[MAX_STACK];
  int sp;
} dvi_state;

static void interpret(conv_ctx *c, dvi_state *st, const unsigned char *p, const unsigned char *end, int depth);

static void run_packet(conv_ctx *c, font_slot *vfslot, const unsigned char *p, size_t len, double h, double v,
                       int depth)
{
  dvi_state *st = calloc(1, sizeof *st);
  if (!st)
    abort();
  st->fonts = vfslot->vf_local;
  st->font = vfslot->vf_local && vfslot->vf_local->n ? vfslot->vf_local->slots[0] : NULL;
  st->scale = vfslot->size / 1048576.0;
  st->r.h = h;
  st->r.v = v;
  interpret(c, st, p, p + len, depth);
  free(st);
}

static void interpret(conv_ctx *c, dvi_state *st, const unsigned char *p, const unsigned char *end, int depth)
{
  while (p < end)
  {
    long n = xdv_insn_length(p, end);
    if (n <= 0)
      break;
    int op = p[0];
    double sc = st->scale;
    if (op < 128 || (op >= DVI_SET1 && op < DVI_SET1 + 4) || (op >= DVI_PUT1 && op < DVI_PUT1 + 4))
    {
      unsigned code;
      bool set = true;
      if (op < 128)
        code = (unsigned)op;
      else if (op < DVI_SET_RULE)
        code = be(p + 1, op - DVI_SET1 + 1);
      else
      {
        code = be(p + 1, op - DVI_PUT1 + 1);
        set = false;
      }
      if (st->font)
      {
        draw_char(c, st->font, code, st->r.h, st->r.v, depth);
        if (set)
          st->r.h += char_width(st->font, code);
      }
    }
    else if (op == DVI_SET_RULE || op == DVI_PUT_RULE)
    {
      double a = sbe(p + 1, 4) * sc, b = sbe(p + 5, 4) * sc;
      emit_rule(c, st->r.h, st->r.v, b, a);
      if (op == DVI_SET_RULE)
        st->r.h += b;
    }
    else if (op == DVI_PUSH)
    {
      if (st->sp < MAX_STACK)
        st->stack[st->sp++] = st->r;
    }
    else if (op == DVI_POP)
    {
      if (st->sp > 0)
        st->r = st->stack[--st->sp];
    }
    else if (op >= DVI_RIGHT1 && op < DVI_RIGHT1 + 4)
      st->r.h += sbe(p + 1, op - DVI_RIGHT1 + 1) * sc;
    else if (op == DVI_W0)
      st->r.h += st->r.w;
    else if (op >= DVI_W1 && op < DVI_W1 + 4)
      st->r.h += (st->r.w = sbe(p + 1, op - DVI_W1 + 1) * sc);
    else if (op == DVI_X0)
      st->r.h += st->r.x;
    else if (op >= DVI_X1 && op < DVI_X1 + 4)
      st->r.h += (st->r.x = sbe(p + 1, op - DVI_X1 + 1) * sc);
    else if (op >= DVI_DOWN1 && op < DVI_DOWN1 + 4)
      st->r.v += sbe(p + 1, op - DVI_DOWN1 + 1) * sc;
    else if (op == DVI_Y0)
      st->r.v += st->r.y;
    else if (op >= DVI_Y1 && op < DVI_Y1 + 4)
      st->r.v += (st->r.y = sbe(p + 1, op - DVI_Y1 + 1) * sc);
    else if (op == DVI_Z0)
      st->r.v += st->r.z;
    else if (op >= DVI_Z1 && op < DVI_Z1 + 4)
      st->r.v += (st->r.z = sbe(p + 1, op - DVI_Z1 + 1) * sc);
    else if ((op >= DVI_FNT_NUM_0 && op < DVI_FNT1) || (op >= DVI_FNT1 && op < DVI_FNT1 + 4))
    {
      int32_t k = op < DVI_FNT1 ? op - DVI_FNT_NUM_0 : sbe(p + 1, op - DVI_FNT1 + 1);
      st->font = table_find(st->fonts, k);
    }
    else if (op >= DVI_XXX1 && op < DVI_XXX1 + 4)
    {
      int k = op - DVI_XXX1 + 1;
      do_special_bounded(c, (const char *)p + 1 + k, (size_t)(n - 1 - k), st->r.h, st->r.v);
    }
    else if (op >= DVI_FNT_DEF1 && op < DVI_FNT_DEF1 + 4)
    {
      if (depth == 0)
        define_tfm_font(c, st->fonts, p, op - DVI_FNT_DEF1 + 1);
    }
    else if (op == XDV_NATIVE_FONT_DEF)
    {
      if (depth == 0)
        define_native_font(c, st->fonts, p);
    }
    else if (op == XDV_GLYPHS || op == XDV_TEXT_AND_GLYPHS)
    {
      const unsigned char *q = p + 1;
      unsigned text_len = op == XDV_TEXT_AND_GLYPHS ? be(p + 1, 2) : 0;
      if (op == XDV_TEXT_AND_GLYPHS)
        q += 2 + 2 * text_len;
      int32_t w = sbe(q, 4);
      int cnt = (int)be(q + 4, 2);
      bool actual_text = text_len && c->render && st->font && st->font->kind == FS_NATIVE;
      if (actual_text)
      {
        /* Preserve the original UTF-16 text when shaping produces
         * ligatures or glyphs that have no Unicode cmap entry. */
        text_end(c);
        pbuf_puts(&cur(c)->content, "/Span<</ActualText<FEFF");
        for (unsigned i = 0; i < 2 * text_len; i++)
          pbuf_printf(&cur(c)->content, "%02X", p[3 + i]);
        pbuf_puts(&cur(c)->content, ">>>BDC\n");
      }
      if (st->font)
        draw_native_glyphs(c, st->font, st->r.h, st->r.v, q + 6, q + 6 + 8 * cnt, cnt);
      if (actual_text)
        emit_raw(c, "EMC", 3);
      st->r.h += w;
    }
    p += n;
  }
}

/* ------------------------------------------------------------------ */
/* Output objects                                                      */

static void write_resources_body(conv_ctx *c, pbuf *b, resources *r)
{
  pbuf_printf(b, "/Font %d 0 R/XObject %d 0 R", c->font_dict_obj, c->xobject_dict_obj);
  for (int i = 0; i < NCAT; i++)
  {
    if (i == CAT_FONT || i == CAT_XOBJECT)
      continue;
    if (r->cat_ref[i])
      pbuf_printf(b, "/%s %s", cat_names[i], r->cat_ref[i]);
    else if (r->cat[i].n)
    {
      pbuf_printf(b, "/%s<<", cat_names[i]);
      for (int j = 0; j < r->cat[i].n; j++)
        pbuf_printf(b, "/%s %s", r->cat[i].items[j].key, r->cat[i].items[j].value);
      pbuf_puts(b, ">>");
    }
  }
}

static void write_form(conv_ctx *c, target *t, const char *extra)
{
  if (!t->form)
    return;
  /* Form-local XObjects (uxobj inside the form) must stay resolvable:
   * merge them into the shared XObject dictionary. */
  for (int j = 0; j < t->res.cat[CAT_XOBJECT].n; j++)
    kv_set(&c->page_res.cat[CAT_XOBJECT], t->res.cat[CAT_XOBJECT].items[j].key,
           strlen(t->res.cat[CAT_XOBJECT].items[j].key), t->res.cat[CAT_XOBJECT].items[j].value,
           strlen(t->res.cat[CAT_XOBJECT].items[j].value));
  pbuf d = { 0 };
  pbuf_puts(&d, "/Type/XObject/Subtype/Form/BBox[");
  for (int i = 0; i < 4; i++)
  {
    pbuf_real(&d, t->bbox[i]);
    pbuf_putc(&d, i < 3 ? ' ' : ']');
  }
  pbuf_puts(&d, "/Resources<<");
  write_resources_body(c, &d, &t->res);
  pbuf_puts(&d, ">>");
  kvlist ex = { 0 };
  if (extra)
    merge_dict(&ex, extra);
  for (int i = 0; i < ex.n; i++)
    pbuf_printf(&d, "/%s %s", ex.items[i].key, ex.items[i].value);
  kv_free(&ex);
  pbuf_putc(&d, 0);
  pdfw_stream(c->pw, t->form->obj, (char *)d.data, t->content.data, t->content.len, true);
  t->form->written = true;
  pbuf_free(&d);
}

static void write_native_font(conv_ctx *c, pdf_font *p)
{
  native_face *f = p->nf;
  if (!f->deflated.len)
    pbuf_deflate(&f->deflated, f->sfnt, f->sfnt_len, 6);
  int desc = pdfw_alloc(c->pw), fd = pdfw_alloc(c->pw), ff = pdfw_alloc(c->pw), tu = pdfw_alloc(c->pw);
  pbuf *o = pdfw_out(c->pw);
  pdfw_begin(c->pw, p->obj);
  pbuf_puts(o, "<</Type/Font/Subtype/Type0/BaseFont");
  pdfw_name(o, f->psname);
  pbuf_printf(o, "/Encoding/Identity-H/DescendantFonts[%d 0 R]/ToUnicode %d 0 R>>", desc, tu);
  pdfw_end(c->pw);

  pdfw_begin(c->pw, desc);
  pbuf_printf(o, "<</Type/Font/Subtype/%s/BaseFont", f->cff ? "CIDFontType0" : "CIDFontType2");
  pdfw_name(o, f->psname);
  pbuf_printf(o, "/CIDSystemInfo<</Registry(Adobe)/Ordering(Identity)/Supplement 0>>/FontDescriptor %d 0 R/DW 1000",
              fd);
  if (!f->cff)
    pbuf_puts(o, "/CIDToGIDMap/Identity");
  pbuf_puts(o, "/W[");
  for (int g = 0; g < p->nused; g++)
  {
    if (!p->used[g])
      continue;
    pbuf_printf(o, "%d[", g);
    int k = g;
    while (k < p->nused && p->used[k])
    {
      pbuf_real(o, native_advance(f, k) * 1000.0 / f->units_per_em);
      pbuf_putc(o, ' ');
      k++;
    }
    pbuf_puts(o, "]");
    g = k;
  }
  pbuf_puts(o, "]>>");
  pdfw_end(c->pw);

  pdfw_begin(c->pw, fd);
  pbuf_puts(o, "<</Type/FontDescriptor/FontName");
  pdfw_name(o, f->psname);
  pbuf_puts(o, "/Flags 4/FontBBox[");
  for (int i = 0; i < 4; i++)
  {
    pbuf_real(o, f->bbox[i]);
    pbuf_putc(o, i < 3 ? ' ' : ']');
  }
  pbuf_puts(o, "/ItalicAngle ");
  pbuf_real(o, f->italic_angle);
  pbuf_puts(o, "/Ascent ");
  pbuf_real(o, f->ascent);
  pbuf_puts(o, "/Descent ");
  pbuf_real(o, f->descent);
  pbuf_puts(o, "/CapHeight ");
  pbuf_real(o, f->cap_height);
  pbuf_printf(o, "/StemV 80/%s %d 0 R>>", f->cff ? "FontFile3" : "FontFile2", ff);
  pdfw_end(c->pw);

  char dict[96];
  if (f->cff)
    snprintf(dict, sizeof dict, "/Subtype/OpenType");
  else
    snprintf(dict, sizeof dict, "/Length1 %zu", f->sfnt_len);
  pdfw_stream_deflated(c->pw, ff, dict, f->deflated.data, f->deflated.len);

  pbuf cmap = { 0 };
  pbuf_puts(&cmap, "/CIDInit/ProcSet findresource begin\n12 dict begin\nbegincmap\n"
                   "/CIDSystemInfo<</Registry(Adobe)/Ordering(UCS)/Supplement 0>>def\n"
                   "/CMapName/Adobe-Identity-UCS def\n/CMapType 2 def\n"
                   "1 begincodespacerange\n<0000><FFFF>\nendcodespacerange\n");
  int count = 0;
  pbuf block = { 0 };
  for (int g = 0; g < p->nused; g++)
  {
    if (!p->used[g] || !f->to_unicode[g])
      continue;
    uint32_t u = f->to_unicode[g];
    if (u >= 0x10000)
    {
      uint32_t v = u - 0x10000;
      pbuf_printf(&block, "<%04X><%04X%04X>\n", g, 0xD800 + (v >> 10), 0xDC00 + (v & 0x3FF));
    }
    else
      pbuf_printf(&block, "<%04X><%04X>\n", g, u);
    if (++count == 100)
    {
      pbuf_printf(&cmap, "100 beginbfchar\n%.*sendbfchar\n", (int)block.len, (char *)block.data);
      pbuf_clear(&block);
      count = 0;
    }
  }
  if (count)
    pbuf_printf(&cmap, "%d beginbfchar\n%.*sendbfchar\n", count, (int)block.len, (char *)block.data);
  pbuf_puts(&cmap, "endcmap\nCMapName currentdict/CMap defineresource pop\nend\nend\n");
  pdfw_stream(c->pw, tu, NULL, cmap.data, cmap.len, true);
  pbuf_free(&cmap);
  pbuf_free(&block);
}

static void write_type1_font(conv_ctx *c, pdf_font *p)
{
  type1_font *t = p->t1;
  if (!t->deflated.len)
    pbuf_deflate(&t->deflated, t->data, t->len1 + t->len2 + t->len3, 6);
  int first = 256, last = -1;
  for (int i = 0; i < 256; i++)
    if (p->used[i])
    {
      if (i < first)
        first = i;
      last = i;
    }
  if (last < 0)
    first = last = 0;
  int fd = pdfw_alloc(c->pw), ff = pdfw_alloc(c->pw);
  pbuf *o = pdfw_out(c->pw);
  pdfw_begin(c->pw, p->obj);
  pbuf_puts(o, "<</Type/Font/Subtype/Type1/BaseFont");
  pdfw_name(o, t->fontname);
  pbuf_printf(o, "/FirstChar %d/LastChar %d/Widths[", first, last);
  for (int i = first; i <= last; i++)
  {
    pbuf_real(o, p->tfm && p->tfm->exists[i] ? p->tfm->width[i] / 1048.576 : 0);
    pbuf_putc(o, ' ');
  }
  pbuf_printf(o, "]/FontDescriptor %d 0 R", fd);
  if (p->enc)
  {
    pbuf_puts(o, "/Encoding<</Type/Encoding/Differences[");
    bool gap = true;
    for (int i = first; i <= last; i++)
    {
      if (!p->used[i] || !p->enc->glyph[i])
      {
        gap = true;
        continue;
      }
      if (gap)
        pbuf_printf(o, " %d", i);
      pdfw_name(o, p->enc->glyph[i]);
      gap = false;
    }
    pbuf_puts(o, "]>>");
  }
  pbuf_puts(o, ">>");
  pdfw_end(c->pw);
  pdfw_begin(c->pw, fd);
  pbuf_puts(o, "<</Type/FontDescriptor/FontName");
  pdfw_name(o, t->fontname);
  pbuf_puts(o, "/Flags 4/FontBBox[");
  for (int i = 0; i < 4; i++)
  {
    pbuf_real(o, t->bbox[i]);
    pbuf_putc(o, i < 3 ? ' ' : ']');
  }
  pbuf_puts(o, "/ItalicAngle 0/Ascent ");
  pbuf_real(o, t->bbox[3]);
  pbuf_puts(o, "/Descent ");
  pbuf_real(o, t->bbox[1]);
  pbuf_puts(o, "/CapHeight ");
  pbuf_real(o, t->bbox[3]);
  pbuf_printf(o, "/StemV 80/FontFile %d 0 R>>", ff);
  pdfw_end(c->pw);
  char dict[96];
  snprintf(dict, sizeof dict, "/Length1 %zu/Length2 %zu/Length3 %zu", t->len1, t->len2, t->len3);
  pdfw_stream_deflated(c->pw, ff, dict, t->deflated.data, t->deflated.len);
}

static void write_xobject(conv_ctx *c, xobj_use *u)
{
  cached_image *ci = u->ci;
  if (!ci->pdf)
  {
    char dict[320];
    int smask = 0;
    if (ci->img.smask.len)
    {
      smask = pdfw_alloc(c->pw);
      char sd[128];
      snprintf(sd, sizeof sd, "/Type/XObject/Subtype/Image/Width %d/Height %d/ColorSpace/DeviceGray/BitsPerComponent 8",
               ci->img.width, ci->img.height);
      pdfw_stream_deflated(c->pw, smask, sd, ci->img.smask.data, ci->img.smask.len);
    }
    if (ci->img.dct)
    {
      snprintf(dict, sizeof dict, "/Type/XObject/Subtype/Image%s/Filter/DCTDecode", ci->img.dict);
      pdfw_stream(c->pw, u->obj, dict, ci->img.data.data, ci->img.data.len, false);
    }
    else
    {
      if (smask)
        snprintf(dict, sizeof dict, "/Type/XObject/Subtype/Image%s/SMask %d 0 R", ci->img.dict, smask);
      else
        snprintf(dict, sizeof dict, "/Type/XObject/Subtype/Image%s", ci->img.dict);
      pdfw_stream_deflated(c->pw, u->obj, dict, ci->img.data.data, ci->img.data.len);
    }
    return;
  }
  pr_page_info info;
  pr_page(ci->doc, pr_normalize_page(ci->doc, u->page) - 1, u->box ? u->box : PR_BOX_CROP, &info);
  double m[6];
  pr_rotation_matrix(info.rotate, m);
  pdfw_import *imp = pdfw_import_begin(c->pw, ci->doc);
  pbuf d = { 0 };
  pbuf_puts(&d, "/Type/XObject/Subtype/Form/BBox[");
  for (int i = 0; i < 4; i++)
  {
    pbuf_real(&d, info.box[i]);
    pbuf_putc(&d, i < 3 ? ' ' : ']');
  }
  pbuf_puts(&d, "/Matrix[");
  for (int i = 0; i < 6; i++)
  {
    pbuf_real(&d, m[i]);
    pbuf_putc(&d, i < 5 ? ' ' : ']');
  }
  pbuf_puts(&d, "/Resources ");
  pdfw_import_value(imp, &d, info.resources);
  pr_obj *group = pr_get(ci->doc, info.page, "Group");
  if (group)
  {
    pbuf_puts(&d, "/Group ");
    pdfw_import_value(imp, &d, group);
  }
  /* page content: one stream or an array of streams */
  pr_obj *contents = pr_get(ci->doc, info.page, "Contents");
  pbuf data = { 0 };
  bool raw_single = false;
  if (contents && contents->type == PR_STREAM)
  {
    pr_obj *filter = pr_get(ci->doc, contents, "Filter");
    pr_obj *parms = pr_get(ci->doc, contents, "DecodeParms");
    size_t len;
    const unsigned char *raw = pr_stream_raw(ci->doc, contents, &len);
    if (raw)
    {
      pbuf_append(&data, raw, len);
      if (filter)
      {
        pbuf_puts(&d, "/Filter ");
        pdfw_import_value(imp, &d, filter);
      }
      if (parms)
      {
        pbuf_puts(&d, "/DecodeParms ");
        pdfw_import_value(imp, &d, parms);
      }
      raw_single = true;
    }
  }
  else if (contents && contents->type == PR_ARRAY)
  {
    for (int i = 0; i < contents->u.arr.n; i++)
    {
      pr_obj *s = pr_resolve(ci->doc, contents->u.arr.items[i]);
      if (!s || s->type != PR_STREAM)
        continue;
      if (!pr_stream_decode(ci->doc, s, &data))
      {
        warn_once(c, "pdfcontent", "PDF figure uses an unsupported content filter%s", "");
        pbuf_clear(&data);
        break;
      }
      pbuf_putc(&data, '\n');
    }
  }
  pbuf_putc(&d, 0);
  if (raw_single)
    pdfw_stream(c->pw, u->obj, (char *)d.data, data.data, data.len, false);
  else
    pdfw_stream(c->pw, u->obj, (char *)d.data, data.data, data.len, true);
  pbuf_free(&d);
  pbuf_free(&data);
  pdfw_import_end(imp);
}

static void write_named(conv_ctx *c, named_obj *o)
{
  if (o->written)
    return;
  o->written = true;
  pbuf *out = pdfw_out(c->pw);
  switch (o->kind)
  {
    case NO_DICT:
      pdfw_begin(c->pw, o->obj);
      pbuf_puts(out, "<<");
      for (int i = 0; i < o->dict.n; i++)
        pbuf_printf(out, "/%s %s", o->dict.items[i].key, o->dict.items[i].value);
      pbuf_puts(out, ">>");
      pdfw_end(c->pw);
      break;
    case NO_ARRAY:
      pdfw_begin(c->pw, o->obj);
      pbuf_putc(out, '[');
      pbuf_append(out, o->array.data, o->array.len);
      pbuf_putc(out, ']');
      pdfw_end(c->pw);
      break;
    case NO_RAW:
      pdfw_begin(c->pw, o->obj);
      pbuf_append(out, o->array.data, o->array.len);
      pdfw_end(c->pw);
      break;
    case NO_STREAM:
    {
      pbuf d = { 0 };
      for (int i = 0; i < o->dict.n; i++)
        if (strcmp(o->dict.items[i].key, "Length") && strcmp(o->dict.items[i].key, "Filter"))
          pbuf_printf(&d, "/%s %s", o->dict.items[i].key, o->dict.items[i].value);
      pbuf_putc(&d, 0);
      pdfw_stream(c->pw, o->obj, (char *)d.data, o->array.data, o->array.len, true);
      pbuf_free(&d);
      break;
    }
    default:
      pdfw_begin(c->pw, o->obj);
      pbuf_puts(out, "null");
      pdfw_end(c->pw);
  }
}

/* ------------------------------------------------------------------ */
/* Driver                                                              */

xdv2pdf *xdv2pdf_new(xdv_resolver resolver)
{
  xdv2pdf *w = calloc(1, sizeof *w);
  if (!w)
    abort();
  w->res = resolver;
  w->fonts = font_cache_new(resolver);
  return w;
}

void xdv2pdf_free(xdv2pdf *w)
{
  if (!w)
    return;
  for (cached_image *ci = w->images; ci;)
  {
    cached_image *n = ci->next;
    tbuf_drop(ci->file);
    image_free(&ci->img);
    pr_close(ci->doc);
    free(ci);
    ci = n;
  }
  font_cache_free(w->fonts);
  free(w);
}

/* Drop cached images whose file buffer is no longer current. */
static void prune_images(xdv2pdf *w)
{
  cached_image **pp = &w->images;
  while (*pp)
  {
    cached_image *ci = *pp;
    if (ci->file->refs <= 1) /* only the cache holds it: file was replaced */
    {
      *pp = ci->next;
      tbuf_drop(ci->file);
      image_free(&ci->img);
      pr_close(ci->doc);
      free(ci);
    }
    else
      pp = &ci->next;
  }
}

static void reset_page_state(conv_ctx *c)
{
  c->ox = c->oy = 0;
  c->off_depth = 0;
  c->q_depth = 0;
  c->in_bt = c->in_tj = c->pos_valid = false;
  c->t_font = -1;
  c->has_background = false;
}

int xdv2pdf_write(xdv2pdf *w, const xdv_range *ranges, int nranges, pbuf *pdf, pbuf *warnings)
{
  w->epoch++;
  conv_ctx *c = calloc(1, sizeof *c);
  if (!c)
    abort();
  c->w = w;
  c->warnings = warnings;
  c->pw = pdfw_new(pdf);
  int catalog = pdfw_alloc(c->pw), pages = pdfw_alloc(c->pw), info = pdfw_alloc(c->pw);
  c->font_dict_obj = pdfw_alloc(c->pw);
  c->xobject_dict_obj = pdfw_alloc(c->pw);
  c->resources_obj = pdfw_alloc(c->pw);
  int *page_objs = NULL, npages = 0, cappages = 0;

  for (int r = 0; r < nranges; r++)
  {
    const xdv_range *rg = &ranges[r];
    uint32_t num, den, mag;
    if (!xdv_index_preamble(rg->index, &num, &den, &mag) || den == 0)
      continue;
    if (mag == 0)
      mag = 1000;
    c->conv = (double)num / den * mag / 1000.0 * 72.0 / 254000.0;
    c->default_w = 595.2756;
    c->default_h = 841.8898;
    for (int i = 0; i < MAX_COLORSTACKS; i++)
    {
      for (int k = 0; k < c->cs[i].depth; k++)
        free(c->cs[i].ops[k]);
      free(c->cs[i].init);
      memset(&c->cs[i], 0, sizeof c->cs[i]);
    }
    for (int i = 0; i < c->old_depth; i++)
      free(c->old_colors[i]);
    c->old_depth = 0;
    font_table *fonts = calloc(1, sizeof *fonts);
    if (!fonts)
      abort();
    int last = rg->first + rg->count;
    if (last > xdv_index_page_count(rg->index))
      last = xdv_index_page_count(rg->index);
    /* Font definitions, colour stacks and named objects persist across
     * pages: replay earlier pages without drawing. */
    dvi_state *st = calloc(1, sizeof *st);
    if (!st)
      abort();
    st->fonts = fonts;
    st->scale = 1;
    c->ntargets = 1;
    memset(&c->targets[0], 0, sizeof c->targets[0]);
    for (int pg = 0; pg < last; pg++)
    {
      const xdv_page *xp = xdv_index_page(rg->index, pg);
      const unsigned char *pb = rg->data + xp->bop + 45, *pe = rg->data + xp->eop_end - 1;
      bool render = pg >= rg->first;
      c->render = render;
      page_size_prescan(c, pb, pe);
      reset_page_state(c);
      memset(&st->r, 0, sizeof st->r);
      st->sp = 0;
      st->font = NULL;
      target *t = &c->targets[0];
      pbuf_clear(&t->content);
      if (render)
      {
        /* page-scoped colour stacks restart from their current value */
        for (int i = 0; i < MAX_COLORSTACKS; i++)
          if (c->cs[i].page || c->cs[i].init)
          {
            const char *ops = c->cs[i].depth ? c->cs[i].ops[c->cs[i].depth - 1] : c->cs[i].init;
            if (ops && *ops)
            {
              pbuf_puts(&t->content, ops);
              pbuf_putc(&t->content, '\n');
            }
          }
        if (c->old_depth)
        {
          pbuf_puts(&t->content, c->old_colors[c->old_depth - 1]);
          pbuf_putc(&t->content, '\n');
        }
      }
      interpret(c, st, pb, pe, 0);
      while (c->ntargets > 1)
      {
        /* unterminated pdf:bxobj at page end */
        pbuf_free(&cur(c)->content);
        resources_free(&cur(c)->res);
        c->render = cur(c)->saved_render;
        c->ntargets--;
      }
      if (!render)
        continue;
      text_end(c);
      while (c->q_depth > 0)
      {
        pbuf_puts(&t->content, "Q\n");
        c->q_depth--;
      }
      pbuf content = { 0 };
      if (c->has_background)
      {
        pbuf_printf(&content, "q ");
        emit_num(&content, c->background[0]);
        emit_num(&content, c->background[1]);
        emit_num(&content, c->background[2]);
        pbuf_printf(&content, "rg 0 0 ");
        emit_num(&content, c->page_w);
        emit_num(&content, c->page_h);
        pbuf_puts(&content, "re f Q\n");
      }
      pbuf_append(&content, t->content.data, t->content.len);
      int cobj = pdfw_alloc(c->pw), pobj = pdfw_alloc(c->pw);
      pdfw_stream(c->pw, cobj, NULL, content.data, content.len, true);
      pbuf_free(&content);
      pbuf *o = pdfw_out(c->pw);
      pdfw_begin(c->pw, pobj);
      pbuf_printf(o, "<</Type/Page/Parent %d 0 R/MediaBox[0 0 ", pages);
      pbuf_real(o, c->page_w);
      pbuf_putc(o, ' ');
      pbuf_real(o, c->page_h);
      pbuf_printf(o, "]/Resources %d 0 R/Contents %d 0 R>>", c->resources_obj, cobj);
      pdfw_end(c->pw);
      if (npages == cappages)
      {
        cappages = cappages ? cappages * 2 : 64;
        page_objs = realloc(page_objs, sizeof(int) * cappages);
        if (!page_objs)
          abort();
      }
      page_objs[npages++] = pobj;
    }
    /* page resources set via pdf:put @resources accumulate document-wide */
    for (int i = 0; i < NCAT; i++)
    {
      for (int j = 0; j < c->targets[0].res.cat[i].n; j++)
        kv_set(&c->page_res.cat[i], c->targets[0].res.cat[i].items[j].key,
               strlen(c->targets[0].res.cat[i].items[j].key), c->targets[0].res.cat[i].items[j].value,
               strlen(c->targets[0].res.cat[i].items[j].value));
      if (c->targets[0].res.cat_ref[i])
      {
        free(c->page_res.cat_ref[i]);
        c->page_res.cat_ref[i] = strdup(c->targets[0].res.cat_ref[i]);
      }
    }
    pbuf_free(&c->targets[0].content);
    resources_free(&c->targets[0].res);
    free(st);
    table_free(fonts);
  }

  /* fonts */
  pbuf *o = pdfw_out(c->pw);
  for (int i = 0; i < c->npfonts; i++)
  {
    if (c->pfonts[i].native)
      write_native_font(c, &c->pfonts[i]);
    else
      write_type1_font(c, &c->pfonts[i]);
  }
  pdfw_begin(c->pw, c->font_dict_obj);
  pbuf_puts(o, "<<");
  for (int i = 0; i < c->npfonts; i++)
    pbuf_printf(o, "/F%d %d 0 R", i + 1, c->pfonts[i].obj);
  pbuf_puts(o, ">>");
  pdfw_end(c->pw);
  for (int i = 0; i < c->nxobjs; i++)
    write_xobject(c, &c->xobjs[i]);
  pdfw_begin(c->pw, c->xobject_dict_obj);
  pbuf_puts(o, "<<");
  for (int i = 0; i < c->nxobjs; i++)
    pbuf_printf(o, "/%s %d 0 R", c->xobjs[i].name, c->xobjs[i].obj);
  for (int j = 0; j < c->page_res.cat[CAT_XOBJECT].n; j++)
    pbuf_printf(o, "/%s %s", c->page_res.cat[CAT_XOBJECT].items[j].key, c->page_res.cat[CAT_XOBJECT].items[j].value);
  pbuf_puts(o, ">>");
  pdfw_end(c->pw);
  pdfw_begin(c->pw, c->resources_obj);
  pbuf_puts(o, "<<");
  write_resources_body(c, o, &c->page_res);
  pbuf_puts(o, "/ProcSet[/PDF/Text/ImageB/ImageC]>>");
  pdfw_end(c->pw);
  for (int i = 0; i < c->nnamed; i++)
    write_named(c, &c->named[i]);

  pdfw_begin(c->pw, pages);
  pbuf_puts(o, "<</Type/Pages/Kids[");
  for (int i = 0; i < npages; i++)
    pbuf_printf(o, "%d 0 R ", page_objs[i]);
  pbuf_printf(o, "]/Count %d>>", npages);
  pdfw_end(c->pw);
  pdfw_begin(c->pw, catalog);
  pbuf_printf(o, "<</Type/Catalog/Pages %d 0 R>>", pages);
  pdfw_end(c->pw);
  pdfw_begin(c->pw, info);
  pbuf_puts(o, "<</Producer(Pitex embedded editing preview)/Subject(Editing preview - not final compiler output)>>");
  pdfw_end(c->pw);
  pdfw_finish(c->pw, catalog, info);

  /* cleanup */
  for (int i = 0; i < c->npfonts; i++)
    free(c->pfonts[i].used);
  free(c->pfonts);
  free(c->xobjs);
  for (int i = 0; i < c->nnamed; i++)
  {
    free(c->named[i].name);
    kv_free(&c->named[i].dict);
    pbuf_free(&c->named[i].array);
  }
  free(c->named);
  resources_free(&c->page_res);
  for (int i = 0; i < MAX_COLORSTACKS; i++)
  {
    for (int k = 0; k < c->cs[i].depth; k++)
      free(c->cs[i].ops[k]);
    free(c->cs[i].init);
  }
  for (int i = 0; i < c->old_depth; i++)
    free(c->old_colors[i]);
  free(page_objs);
  pdfw_free(c->pw);
  free(c);
  prune_images(w);
  return npages > 0 ? 0 : 1;
}
