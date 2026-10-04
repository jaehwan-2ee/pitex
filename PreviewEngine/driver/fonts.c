/* Pitex embedded preview engine — font file readers for PDF embedding.
 * Independently written from the OpenType/TrueType, TFM/VF, Type 1 and
 * pdfTeX font map format descriptions. Pitex-authored (PolyForm Shield). */
#include "fonts.h"

#include <ctype.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef enum { K_NATIVE, K_TFM, K_TYPE1, K_ENC, K_VF } entry_kind;

typedef struct fc_entry {
  struct fc_entry *next;
  entry_kind kind;
  char *name;
  int index;
  tbuf *file;
  void *value; /* NULL cached miss (VF) */
} fc_entry;

struct font_cache {
  xdv_resolver res;
  fc_entry *entries;
  bool map_loaded;
  map_entry *map;
  int nmap, capmap;
  int anon_counter;
};

font_cache *font_cache_new(xdv_resolver resolver)
{
  font_cache *fc = calloc(1, sizeof *fc);
  if (!fc)
    abort();
  fc->res = resolver;
  return fc;
}

static void free_value(fc_entry *e)
{
  if (!e->value)
    return;
  switch (e->kind)
  {
    case K_NATIVE: {
      native_face *f = e->value;
      free(f->sfnt);
      free(f->to_unicode);
      pbuf_free(&f->deflated);
      break;
    }
    case K_TYPE1: {
      type1_font *t = e->value;
      free(t->data);
      pbuf_free(&t->deflated);
      break;
    }
    case K_ENC: {
      enc_vector *v = e->value;
      for (int i = 0; i < 256; i++)
        free(v->glyph[i]);
      break;
    }
    case K_VF: {
      vf_font *v = e->value;
      for (int i = 0; i < v->nfonts; i++)
        free(v->fonts[i].name);
      free(v->fonts);
      break;
    }
    default: break;
  }
  free(e->value);
}

void font_cache_free(font_cache *fc)
{
  if (!fc)
    return;
  for (fc_entry *e = fc->entries; e;)
  {
    fc_entry *n = e->next;
    free_value(e);
    tbuf_drop(e->file);
    free(e->name);
    free(e);
    e = n;
  }
  for (int i = 0; i < fc->nmap; i++)
  {
    free(fc->map[i].tfm);
    free(fc->map[i].psname);
    free(fc->map[i].fontfile);
    free(fc->map[i].encfile);
  }
  free(fc->map);
  free(fc);
}

static fc_entry *lookup(font_cache *fc, entry_kind k, const char *name, int index)
{
  for (fc_entry *e = fc->entries; e; e = e->next)
    if (e->kind == k && e->index == index && strcmp(e->name, name) == 0)
      return e;
  return NULL;
}

static fc_entry *insert(font_cache *fc, entry_kind k, const char *name, int index, tbuf *file, void *value)
{
  fc_entry *e = calloc(1, sizeof *e);
  if (!e)
    abort();
  e->kind = k;
  e->name = strdup(name);
  e->index = index;
  e->file = file;
  e->value = value;
  e->next = fc->entries;
  fc->entries = e;
  return e;
}

static void *xcalloc(size_t n, size_t s)
{
  void *p = calloc(n ? n : 1, s);
  if (!p)
    abort();
  return p;
}

/* ------------------------------------------------------------------ */
/* sfnt                                                                */

static uint32_t rd32(const unsigned char *p) { return (uint32_t)p[0] << 24 | (uint32_t)p[1] << 16 | (uint32_t)p[2] << 8 | p[3]; }
static unsigned rd16(const unsigned char *p) { return (unsigned)p[0] << 8 | p[1]; }
static int rds16(const unsigned char *p) { return (int16_t)rd16(p); }

static bool find_table(const unsigned char *s, size_t len, const char *tag, const unsigned char **out, size_t *olen)
{
  if (len < 12)
    return false;
  unsigned n = rd16(s + 4);
  if (12 + (size_t)n * 16 > len)
    return false;
  for (unsigned i = 0; i < n; i++)
  {
    const unsigned char *r = s + 12 + i * 16;
    if (memcmp(r, tag, 4) == 0)
    {
      uint32_t off = rd32(r + 8), l = rd32(r + 12);
      if (off > len || l > len - off)
        return false;
      *out = s + off;
      *olen = l;
      return true;
    }
  }
  return false;
}

