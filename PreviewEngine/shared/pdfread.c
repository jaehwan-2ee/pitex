/* Pitex embedded preview engine — minimal PDF reader.
 * Independently written from the PDF 1.7 / ISO 32000 specification.
 * Pitex-authored (PolyForm Shield 1.0.0, see repository LICENSE). */
#include "pdfread.h"

#include <ctype.h>
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAX_DEPTH 64
#define MAX_OBJECTS 8000000

/* ------------------------------------------------------------------ */
/* Arena                                                               */

typedef struct arena_chunk {
  struct arena_chunk *next;
  size_t used, cap;
  unsigned char data[];
} arena_chunk;

typedef struct {
  unsigned char type; /* 0 free / unknown, 1 offset, 2 compressed */
  unsigned char loading;
  size_t off;         /* type 1: byte offset; type 2: object stream number */
  int idx;            /* type 2: index within object stream */
  pr_obj *obj;
} xref_entry;

typedef struct {
  pr_obj *dict;
  pr_obj *resources, *mediabox, *cropbox;
  int rotate;
} page_entry;

struct pr_doc {
  const unsigned char *data;
  size_t len;
  arena_chunk *arena;
  xref_entry *xref;
  int nxref;
  pr_obj *trailer;
  page_entry *pages;
  int npages, cappages;
  bool pages_loaded;
  int resolve_budget;
};

static void *arena_alloc(pr_doc *d, size_t n)
{
  n = (n + 15) & ~(size_t)15;
  arena_chunk *c = d->arena;
  if (!c || c->used + n > c->cap)
  {
    size_t cap = n > 65536 ? n : 65536;
    c = malloc(sizeof(arena_chunk) + cap);
    if (!c)
      abort();
    c->next = d->arena;
    c->used = 0;
    c->cap = cap;
    d->arena = c;
  }
  void *p = c->data + c->used;
  c->used += n;
  memset(p, 0, n);
  return p;
}

static pr_obj *new_obj(pr_doc *d, pr_type t)
{
  pr_obj *o = arena_alloc(d, sizeof *o);
  o->type = t;
  return o;
}

static pr_obj null_obj = { .type = PR_NULL };

/* ------------------------------------------------------------------ */
/* Lexer                                                               */

typedef struct {
  const unsigned char *p;
  size_t pos, len;
} lexer;

static bool is_ws(int c) { return c == 0 || c == '\t' || c == '\n' || c == '\f' || c == '\r' || c == ' '; }
static bool is_delim(int c)
{
  return c == '(' || c == ')' || c == '<' || c == '>' || c == '[' || c == ']' || c == '{' || c == '}' ||
         c == '/' || c == '%';
}
static bool is_regular(int c) { return !is_ws(c) && !is_delim(c); }

static void skip_ws(lexer *lx)
{
  while (lx->pos < lx->len)
  {
    int c = lx->p[lx->pos];
    if (is_ws(c))
      lx->pos++;
    else if (c == '%')
    {
      while (lx->pos < lx->len && lx->p[lx->pos] != '\n' && lx->p[lx->pos] != '\r')
        lx->pos++;
    }
    else
      break;
  }
}

static bool at_keyword(lexer *lx, const char *kw)
{
  size_t n = strlen(kw);
  if (lx->pos + n > lx->len || memcmp(lx->p + lx->pos, kw, n) != 0)
    return false;
  if (lx->pos + n < lx->len && is_regular(lx->p[lx->pos + n]))
    return false;
  return true;
}

/* Parse an unsigned integer at the current position without consuming on
 * failure. */
static bool peek_uint(lexer *lx, size_t at, long long *out, size_t *end)
{
  size_t i = at;
  long long v = 0;
  if (i >= lx->len || !isdigit(lx->p[i]))
    return false;
  while (i < lx->len && isdigit(lx->p[i]))
  {
    v = v * 10 + (lx->p[i] - '0');
    if (v > (1LL << 40))
      return false;
    i++;
  }
  if (i < lx->len && is_regular(lx->p[i]))
    return false;
  *out = v;
  *end = i;
  return true;
}

static pr_obj *parse_obj(pr_doc *d, lexer *lx, int depth);

static pr_obj *parse_number(pr_doc *d, lexer *lx)
{
  size_t start = lx->pos;
  bool real = false;
  if (lx->pos < lx->len && (lx->p[lx->pos] == '+' || lx->p[lx->pos] == '-'))
    lx->pos++;
  while (lx->pos < lx->len && (isdigit(lx->p[lx->pos]) || lx->p[lx->pos] == '.'))
  {
    if (lx->p[lx->pos] == '.')
      real = true;
    lx->pos++;
  }
  /* tolerate garbage like "1.2.3" or "--1": consume regular chars */
  while (lx->pos < lx->len && is_regular(lx->p[lx->pos]))
    lx->pos++;
  char tmp[64];
  size_t n = lx->pos - start;
  if (n >= sizeof tmp)
    n = sizeof tmp - 1;
  memcpy(tmp, lx->p + start, n);
  tmp[n] = 0;
  if (real)
  {
    pr_obj *o = new_obj(d, PR_REAL);
    o->u.r = strtod(tmp, NULL);
    return o;
  }
  pr_obj *o = new_obj(d, PR_INT);
  o->u.i = strtoll(tmp, NULL, 10);
  return o;
}

static int hexval(int c)
{
  if (c >= '0' && c <= '9')
    return c - '0';
  if (c >= 'a' && c <= 'f')
    return c - 'a' + 10;
  if (c >= 'A' && c <= 'F')
    return c - 'A' + 10;
  return -1;
}