/* Build a standalone sfnt from the face at `face_off` in `file`. */
static unsigned char *extract_face(const unsigned char *file, size_t flen, size_t face_off, size_t *out_len)
{
  if (face_off + 12 > flen)
    return NULL;
  const unsigned char *h = file + face_off;
  unsigned n = rd16(h + 4);
  if (face_off + 12 + (size_t)n * 16 > flen)
    return NULL;
  size_t total = 12 + (size_t)n * 16;
  for (unsigned i = 0; i < n; i++)
  {
    const unsigned char *r = h + 12 + i * 16;
    uint32_t off = rd32(r + 8), l = rd32(r + 12);
    if (off > flen || l > flen - off)
      return NULL;
    total += ((size_t)l + 3) & ~(size_t)3;
  }
  unsigned char *o = xcalloc(total, 1);
  memcpy(o, h, 12);
  size_t pos = 12 + (size_t)n * 16;
  for (unsigned i = 0; i < n; i++)
  {
    const unsigned char *r = h + 12 + i * 16;
    uint32_t off = rd32(r + 8), l = rd32(r + 12);
    unsigned char *rec = o + 12 + i * 16;
    memcpy(rec, r, 8);
    rec[8] = pos >> 24;
    rec[9] = pos >> 16;
    rec[10] = pos >> 8;
    rec[11] = pos;
    memcpy(rec + 12, r + 12, 4);
    memcpy(o + pos, file + off, l);
    pos += ((size_t)l + 3) & ~(size_t)3;
  }
  *out_len = total;
  return o;
}

static void set_unicode(native_face *f, unsigned gid, uint32_t cp)
{
  if (gid && gid < (unsigned)f->num_glyphs && !f->to_unicode[gid])
    f->to_unicode[gid] = cp;
}

static void parse_cmap(native_face *f)
{
  const unsigned char *c;
  size_t len;
  if (!find_table(f->sfnt, f->sfnt_len, "cmap", &c, &len) || len < 4)
    return;
  unsigned n = rd16(c + 2);
  const unsigned char *best = NULL;
  int best_rank = 0;
  size_t best_len = 0;
  bool symbol = false;
  for (unsigned i = 0; i < n && 4 + (i + 1) * 8 <= len; i++)
  {
    const unsigned char *r = c + 4 + i * 8;
    unsigned pid = rd16(r), eid = rd16(r + 2);
    uint32_t off = rd32(r + 4);
    if (off > len - 4)
      continue;
    unsigned fmt = rd16(c + off);
    int rank = 0;
    if (fmt == 12 && ((pid == 3 && eid == 10) || pid == 0))
      rank = 4;
    else if (fmt == 4 && ((pid == 3 && eid == 1) || pid == 0))
      rank = 3;
    else if (fmt == 4 && pid == 3 && eid == 0)
      rank = 1;
    if (rank > best_rank)
    {
      best_rank = rank;
      best = c + off;
      best_len = len - off;
      symbol = rank == 1;
    }
  }
  if (!best)
    return;
  unsigned fmt = rd16(best);
  if (fmt == 12 && best_len >= 16)
  {
    uint32_t groups = rd32(best + 12);
    for (uint32_t g = 0; g < groups && 16 + (size_t)(g + 1) * 12 <= best_len; g++)
    {
      const unsigned char *p = best + 16 + g * 12;
      uint32_t s = rd32(p), e = rd32(p + 4), gid = rd32(p + 8);
      if (e < s || e - s > 0x110000)
        continue;
      for (uint32_t cp = s; cp <= e && cp < 0x110000; cp++)
        set_unicode(f, gid + (cp - s), cp);
    }
  }
  else if (fmt == 4 && best_len >= 14)
  {
    unsigned segx2 = rd16(best + 6);
    size_t need = 16 + (size_t)segx2 * 4;
    if (need > best_len)
      return;
    const unsigned char *ends = best + 14, *starts = ends + segx2 + 2, *deltas = starts + segx2,
                        *ranges = deltas + segx2;
    for (unsigned s = 0; s < segx2 / 2; s++)
    {
      unsigned end = rd16(ends + 2 * s), start = rd16(starts + 2 * s);
      int delta = rds16(deltas + 2 * s);
      unsigned ro = rd16(ranges + 2 * s);
      for (unsigned cp = start; cp <= end && cp != 0xFFFF; cp++)
      {
        unsigned gid;
        if (ro == 0)
          gid = (cp + (unsigned)delta) & 0xFFFF;
        else
        {
          const unsigned char *g = ranges + 2 * s + ro + 2 * (cp - start);
          if (g + 2 > best + best_len)
            break;
          gid = rd16(g);
          if (gid)
            gid = (gid + (unsigned)delta) & 0xFFFF;
        }
        uint32_t u = cp;
        if (symbol && cp >= 0xF000 && cp <= 0xF0FF)
          u = cp - 0xF000;
        set_unicode(f, gid, u);
      }
    }
  }
}