static pr_obj *parse_name(pr_doc *d, lexer *lx)
{
  lx->pos++; /* '/' */
  size_t start = lx->pos;
  while (lx->pos < lx->len && is_regular(lx->p[lx->pos]))
    lx->pos++;
  size_t n = lx->pos - start;
  pr_obj *o = new_obj(d, PR_NAME);
  char *s = arena_alloc(d, n + 1);
  size_t j = 0;
  for (size_t i = 0; i < n; i++)
  {
    int c = lx->p[start + i];
    if (c == '#' && i + 2 < n)
    {
      int h1 = hexval(lx->p[start + i + 1]);
      int h2 = hexval(lx->p[start + i + 2]);
      if (h1 >= 0 && h2 >= 0)
      {
        s[j++] = (char)(h1 * 16 + h2);
        i += 2;
        continue;
      }
    }
    s[j++] = (char)c;
  }
  s[j] = 0;
  o->u.str.s = s;
  o->u.str.len = j;
  return o;
}

static pr_obj *parse_literal_string(pr_doc *d, lexer *lx)
{
  lx->pos++; /* '(' */
  pbuf tmp = { 0 };
  int nest = 1;
  while (lx->pos < lx->len)
  {
    int c = lx->p[lx->pos++];
    if (c == '(')
    {
      nest++;
      pbuf_putc(&tmp, c);
    }
    else if (c == ')')
    {
      if (--nest == 0)
        break;
      pbuf_putc(&tmp, c);
    }
    else if (c == '\\' && lx->pos < lx->len)
    {
      int e = lx->p[lx->pos++];
      switch (e)
      {
        case 'n': pbuf_putc(&tmp, '\n'); break;
        case 'r': pbuf_putc(&tmp, '\r'); break;
        case 't': pbuf_putc(&tmp, '\t'); break;
        case 'b': pbuf_putc(&tmp, '\b'); break;
        case 'f': pbuf_putc(&tmp, '\f'); break;
        case '\r':
          if (lx->pos < lx->len && lx->p[lx->pos] == '\n')
            lx->pos++;
          break;
        case '\n': break;
        default:
          if (e >= '0' && e <= '7')
          {
            int v = e - '0';
            for (int k = 0; k < 2 && lx->pos < lx->len && lx->p[lx->pos] >= '0' && lx->p[lx->pos] <= '7'; k++)
              v = v * 8 + (lx->p[lx->pos++] - '0');
            pbuf_putc(&tmp, v & 0xff);
          }
          else
            pbuf_putc(&tmp, e);
      }
    }
    else
      pbuf_putc(&tmp, c);
  }
  pr_obj *o = new_obj(d, PR_STRING);
  o->u.str.s = arena_alloc(d, tmp.len + 1);
  if (tmp.len)
    memcpy(o->u.str.s, tmp.data, tmp.len);
  o->u.str.len = tmp.len;
  pbuf_free(&tmp);
  return o;
}

static pr_obj *parse_hex_string(pr_doc *d, lexer *lx)
{
  lx->pos++; /* '<' */
  pbuf tmp = { 0 };
  int hi = -1;
  while (lx->pos < lx->len)
  {
    int c = lx->p[lx->pos++];
    if (c == '>')
      break;
    int v = hexval(c);
    if (v < 0)
      continue;
    if (hi < 0)
      hi = v;
    else
    {
      pbuf_putc(&tmp, hi * 16 + v);
      hi = -1;
    }
  }
  if (hi >= 0)
    pbuf_putc(&tmp, hi * 16);
  pr_obj *o = new_obj(d, PR_STRING);
  o->u.str.s = arena_alloc(d, tmp.len + 1);
  if (tmp.len)
    memcpy(o->u.str.s, tmp.data, tmp.len);
  o->u.str.len = tmp.len;
  pbuf_free(&tmp);
  return o;
}

static pr_obj *parse_array(pr_doc *d, lexer *lx, int depth)
{
  lx->pos++; /* '[' */
  int cap = 8, n = 0;
  pr_obj **items = malloc(sizeof(pr_obj *) * cap);
  if (!items)
    abort();
  for (;;)
  {
    skip_ws(lx);
    if (lx->pos >= lx->len)
      break;
    if (lx->p[lx->pos] == ']')
    {
      lx->pos++;
      break;
    }
    size_t before = lx->pos;
    pr_obj *item = parse_obj(d, lx, depth + 1);
    if (lx->pos == before)
    {
      lx->pos++; /* unparseable byte: skip */
      continue;
    }
    if (n == cap)
    {
      cap *= 2;
      items = realloc(items, sizeof(pr_obj *) * cap);
      if (!items)
        abort();
    }
    items[n++] = item;
  }
  pr_obj *o = new_obj(d, PR_ARRAY);
  o->u.arr.items = arena_alloc(d, sizeof(pr_obj *) * (n ? n : 1));
  memcpy(o->u.arr.items, items, sizeof(pr_obj *) * n);
  o->u.arr.n = n;
  free(items);
  return o;
}