static void read_psname(native_face *f, font_cache *fc)
{
  const unsigned char *t;
  size_t len;
  f->psname[0] = 0;
  if (find_table(f->sfnt, f->sfnt_len, "name", &t, &len) && len >= 6)
  {
    unsigned count = rd16(t + 2), soff = rd16(t + 4);
    for (int pass = 0; pass < 2 && !f->psname[0]; pass++)
    {
      for (unsigned i = 0; i < count && 6 + (size_t)(i + 1) * 12 <= len; i++)
      {
        const unsigned char *r = t + 6 + i * 12;
        unsigned pid = rd16(r), nid = rd16(r + 6), l = rd16(r + 8), o = rd16(r + 10);
        if (nid != 6 || (size_t)soff + o + l > len)
          continue;
        const unsigned char *s = t + soff + o;
        size_t k = 0;
        if (pass == 0 && pid == 3)
        {
          for (unsigned j = 0; j + 1 < l && k < sizeof f->psname - 1; j += 2)
            if (s[j] == 0 && s[j + 1] > 32 && s[j + 1] < 127)
              f->psname[k++] = (char)s[j + 1];
        }
        else if (pass == 1 && pid == 1)
        {
          for (unsigned j = 0; j < l && k < sizeof f->psname - 1; j++)
            if (s[j] > 32 && s[j] < 127)
              f->psname[k++] = (char)s[j];
        }
        f->psname[k] = 0;
        if (k)
          break;
      }
    }
  }
  for (char *p = f->psname; *p; p++)
    if (strchr("()<>[]{}/%#", *p))
      *p = '-';
  if (!f->psname[0])
    snprintf(f->psname, sizeof f->psname, "PitexFont%d", ++fc->anon_counter);
}