static pr_obj *parse_dict(pr_doc *d, lexer *lx, int depth)
{
  lx->pos += 2; /* '<<' */
  int cap = 8, n = 0;
  char **keys = malloc(sizeof(char *) * cap);
  pr_obj **vals = malloc(sizeof(pr_obj *) * cap);
  if (!keys || !vals)
    abort();
  for (;;)
  {
    skip_ws(lx);
    if (lx->pos >= lx->len)
      break;
    if (lx->p[lx->pos] == '>' && lx->pos + 1 < lx->len && lx->p[lx->pos + 1] == '>')
    {
      lx->pos += 2;
      break;
    }
    if (lx->p[lx->pos] != '/')
    {
      /* malformed: skip a token */
      size_t before = lx->pos;
      parse_obj(d, lx, depth + 1);
      if (lx->pos == before)
        lx->pos++;
      continue;
    }
    pr_obj *key = parse_name(d, lx);
    skip_ws(lx);
    pr_obj *val = &null_obj;
    if (lx->pos < lx->len && !(lx->p[lx->pos] == '>' && lx->pos + 1 < lx->len && lx->p[lx->pos + 1] == '>'))
      val = parse_obj(d, lx, depth + 1);
    if (n == cap)
    {
      cap *= 2;
      keys = realloc(keys, sizeof(char *) * cap);
      vals = realloc(vals, sizeof(pr_obj *) * cap);
      if (!keys || !vals)
        abort();
    }
    keys[n] = key->u.str.s;
    vals[n] = val;
    n++;
  }
  pr_obj *o = new_obj(d, PR_DICT);
  o->u.dict.keys = arena_alloc(d, sizeof(char *) * (n ? n : 1));
  o->u.dict.vals = arena_alloc(d, sizeof(pr_obj *) * (n ? n : 1));
  memcpy(o->u.dict.keys, keys, sizeof(char *) * n);
  memcpy(o->u.dict.vals, vals, sizeof(pr_obj *) * n);
  o->u.dict.n = n;
  free(keys);
  free(vals);
  return o;
}

static pr_obj *parse_obj(pr_doc *d, lexer *lx, int depth)
{
  skip_ws(lx);
  if (lx->pos >= lx->len || depth > MAX_DEPTH)
    return &null_obj;
  int c = lx->p[lx->pos];
  if (c == '/')
    return parse_name(d, lx);
  if (c == '(')
    return parse_literal_string(d, lx);
  if (c == '<')
  {
    if (lx->pos + 1 < lx->len && lx->p[lx->pos + 1] == '<')
      return parse_dict(d, lx, depth);
    return parse_hex_string(d, lx);
  }
  if (c == '[')
    return parse_array(d, lx, depth);
  if (isdigit(c))
  {
    /* maybe "num gen R" */
    long long num, gen;
    size_t e1, e2;
    if (peek_uint(lx, lx->pos, &num, &e1))
    {
      lexer t = *lx;
      t.pos = e1;
      skip_ws(&t);
      if (peek_uint(&t, t.pos, &gen, &e2))
      {
        t.pos = e2;
        skip_ws(&t);
        if (t.pos < t.len && t.p[t.pos] == 'R' && (t.pos + 1 >= t.len || !is_regular(t.p[t.pos + 1])))
        {
          pr_obj *o = new_obj(d, PR_REF);
          o->u.ref.num = (int)num;
          o->u.ref.gen = (int)gen;
          lx->pos = t.pos + 1;
          return o;
        }
      }
    }
    return parse_number(d, lx);
  }
  if (c == '+' || c == '-' || c == '.')
    return parse_number(d, lx);
  if (at_keyword(lx, "true") || at_keyword(lx, "false"))
  {
    pr_obj *o = new_obj(d, PR_BOOL);
    o->u.b = lx->p[lx->pos] == 't';
    lx->pos += o->u.b ? 4 : 5;
    return o;
  }
  if (at_keyword(lx, "null"))
  {
    lx->pos += 4;
    return &null_obj;
  }
  /* Unknown keyword: consume it so callers make progress. */
  while (lx->pos < lx->len && is_regular(lx->p[lx->pos]))
    lx->pos++;
  return &null_obj;
}

/* ------------------------------------------------------------------ */
/* Accessors                                                           */

pr_obj *pr_dict_raw(pr_obj *dict, const char *key)
{
  if (!dict)
    return NULL;
  if (dict->type == PR_STREAM)
    dict = dict->u.stream.dict;
  if (!dict || dict->type != PR_DICT)
    return NULL;
  for (int i = 0; i < dict->u.dict.n; i++)
    if (strcmp(dict->u.dict.keys[i], key) == 0)
      return dict->u.dict.vals[i];
  return NULL;
}

pr_obj *pr_resolve(pr_doc *d, pr_obj *o)
{
  for (int guard = 0; o && o->type == PR_REF && guard < 32; guard++)
    o = pr_load(d, o->u.ref.num);
  if (o && o->type == PR_REF)
    return NULL;
  return o;
}

pr_obj *pr_get(pr_doc *d, pr_obj *dict, const char *key)
{
  pr_obj *o = pr_resolve(d, pr_dict_raw(dict, key));
  if (o && o->type == PR_NULL)
    return NULL;
  return o;
}

bool pr_number(pr_obj *o, double *out)
{
  if (!o)
    return false;
  if (o->type == PR_INT)
  {
    *out = (double)o->u.i;
    return true;
  }
  if (o->type == PR_REAL)
  {
    *out = o->u.r;
    return true;
  }
  return false;
}

static bool name_is(pr_obj *o, const char *s)
{
  return o && o->type == PR_NAME && strcmp(o->u.str.s, s) == 0;
}

static long long get_int(pr_doc *d, pr_obj *dict, const char *key, long long dflt)
{
  double v;
  if (pr_number(pr_get(d, dict, key), &v))
    return (long long)v;
  return dflt;
}

/* ------------------------------------------------------------------ */
/* Indirect objects                                                    */

static bool ensure_xref(pr_doc *d, int num)
{
  if (num < 0 || num >= MAX_OBJECTS)
    return false;
  if (num < d->nxref)
    return true;
  int n = d->nxref ? d->nxref : 64;
  while (n <= num)
    n *= 2;
  if (n > MAX_OBJECTS)
    n = MAX_OBJECTS;
  xref_entry *x = realloc(d->xref, sizeof(xref_entry) * n);
  if (!x)
    abort();
  memset(x + d->nxref, 0, sizeof(xref_entry) * (n - d->nxref));
  d->xref = x;
  d->nxref = n;
  return true;
}

static void set_entry(pr_doc *d, int num, int type, size_t off, int idx, bool overwrite)
{
  if (!ensure_xref(d, num))
    return;
  xref_entry *e = &d->xref[num];
  if (e->type && !overwrite)
    return;
  e->type = (unsigned char)type;
  e->off = off;
  e->idx = idx;
}

/* Parse "num gen obj <object> [stream ... endstream]" at offset. */
static pr_obj *parse_indirect_at(pr_doc *d, size_t off, int expect_num)
{
  lexer lx = { d->data, off, d->len };
  long long num, gen;
  size_t e;
  skip_ws(&lx);
  if (!peek_uint(&lx, lx.pos, &num, &e))
    return NULL;
  lx.pos = e;
  skip_ws(&lx);
  if (!peek_uint(&lx, lx.pos, &gen, &e))
    return NULL;
  lx.pos = e;
  skip_ws(&lx);
  if (!at_keyword(&lx, "obj"))
    return NULL;
  if (expect_num >= 0 && num != expect_num)
    return NULL;
  lx.pos += 3;
  pr_obj *o = parse_obj(d, &lx, 0);
  skip_ws(&lx);
  if (o->type == PR_DICT && at_keyword(&lx, "stream"))
  {
    lx.pos += 6;
    if (lx.pos < lx.len && lx.p[lx.pos] == '\r')
      lx.pos++;
    if (lx.pos < lx.len && lx.p[lx.pos] == '\n')
      lx.pos++;
    size_t start = lx.pos;
    pr_obj *s = new_obj(d, PR_STREAM);
    s->u.stream.dict = o;
    s->u.stream.offset = start;
    /* /Length may be indirect; guard against recursion via loading flag */
    long long length = -1;
    pr_obj *lo = pr_dict_raw(o, "Length");
    if (lo && lo->type == PR_INT)
      length = lo->u.i;
    else if (lo && lo->type == PR_REF)
    {
      pr_obj *r = pr_resolve(d, lo);
      if (r && r->type == PR_INT)
        length = r->u.i;
    }
    bool ok = length >= 0 && (size_t)length <= d->len - start;
    if (ok)
    {
      lexer t = { d->data, start + (size_t)length, d->len };
      skip_ws(&t);
      ok = at_keyword(&t, "endstream");
    }
    if (!ok)
    {
      /* search for endstream */
      const unsigned char *hit = NULL;
      for (size_t i = start; i + 9 <= d->len; i++)
        if (d->data[i] == 'e' && memcmp(d->data + i, "endstream", 9) == 0)
        {
          hit = d->data + i;
          break;
        }
      if (!hit)
        return NULL;
      size_t end = (size_t)(hit - d->data);
      while (end > start && (d->data[end - 1] == '\n' || d->data[end - 1] == '\r'))
        end--;
      length = (long long)(end - start);
    }
    s->u.stream.length = (size_t)length;
    return s;
  }
  return o;
}

static pr_obj *load_from_objstm(pr_doc *d, int stmnum, int idx, int want);

pr_obj *pr_load(pr_doc *d, int num)
{
  if (num < 0 || num >= d->nxref)
    return NULL;
  xref_entry *e = &d->xref[num];
  if (e->obj)
    return e->obj;
  if (e->loading || d->resolve_budget <= 0)
    return NULL;
  d->resolve_budget--;
  e->loading = 1;
  pr_obj *o = NULL;
  if (e->type == 1 && e->off < d->len)
    o = parse_indirect_at(d, e->off, num);
  else if (e->type == 2)
    o = load_from_objstm(d, (int)e->off, e->idx, num);
  e->loading = 0;
  e->obj = o ? o : &null_obj;
  d->resolve_budget++;
  return e->obj;
}

static pr_obj *load_from_objstm(pr_doc *d, int stmnum, int idx, int want)
{
  pr_obj *stm = pr_load(d, stmnum);
  if (!stm || stm->type != PR_STREAM)
    return NULL;
  pbuf buf = { 0 };
  if (!pr_stream_decode(d, stm, &buf))
  {
    pbuf_free(&buf);
    return NULL;
  }
  long long n = get_int(d, stm, "N", 0), first = get_int(d, stm, "First", 0);
  pr_obj *result = NULL;
  /* The decoded buffer must outlive parsed strings; copy into arena. */
  unsigned char *copy = arena_alloc(d, buf.len + 1);
  memcpy(copy, buf.data, buf.len);
  size_t blen = buf.len;
  pbuf_free(&buf);
  lexer lx = { copy, 0, blen };
  for (long long i = 0; i < n && i < 1000000; i++)
  {
    long long onum, ooff;
    size_t e;
    skip_ws(&lx);
    if (!peek_uint(&lx, lx.pos, &onum, &e))
      break;
    lx.pos = e;
    skip_ws(&lx);
    if (!peek_uint(&lx, lx.pos, &ooff, &e))
      break;
    lx.pos = e;
    if (i == idx || onum == want)
    {
      if (first + ooff >= 0 && (size_t)(first + ooff) < blen)
      {
        lexer ol = { copy, (size_t)(first + ooff), blen };
        result = parse_obj(d, &ol, 0);
      }
      if (onum == want)
        break;
    }
  }
  return result;
}