native_face *font_native(font_cache *fc, const char *name, int index, pbuf *warnings)
{
  fc_entry *e = lookup(fc, K_NATIVE, name, index);
  if (e)
    return e->value;
  native_face *f = xcalloc(1, sizeof *f);
  tbuf *file = fc->res.load(fc->res.env, name, RES_NATIVE_FONT);
  insert(fc, K_NATIVE, name, index, file, f);
  if (!file)
  {
    pbuf_printf(warnings, "font file not found: %s\n", name);
    return f;
  }
  const unsigned char *d = file->data;
  size_t n = file->len;
  size_t face_off = 0;
  if (n >= 12 && memcmp(d, "ttcf", 4) == 0)
  {
    uint32_t count = rd32(d + 8);
    if (index < 0 || (uint32_t)index >= count || 12 + (size_t)(index + 1) * 4 > n)
    {
      pbuf_printf(warnings, "font collection index %d out of range: %s\n", index, name);
      return f;
    }
    face_off = rd32(d + 12 + index * 4);
  }
  if (face_off + 4 > n)
    return f;
  uint32_t ver = rd32(d + face_off);
  if (ver != 0x00010000 && ver != 0x4F54544F /* OTTO */ && ver != 0x74727565 /* true */)
  {
    pbuf_printf(warnings, "unsupported native font format (not OpenType/TrueType): %s\n", name);
    return f;
  }
  f->sfnt = extract_face(d, n, face_off, &f->sfnt_len);
  if (!f->sfnt)
  {
    pbuf_printf(warnings, "corrupt font file: %s\n", name);
    return f;
  }
  f->cff = ver == 0x4F54544F;
  const unsigned char *t;
  size_t len;
  f->units_per_em = 1000;
  double bb[4] = { 0, -250, 1000, 750 };
  if (find_table(f->sfnt, f->sfnt_len, "head", &t, &len) && len >= 54)
  {
    f->units_per_em = rd16(t + 18);
    if (f->units_per_em < 16)
      f->units_per_em = 1000;
    bb[0] = rds16(t + 36);
    bb[1] = rds16(t + 38);
    bb[2] = rds16(t + 40);
    bb[3] = rds16(t + 42);
  }
  double k = 1000.0 / f->units_per_em;
  for (int i = 0; i < 4; i++)
    f->bbox[i] = bb[i] * k;
  if (find_table(f->sfnt, f->sfnt_len, "maxp", &t, &len) && len >= 6)
    f->num_glyphs = (int)rd16(t + 4);
  f->ascent = f->bbox[3];
  f->descent = f->bbox[1];
  if (find_table(f->sfnt, f->sfnt_len, "hhea", &t, &len) && len >= 36)
  {
    f->ascent = rds16(t + 4) * k;
    f->descent = rds16(t + 6) * k;
    f->num_hmetrics = (int)rd16(t + 34);
  }
  if (find_table(f->sfnt, f->sfnt_len, "hmtx", &t, &len))
  {
    f->hmtx = t;
    f->hmtx_len = len;
    if ((size_t)f->num_hmetrics * 4 > len)
      f->num_hmetrics = (int)(len / 4);
  }
  f->cap_height = f->ascent;
  if (find_table(f->sfnt, f->sfnt_len, "OS/2", &t, &len) && len >= 90 && rd16(t) >= 2)
    f->cap_height = rds16(t + 88) * k;
  if (find_table(f->sfnt, f->sfnt_len, "post", &t, &len) && len >= 8)
    f->italic_angle = (int32_t)rd32(t + 4) / 65536.0;
  if (f->num_glyphs <= 0)
    f->num_glyphs = 65535;
  f->to_unicode = xcalloc((size_t)f->num_glyphs, sizeof(uint32_t));
  parse_cmap(f);
  read_psname(f, fc);
  f->ok = true;
  return f;
}

int native_advance(const native_face *f, int gid)
{
  if (!f->hmtx || f->num_hmetrics <= 0 || gid < 0)
    return 0;
  int i = gid < f->num_hmetrics ? gid : f->num_hmetrics - 1;
  return (int)rd16(f->hmtx + 4 * i);
}

/* ------------------------------------------------------------------ */
/* TFM                                                                 */

tfm_font *font_tfm(font_cache *fc, const char *name)
{
  fc_entry *e = lookup(fc, K_TFM, name, 0);
  if (e)
    return e->value;
  tfm_font *t = xcalloc(1, sizeof *t);
  tbuf *file = fc->res.load(fc->res.env, name, RES_TFM);
  insert(fc, K_TFM, name, 0, file, t);
  if (!file || file->len < 24)
    return t;
  const unsigned char *d = file->data;
  unsigned lf = rd16(d), lh = rd16(d + 2), bc = rd16(d + 4), ec = rd16(d + 6), nw = rd16(d + 8);
  if ((size_t)lf * 4 > file->len || lh < 2 || bc > ec + 1 || ec > 255)
    return t;
  size_t header = 24, ci = header + (size_t)lh * 4, wt = ci + (size_t)(ec - bc + 1) * 4;
  if (wt + (size_t)nw * 4 > file->len)
    return t;
  t->checksum = rd32(d + header);
  t->design = (int32_t)rd32(d + header + 4);
  t->bc = (int)bc;
  t->ec = (int)ec;
  for (unsigned c = bc; c <= ec; c++)
  {
    unsigned wi = d[ci + (c - bc) * 4];
    if (wi == 0 || wi >= nw)
      continue;
    t->width[c] = (int32_t)rd32(d + wt + (size_t)wi * 4);
    t->exists[c] = true;
  }
  t->ok = true;
  return t;
}

/* ------------------------------------------------------------------ */
/* Font map                                                            */