/* ------------------------------------------------------------------ */
/* Filters                                                             */

static bool apply_predictor(pbuf *data, int predictor, int colors, int bpc, int columns)
{
  if (predictor < 10)
    return predictor == 1;
  size_t bpp = (size_t)((colors * bpc + 7) / 8);
  size_t row = (size_t)((colors * bpc * columns + 7) / 8);
  if (bpp == 0 || row == 0)
    return false;
  size_t nrows = data->len / (row + 1);
  unsigned char *out = malloc(nrows * row + 1);
  unsigned char *prev = calloc(row, 1);
  if (!out || !prev)
    abort();
  for (size_t r = 0; r < nrows; r++)
  {
    unsigned char *src = data->data + r * (row + 1);
    int type = src[0];
    src++;
    unsigned char *dst = out + r * row;
    for (size_t i = 0; i < row; i++)
    {
      int a = i >= bpp ? dst[i - bpp] : 0;
      int b = prev[i];
      int c = i >= bpp ? prev[i - bpp] : 0;
      int x = src[i];
      switch (type)
      {
        case 0: break;
        case 1: x += a; break;
        case 2: x += b; break;
        case 3: x += (a + b) / 2; break;
        case 4: {
          int p = a + b - c, pa = abs(p - a), pb = abs(p - b), pc = abs(p - c);
          x += (pa <= pb && pa <= pc) ? a : (pb <= pc ? b : c);
          break;
        }
        default: break;
      }
      dst[i] = (unsigned char)x;
    }
    memcpy(prev, dst, row);
  }
  free(prev);
  free(data->data);
  data->data = out;
  data->len = nrows * row;
  data->cap = nrows * row + 1;
  return true;
}

static bool decode_ahx(pbuf *out, const unsigned char *s, size_t n)
{
  int hi = -1;
  for (size_t i = 0; i < n; i++)
  {
    if (s[i] == '>')
      break;
    int v = hexval(s[i]);
    if (v < 0)
      continue;
    if (hi < 0)
      hi = v;
    else
    {
      pbuf_putc(out, hi * 16 + v);
      hi = -1;
    }
  }
  if (hi >= 0)
    pbuf_putc(out, hi * 16);
  return true;
}

static bool decode_a85(pbuf *out, const unsigned char *s, size_t n)
{
  uint32_t tuple = 0;
  int count = 0;
  size_t i = 0;
  if (n >= 2 && s[0] == '<' && s[1] == '~')
    i = 2;
  for (; i < n; i++)
  {
    int c = s[i];
    if (c == '~')
      break;
    if (is_ws(c))
      continue;
    if (c == 'z' && count == 0)
    {
      pbuf_append(out, "\0\0\0\0", 4);
      continue;
    }
    if (c < '!' || c > 'u')
      return false;
    tuple = tuple * 85 + (uint32_t)(c - '!');
    if (++count == 5)
    {
      unsigned char b[4] = { tuple >> 24, tuple >> 16, tuple >> 8, tuple };
      pbuf_append(out, b, 4);
      tuple = 0;
      count = 0;
    }
  }
  if (count > 1)
  {
    for (int k = count; k < 5; k++)
      tuple = tuple * 85 + 84;
    unsigned char b[4] = { tuple >> 24, tuple >> 16, tuple >> 8, tuple };
    pbuf_append(out, b, (size_t)(count - 1));
  }
  return true;
}

const unsigned char *pr_stream_raw(pr_doc *d, pr_obj *s, size_t *len)
{
  if (!s || s->type != PR_STREAM || s->u.stream.offset > d->len)
    return NULL;
  size_t n = s->u.stream.length;
  if (n > d->len - s->u.stream.offset)
    n = d->len - s->u.stream.offset;
  *len = n;
  return d->data + s->u.stream.offset;
}

bool pr_stream_decode(pr_doc *d, pr_obj *s, pbuf *out)
{
  size_t n;
  const unsigned char *raw = pr_stream_raw(d, s, &n);
  if (!raw)
    return false;
  pr_obj *filter = pr_get(d, s, "Filter");
  pr_obj *parms = pr_get(d, s, "DecodeParms");
  int nf = 0;
  pr_obj *filters[8];
  pr_obj *params[8];
  if (filter && filter->type == PR_NAME)
  {
    filters[0] = filter;
    params[0] = parms && parms->type == PR_ARRAY ? (parms->u.arr.n ? pr_resolve(d, parms->u.arr.items[0]) : NULL) : parms;
    nf = 1;
  }
  else if (filter && filter->type == PR_ARRAY)
  {
    for (int i = 0; i < filter->u.arr.n && nf < 8; i++)
    {
      filters[nf] = pr_resolve(d, filter->u.arr.items[i]);
      params[nf] = (parms && parms->type == PR_ARRAY && i < parms->u.arr.n) ? pr_resolve(d, parms->u.arr.items[i]) : NULL;
      nf++;
    }
  }
  pbuf cur = { 0 };
  pbuf_append(&cur, raw, n);
  for (int i = 0; i < nf; i++)
  {
    pbuf next = { 0 };
    bool ok;
    if (name_is(filters[i], "FlateDecode") || name_is(filters[i], "Fl"))
    {
      ok = pbuf_inflate(&next, cur.data, cur.len) == 0;
      pr_obj *p = params[i];
      if (ok && p && p->type == PR_DICT)
      {
        int pred = (int)get_int(d, p, "Predictor", 1);
        if (pred > 1)
          ok = apply_predictor(&next, pred, (int)get_int(d, p, "Colors", 1), (int)get_int(d, p, "BitsPerComponent", 8),
                               (int)get_int(d, p, "Columns", 1));
      }
    }
    else if (name_is(filters[i], "ASCIIHexDecode") || name_is(filters[i], "AHx"))
      ok = decode_ahx(&next, cur.data, cur.len);
    else if (name_is(filters[i], "ASCII85Decode") || name_is(filters[i], "A85"))
      ok = decode_a85(&next, cur.data, cur.len);
    else
      ok = false;
    pbuf_free(&cur);
    cur = next;
    if (!ok)
    {
      pbuf_free(&cur);
      return false;
    }
  }
  pbuf_append(out, cur.data, cur.len);
  pbuf_free(&cur);
  return true;
}

/* ------------------------------------------------------------------ */
/* Cross-reference loading                                             */

static bool load_xref_stream(pr_doc *d, pr_obj *s)
{
  if (!s || s->type != PR_STREAM)
    return false;
  pr_obj *w = pr_get(d, s, "W");
  if (!w || w->type != PR_ARRAY || w->u.arr.n < 3)
    return false;
  int W[3];
  for (int i = 0; i < 3; i++)
  {
    double v;
    if (!pr_number(pr_resolve(d, w->u.arr.items[i]), &v) || v < 0 || v > 8)
      return false;
    W[i] = (int)v;
  }
  pbuf buf = { 0 };
  if (!pr_stream_decode(d, s, &buf))
  {
    pbuf_free(&buf);
    return false;
  }
  long long size = get_int(d, s, "Size", 0);
  pr_obj *index = pr_get(d, s, "Index");
  long long ranges[2 * 64];
  int nr = 0;
  if (index && index->type == PR_ARRAY)
  {
    for (int i = 0; i + 1 < index->u.arr.n && nr < 64; i += 2)
    {
      double a, b;
      if (pr_number(pr_resolve(d, index->u.arr.items[i]), &a) && pr_number(pr_resolve(d, index->u.arr.items[i + 1]), &b))
      {
        ranges[2 * nr] = (long long)a;
        ranges[2 * nr + 1] = (long long)b;
        nr++;
      }
    }
  }
  else
  {
    ranges[0] = 0;
    ranges[1] = size;
    nr = 1;
  }
  size_t rowlen = (size_t)(W[0] + W[1] + W[2]);
  size_t pos = 0;
  for (int r = 0; r < nr; r++)
  {
    for (long long k = 0; k < ranges[2 * r + 1]; k++)
    {
      if (pos + rowlen > buf.len)
        break;
      unsigned long long f[3] = { 1, 0, 0 };
      if (W[0] == 0)
        f[0] = 1;
      for (int j = 0; j < 3; j++)
      {
        if (W[j] == 0)
          continue;
        unsigned long long v = 0;
        for (int b = 0; b < W[j]; b++)
          v = (v << 8) | buf.data[pos++];
        f[j] = v;
      }
      long long num = ranges[2 * r] + k;
      if (num < 0 || num >= MAX_OBJECTS)
        continue;
      if (f[0] == 1)
        set_entry(d, (int)num, 1, (size_t)f[1], 0, false);
      else if (f[0] == 2)
        set_entry(d, (int)num, 2, (size_t)f[1], (int)f[2], false);
      else if (f[0] == 0)
      {
        /* free entry: reserve slot so older sections don't resurrect it */
        if (ensure_xref(d, (int)num) && !d->xref[num].type)
          d->xref[num].type = 3;
      }
    }
  }
  pbuf_free(&buf);
  return true;
}

static bool load_xref_at(pr_doc *d, size_t off, int depth)
{
  if (depth > 32 || off >= d->len)
    return false;
  lexer lx = { d->data, off, d->len };
  skip_ws(&lx);
  pr_obj *trailer = NULL;
  if (at_keyword(&lx, "xref"))
  {
    lx.pos += 4;
    for (;;)
    {
      skip_ws(&lx);
      long long start, count;
      size_t e;
      if (!peek_uint(&lx, lx.pos, &start, &e))
        break;
      lx.pos = e;
      skip_ws(&lx);
      if (!peek_uint(&lx, lx.pos, &count, &e))
        return false;
      lx.pos = e;
      for (long long k = 0; k < count; k++)
      {
        long long o, g;
        skip_ws(&lx);
        if (!peek_uint(&lx, lx.pos, &o, &e))
          return false;
        lx.pos = e;
        skip_ws(&lx);
        if (!peek_uint(&lx, lx.pos, &g, &e))
          return false;
        lx.pos = e;
        skip_ws(&lx);
        if (lx.pos >= lx.len)
          return false;
        int kind = lx.p[lx.pos++];
        if (start + k >= MAX_OBJECTS)
          continue;
        if (kind == 'n')
          set_entry(d, (int)(start + k), 1, (size_t)o, 0, false);
        else if (ensure_xref(d, (int)(start + k)) && !d->xref[start + k].type)
          d->xref[start + k].type = 3;
      }
    }
    skip_ws(&lx);
    if (!at_keyword(&lx, "trailer"))
      return false;
    lx.pos += 7;
    trailer = parse_obj(d, &lx, 0);
    if (trailer->type != PR_DICT)
      return false;
    pr_obj *xs = pr_dict_raw(trailer, "XRefStm");
    if (xs && xs->type == PR_INT)
    {
      pr_obj *s = parse_indirect_at(d, (size_t)xs->u.i, -1);
      load_xref_stream(d, s);
    }
  }
  else
  {
    pr_obj *s = parse_indirect_at(d, lx.pos, -1);
    if (!s || s->type != PR_STREAM || !name_is(pr_dict_raw(s, "Type"), "XRef"))
      return false;
    if (!load_xref_stream(d, s))
      return false;
    trailer = s->u.stream.dict;
  }
  if (!d->trailer)
    d->trailer = trailer;
  pr_obj *prev = pr_dict_raw(trailer, "Prev");
  if (prev && prev->type == PR_INT && prev->u.i >= 0 && (size_t)prev->u.i != off)
    load_xref_at(d, (size_t)prev->u.i, depth + 1);
  return true;
}