static char *next_token(char **p, bool *quoted)
{
  char *s = *p;
  while (*s == ' ' || *s == '\t')
    s++;
  if (!*s)
    return NULL;
  *quoted = false;
  char *start;
  if (*s == '"')
  {
    *quoted = true;
    start = ++s;
    while (*s && *s != '"')
      s++;
  }
  else
  {
    start = s;
    while (*s && *s != ' ' && *s != '\t')
      s++;
  }
  if (*s)
    *s++ = 0;
  *p = s;
  return start;
}

static bool ends_with(const char *s, const char *suffix)
{
  size_t a = strlen(s), b = strlen(suffix);
  return a >= b && strcasecmp(s + a - b, suffix) == 0;
}

static void load_map(font_cache *fc)
{
  fc->map_loaded = true;
  tbuf *file = fc->res.load(fc->res.env, "pdftex.map", RES_MAP);
  if (!file)
    return;
  char *text = malloc(file->len + 1);
  if (!text)
    abort();
  memcpy(text, file->data, file->len);
  text[file->len] = 0;
  tbuf_drop(file);
  for (char *line = text; line && *line;)
  {
    char *nl = strchr(line, '\n');
    if (nl)
      *nl = 0;
    char *p = line;
    while (*p == ' ' || *p == '\t')
      p++;
    if (*p && !strchr("%#*;", *p))
    {
      map_entry m = { 0 };
      m.extend = 1;
      bool q;
      char *tok = next_token(&p, &q);
      m.tfm = strdup(tok);
      while ((tok = next_token(&p, &q)))
      {
        if (q)
        {
          /* PostScript instructions: "<n> SlantFont", "<n> ExtendFont" */
          char *s;
          if ((s = strstr(tok, "SlantFont")))
          {
            *s = 0;
            char *num = strrchr(tok, ' ');
            m.slant = strtod(num ? num : tok, NULL);
            *s = 'S';
          }
          if ((s = strstr(tok, "ExtendFont")))
          {
            *s = 0;
            char *num = strrchr(tok, ' ');
            m.extend = strtod(num ? num : tok, NULL);
          }
          continue;
        }
        if (tok[0] == '<')
        {
          char *f = tok + 1;
          if (*f == '<' || *f == '[')
            f++;
          if (!*f)
          {
            f = next_token(&p, &q);
            if (!f)
              break;
          }
          if (ends_with(f, ".enc"))
          {
            free(m.encfile);
            m.encfile = strdup(f);
          }
          else
          {
            free(m.fontfile);
            m.fontfile = strdup(f);
          }
          continue;
        }
        if (!m.psname && !isdigit((unsigned char)tok[0]))
          m.psname = strdup(tok);
      }
      if (fc->nmap == fc->capmap)
      {
        fc->capmap = fc->capmap ? fc->capmap * 2 : 1024;
        fc->map = realloc(fc->map, sizeof(map_entry) * fc->capmap);
        if (!fc->map)
          abort();
      }
      fc->map[fc->nmap++] = m;
    }
    line = nl ? nl + 1 : NULL;
  }
  free(text);
}

const map_entry *font_map_lookup(font_cache *fc, const char *tfm)
{
  if (!fc->map_loaded)
    load_map(fc);
  for (int i = 0; i < fc->nmap; i++)
    if (strcmp(fc->map[i].tfm, tfm) == 0)
      return &fc->map[i];
  return NULL;
}

/* ------------------------------------------------------------------ */
/* Type 1                                                              */

static const char *memfind(const char *hay, size_t n, const char *needle)
{
  size_t k = strlen(needle);
  for (size_t i = 0; i + k <= n; i++)
    if (hay[i] == needle[0] && memcmp(hay + i, needle, k) == 0)
      return hay + i;
  return NULL;
}

static void parse_type1_header(type1_font *t)
{
  const char *s = (const char *)t->data;
  size_t n = t->len1;
  const char *p = memfind(s, n, "/FontBBox");
  if (p)
  {
    p += 9;
    const char *end = s + n;
    while (p < end && (*p == ' ' || *p == '{' || *p == '['))
      p++;
    for (int i = 0; i < 4 && p < end; i++)
    {
      char *q;
      t->bbox[i] = strtod(p, &q);
      if (q == p)
        break;
      p = q;
    }
  }
  p = memfind(s, n, "/FontName");
  if (p)
  {
    p += 9;
    while (p < s + n && (*p == ' ' || *p == '/'))
      p++;
    size_t k = 0;
    while (p < s + n && k < sizeof t->fontname - 1 && *p > 32 && *p < 127 && !strchr("/()<>[]{}%", *p))
      t->fontname[k++] = *p++;
    t->fontname[k] = 0;
  }
}