/* Rebuild the table by scanning for "N G obj" when xref is damaged. */
static bool reconstruct(pr_doc *d)
{
  free(d->xref);
  d->xref = NULL;
  d->nxref = 0;
  d->trailer = NULL;
  for (size_t i = 0; i + 5 < d->len; i++)
  {
    if (d->data[i] != 'o' || memcmp(d->data + i, "obj", 3) != 0)
      continue;
    if (i + 3 < d->len && is_regular(d->data[i + 3]))
      continue;
    /* walk back over "num ws gen ws" */
    size_t j = i;
    while (j > 0 && is_ws(d->data[j - 1]))
      j--;
    size_t gend = j;
    while (j > 0 && isdigit(d->data[j - 1]))
      j--;
    if (j == gend)
      continue;
    while (j > 0 && is_ws(d->data[j - 1]))
      j--;
    size_t nend = j;
    while (j > 0 && isdigit(d->data[j - 1]))
      j--;
    if (j == nend || (j > 0 && is_regular(d->data[j - 1])))
      continue;
    long long num = strtoll((const char *)d->data + j, NULL, 10);
    if (num > 0 && num < MAX_OBJECTS)
      set_entry(d, (int)num, 1, j, 0, true);
  }
  /* trailer: last "trailer" dict, else an XRef stream dict, else Catalog */
  for (size_t i = d->len; i-- > 7;)
  {
    if (memcmp(d->data + i, "trailer", 7) == 0)
    {
      lexer lx = { d->data, i + 7, d->len };
      pr_obj *t = parse_obj(d, &lx, 0);
      if (t->type == PR_DICT && pr_dict_raw(t, "Root"))
      {
        d->trailer = t;
        break;
      }
    }
  }
  if (!d->trailer)
  {
    for (int n = 0; n < d->nxref; n++)
    {
      if (d->xref[n].type != 1)
        continue;
      pr_obj *o = pr_load(d, n);
      if (o && o->type == PR_DICT && name_is(pr_dict_raw(o, "Type"), "Catalog"))
      {
        pr_obj *t = new_obj(d, PR_DICT);
        t->u.dict.keys = arena_alloc(d, sizeof(char *));
        t->u.dict.vals = arena_alloc(d, sizeof(pr_obj *));
        t->u.dict.keys[0] = "Root";
        pr_obj *ref = new_obj(d, PR_REF);
        ref->u.ref.num = n;
        t->u.dict.vals[0] = ref;
        t->u.dict.n = 1;
        d->trailer = t;
        break;
      }
      if (o && o->type == PR_STREAM && name_is(pr_dict_raw(o, "Type"), "XRef") && pr_dict_raw(o, "Root"))
      {
        d->trailer = o->u.stream.dict;
        load_xref_stream(d, o); /* object streams need type-2 entries */
        break;
      }
    }
  }
  return d->trailer != NULL;
}

pr_doc *pr_open(const unsigned char *data, size_t len)
{
  if (!data || len < 8)
    return NULL;
  pr_doc *d = calloc(1, sizeof *d);
  if (!d)
    abort();
  d->data = data;
  d->len = len;
  d->resolve_budget = 256;
  /* startxref within the last 4 KiB */
  bool ok = false;
  size_t from = len > 4096 ? len - 4096 : 0;
  for (size_t i = len - 9; i-- > from;)
  {
    if (memcmp(data + i, "startxref", 9) == 0)
    {
      lexer lx = { data, i + 9, len };
      skip_ws(&lx);
      long long off;
      size_t e;
      if (peek_uint(&lx, lx.pos, &off, &e))
        ok = load_xref_at(d, (size_t)off, 0);
      break;
    }
  }
  if (ok && !pr_get(d, d->trailer, "Root"))
    ok = false;
  if (!ok && !reconstruct(d))
  {
    pr_close(d);
    return NULL;
  }
  return d;
}

void pr_close(pr_doc *d)
{
  if (!d)
    return;
  for (arena_chunk *c = d->arena; c;)
  {
    arena_chunk *n = c->next;
    free(c);
    c = n;
  }
  free(d->xref);
  free(d->pages);
  free(d);
}

bool pr_is_encrypted(pr_doc *d) { return pr_dict_raw(d->trailer, "Encrypt") != NULL; }

/* ------------------------------------------------------------------ */
/* Page tree                                                           */