type1_font *font_type1(font_cache *fc, const char *file, pbuf *warnings)
{
  fc_entry *e = lookup(fc, K_TYPE1, file, 0);
  if (e)
    return e->value;
  type1_font *t = xcalloc(1, sizeof *t);
  tbuf *b = fc->res.load(fc->res.env, file, RES_TYPE1);
  insert(fc, K_TYPE1, file, 0, b, t);
  if (!b)
  {
    pbuf_printf(warnings, "Type 1 font not found: %s\n", file);
    return t;
  }
  const unsigned char *d = b->data;
  size_t n = b->len;
  pbuf clear = { 0 }, bin = { 0 }, trail = { 0 };
  if (n > 6 && d[0] == 0x80)
  {
    size_t pos = 0;
    while (pos + 2 <= n && d[pos] == 0x80)
    {
      int type = d[pos + 1];
      if (type == 3)
        break;
      if (pos + 6 > n)
        break;
      uint32_t l = (uint32_t)d[pos + 2] | (uint32_t)d[pos + 3] << 8 | (uint32_t)d[pos + 4] << 16 |
                   (uint32_t)d[pos + 5] << 24;
      pos += 6;
      if (l > n - pos)
        break;
      if (type == 1)
        pbuf_append(bin.len ? &trail : &clear, d + pos, l);
      else if (type == 2)
        pbuf_append(&bin, d + pos, l);
      pos += l;
    }
  }
  else
  {
    /* PFA: hex-encoded eexec section */
    const char *s = (const char *)d;
    const char *ex = memfind(s, n, "eexec");
    if (ex)
    {
      size_t cl = (size_t)(ex - s) + 5;
      while (cl < n && (s[cl] == '\r' || s[cl] == '\n'))
        cl++;
      pbuf_append(&clear, s, cl);
      size_t i = cl;
      int zeros = 0, hi = -1;
      size_t zstart = n;
      for (; i < n; i++)
      {
        int c = s[i];
        if (c == '0')
        {
          if (zeros++ == 0)
            zstart = i;
          if (zeros >= 64)
            break;
        }
        else if (!isspace(c))
          zeros = 0;
        int v = isdigit(c) ? c - '0' : (c >= 'a' && c <= 'f') ? c - 'a' + 10 : (c >= 'A' && c <= 'F') ? c - 'A' + 10 : -1;
        if (v < 0)
          continue;
        if (hi < 0)
          hi = v;
        else
        {
          pbuf_putc(&bin, hi * 16 + v);
          hi = -1;
        }
      }
      /* drop the zero hex digits we consumed at the end of binary */
      size_t zbytes = 0;
      for (size_t k = zstart; k < i; k++)
        zbytes += s[k] == '0';
      if (bin.len >= zbytes / 2)
        bin.len -= zbytes / 2;
      if (zstart < n)
        pbuf_append(&trail, s + zstart, n - zstart);
    }
  }
  if (!clear.len || !bin.len)
  {
    pbuf_printf(warnings, "unrecognized Type 1 font file: %s\n", file);
    pbuf_free(&clear);
    pbuf_free(&bin);
    pbuf_free(&trail);
    return t;
  }
  t->len1 = clear.len;
  t->len2 = bin.len;
  t->len3 = trail.len;
  t->data = malloc(t->len1 + t->len2 + t->len3 + 1);
  if (!t->data)
    abort();
  memcpy(t->data, clear.data, t->len1);
  memcpy(t->data + t->len1, bin.data, t->len2);
  if (t->len3)
    memcpy(t->data + t->len1 + t->len2, trail.data, t->len3);
  pbuf_free(&clear);
  pbuf_free(&bin);
  pbuf_free(&trail);
  t->bbox[0] = 0;
  t->bbox[1] = -250;
  t->bbox[2] = 1000;
  t->bbox[3] = 750;
  parse_type1_header(t);
  if (!t->fontname[0])
    snprintf(t->fontname, sizeof t->fontname, "PitexType1-%d", ++fc->anon_counter);
  t->ok = true;
  return t;
}

/* ------------------------------------------------------------------ */
/* Encoding vectors (.enc)                                             */

enc_vector *font_enc(font_cache *fc, const char *file)
{
  fc_entry *e = lookup(fc, K_ENC, file, 0);
  if (e)
    return e->value;
  enc_vector *v = xcalloc(1, sizeof *v);
  tbuf *b = fc->res.load(fc->res.env, file, RES_ENC);
  insert(fc, K_ENC, file, 0, NULL, v);
  if (!b)
    return v;
  const char *s = (const char *)b->data, *end = s + b->len;
  bool in_array = false;
  int idx = 0;
  while (s < end && idx < 256)
  {
    char c = *s;
    if (c == '%')
    {
      while (s < end && *s != '\n' && *s != '\r')
        s++;
      continue;
    }
    if (c == '[')
    {
      in_array = true;
      s++;
      continue;
    }
    if (c == ']')
      break;
    if (c == '/' && in_array)
    {
      const char *st = ++s;
      while (s < end && *s > 32 && !strchr("/[]{}()<>%", *s))
        s++;
      v->glyph[idx++] = strndup(st, (size_t)(s - st));
      continue;
    }
    if (c == '/' && !in_array)
    {
      /* encoding name before '[' */
      s++;
      while (s < end && *s > 32 && !strchr("/[]{}()<>%", *s))
        s++;
      continue;
    }
    s++;
  }
  tbuf_drop(b);
  v->ok = idx > 0;
  return v;
}

/* ------------------------------------------------------------------ */
/* Virtual fonts                                                       */

vf_font *font_vf(font_cache *fc, const char *name)
{
  fc_entry *e = lookup(fc, K_VF, name, 0);
  if (e)
    return e->value;
  tbuf *b = fc->res.load(fc->res.env, name, RES_VF);
  if (!b || b->len < 11 || b->data[0] != 247 || b->data[1] != 202)
  {
    tbuf_drop(b);
    insert(fc, K_VF, name, 0, NULL, NULL);
    return NULL;
  }
  vf_font *v = xcalloc(1, sizeof *v);
  insert(fc, K_VF, name, 0, b, v);
  const unsigned char *d = b->data, *end = d + b->len;
  const unsigned char *p = d + 3 + d[2] + 8;
  int capf = 0;
  while (p < end)
  {
    int op = *p;
    if (op >= 243 && op <= 246)
    {
      int k = op - 242;
      if (p + 1 + k + 14 > end)
        break;
      int32_t num = 0;
      for (int i = 0; i < k; i++)
        num = (num << 8) | p[1 + i];
      const unsigned char *q = p + 1 + k;
      int32_t s = (int32_t)rd32(q + 4);
      int a = q[12], l = q[13];
      if (q + 14 + a + l > end)
        break;
      if (v->nfonts == capf)
      {
        capf = capf ? capf * 2 : 4;
        v->fonts = realloc(v->fonts, sizeof(vf_fontdef) * capf);
        if (!v->fonts)
          abort();
      }
      v->fonts[v->nfonts].k = num;
      v->fonts[v->nfonts].scaled_fix = s;
      v->fonts[v->nfonts].name = strndup((const char *)q + 14 + a, (size_t)l);
      v->nfonts++;
      p = q + 14 + a + l;
    }
    else if (op < 242)
    {
      if (p + 5 + op > end)
        break;
      int cc = p[1];
      int32_t w = (int32_t)((uint32_t)p[2] << 16 | (uint32_t)p[3] << 8 | p[4]);
      if (w & 0x800000)
        w -= 0x1000000;
      v->chars[cc] = (vf_char){ p + 5, (uint32_t)op, w, true };
      p += 5 + op;
    }
    else if (op == 242)
    {
      if (p + 13 > end)
        break;
      uint32_t pl = rd32(p + 1), cc = rd32(p + 5);
      int32_t w = (int32_t)rd32(p + 9);
      if (pl > (size_t)(end - p - 13))
        break;
      if (cc < 256)
        v->chars[cc] = (vf_char){ p + 13, pl, w, true };
      p += 13 + pl;
    }
    else
      break; /* post (248) or garbage */
  }
  v->ok = true;
  return v;
}