static void collect_pages(pr_doc *d, pr_obj *node, page_entry inherit, int depth)
{
  if (!node || node->type != PR_DICT || depth > 64 || d->npages > 1000000)
    return;
  pr_obj *v;
  if ((v = pr_get(d, node, "Resources")))
    inherit.resources = v;
  if ((v = pr_get(d, node, "MediaBox")))
    inherit.mediabox = v;
  if ((v = pr_get(d, node, "CropBox")))
    inherit.cropbox = v;
  double rot;
  if (pr_number(pr_get(d, node, "Rotate"), &rot))
    inherit.rotate = (int)rot;
  pr_obj *kids = pr_get(d, node, "Kids");
  pr_obj *type = pr_get(d, node, "Type");
  if (kids && kids->type == PR_ARRAY && !name_is(type, "Page"))
  {
    for (int i = 0; i < kids->u.arr.n; i++)
    {
      pr_obj *k = kids->u.arr.items[i];
      /* loop protection: a kid referring back to an ancestor is caught by
       * the depth limit; mark visited nodes via the loading flag */
      if (k->type == PR_REF && k->u.ref.num >= 0 && k->u.ref.num < d->nxref)
      {
        xref_entry *e = &d->xref[k->u.ref.num];
        if (e->loading)
          continue;
        pr_obj *kd = pr_resolve(d, k);
        e->loading = 1;
        collect_pages(d, kd, inherit, depth + 1);
        e->loading = 0;
      }
      else
        collect_pages(d, pr_resolve(d, k), inherit, depth + 1);
    }
    return;
  }
  if (d->npages == d->cappages)
  {
    d->cappages = d->cappages ? d->cappages * 2 : 16;
    d->pages = realloc(d->pages, sizeof(page_entry) * d->cappages);
    if (!d->pages)
      abort();
  }
  inherit.dict = node;
  d->pages[d->npages++] = inherit;
}

static void load_pages(pr_doc *d)
{
  if (d->pages_loaded)
    return;
  d->pages_loaded = true;
  pr_obj *root = pr_get(d, d->trailer, "Root");
  pr_obj *pages = pr_get(d, root, "Pages");
  page_entry base = { 0 };
  collect_pages(d, pages, base, 0);
}

int pr_page_count(pr_doc *d)
{
  load_pages(d);
  return d->npages;
}

int pr_normalize_page(pr_doc *d, int page)
{
  int pages = pr_page_count(d);
  if (page > pages)
    page = pages;
  if (page < 0)
    page = pages + 1 + page;
  if (page < 1)
    page = 1;
  return page;
}

static bool read_rect(pr_doc *d, pr_obj *a, double r[4])
{
  a = pr_resolve(d, a);
  if (!a || a->type != PR_ARRAY || a->u.arr.n < 4)
    return false;
  for (int i = 0; i < 4; i++)
    if (!pr_number(pr_resolve(d, a->u.arr.items[i]), &r[i]))
      return false;
  double x0 = fmin(r[0], r[2]), x1 = fmax(r[0], r[2]);
  double y0 = fmin(r[1], r[3]), y1 = fmax(r[1], r[3]);
  r[0] = x0;
  r[1] = y0;
  r[2] = x1;
  r[3] = y1;
  return true;
}

static void intersect(double a[4], const double b[4])
{
  a[0] = fmax(a[0], b[0]);
  a[1] = fmax(a[1], b[1]);
  a[2] = fmin(a[2], b[2]);
  a[3] = fmin(a[3], b[3]);
  if (a[2] < a[0])
    a[2] = a[0];
  if (a[3] < a[1])
    a[3] = a[1];
}

bool pr_page(pr_doc *d, int index0, int kind, pr_page_info *out)
{
  load_pages(d);
  if (index0 < 0 || index0 >= d->npages)
    return false;
  page_entry *p = &d->pages[index0];
  double media[4] = { 0, 0, 612, 792 }, crop[4], box[4];
  read_rect(d, p->mediabox, media);
  memcpy(crop, media, sizeof crop);
  if (read_rect(d, p->cropbox, crop))
    intersect(crop, media);
  memcpy(box, crop, sizeof box);
  const char *key = NULL;
  switch (kind)
  {
    case PR_BOX_MEDIA: memcpy(box, media, sizeof box); break;
    case PR_BOX_BLEED: key = "BleedBox"; break;
    case PR_BOX_TRIM: key = "TrimBox"; break;
    case PR_BOX_ART: key = "ArtBox"; break;
    default: break;
  }
  if (key && read_rect(d, pr_get(d, p->dict, key), box))
    intersect(box, media);
  memcpy(out->box, box, sizeof box);
  int r = p->rotate % 360;
  if (r < 0)
    r += 360;
  out->rotate = (r / 90) * 90;
  out->page = p->dict;
  out->resources = p->resources;
  return true;
}

void pr_rotation_matrix(int rotate, double m[6])
{
  /* Displayed page is rotated clockwise by `rotate` degrees. */
  double a = 1, b = 0, c = 0, dd = 1;
  switch (((rotate % 360) + 360) % 360)
  {
    case 90: a = 0; b = -1; c = 1; dd = 0; break;
    case 180: a = -1; b = 0; c = 0; dd = -1; break;
    case 270: a = 0; b = 1; c = -1; dd = 0; break;
    default: break;
  }
  m[0] = a;
  m[1] = b;
  m[2] = c;
  m[3] = dd;
  m[4] = 0;
  m[5] = 0;
}

void pr_transform_box(const double m[6], const double box[4], double out[4])
{
  double xs[4] = { box[0], box[2], box[2], box[0] };
  double ys[4] = { box[1], box[1], box[3], box[3] };
  double x0 = INFINITY, y0 = INFINITY, x1 = -INFINITY, y1 = -INFINITY;
  for (int i = 0; i < 4; i++)
  {
    double x = m[0] * xs[i] + m[2] * ys[i] + m[4];
    double y = m[1] * xs[i] + m[3] * ys[i] + m[5];
    x0 = fmin(x0, x);
    y0 = fmin(y0, y);
    x1 = fmax(x1, x);
    y1 = fmax(y1, y);
  }
  out[0] = x0;
  out[1] = y0;
  out[2] = x1;
  out[3] = y1;
}
