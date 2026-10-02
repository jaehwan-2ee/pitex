/* Pitex embedded preview engine — DVI/XDV stream index.
 * Independently written from the DVI format and XeTeX XDV extensions.
 * Pitex-authored (PolyForm Shield 1.0.0). */
#include "xdv.h"

#include <stdlib.h>
#include <string.h>

static uint32_t be(const unsigned char *p, int n)
{
  uint32_t v = 0;
  for (int i = 0; i < n; i++)
    v = (v << 8) | p[i];
  return v;
}

long xdv_insn_length(const unsigned char *p, const unsigned char *end)
{
  long avail = (long)(end - p);
  if (avail < 1)
    return 0;
  int op = p[0];
#define NEED(n) do { if (avail < (long)(n)) return 0; } while (0)
  if (op < 128 || (op >= DVI_FNT_NUM_0 && op < DVI_FNT1) || op == DVI_NOP || op == DVI_PUSH || op == DVI_POP ||
      op == DVI_EOP || op == DVI_W0 || op == DVI_X0 || op == DVI_Y0 || op == DVI_Z0)
    return 1;
  if (op >= DVI_SET1 && op < DVI_SET1 + 4)
    return 1 + (op - DVI_SET1 + 1);
  if (op >= DVI_PUT1 && op < DVI_PUT1 + 4)
    return 1 + (op - DVI_PUT1 + 1);
  if (op == DVI_SET_RULE || op == DVI_PUT_RULE)
    return 9;
  if (op == DVI_BOP)
    return 45;
  if (op >= DVI_RIGHT1 && op < DVI_RIGHT1 + 4)
    return 1 + (op - DVI_RIGHT1 + 1);
  if (op >= DVI_W1 && op < DVI_W1 + 4)
    return 1 + (op - DVI_W1 + 1);
  if (op >= DVI_X1 && op < DVI_X1 + 4)
    return 1 + (op - DVI_X1 + 1);
  if (op >= DVI_DOWN1 && op < DVI_DOWN1 + 4)
    return 1 + (op - DVI_DOWN1 + 1);
  if (op >= DVI_Y1 && op < DVI_Y1 + 4)
    return 1 + (op - DVI_Y1 + 1);
  if (op >= DVI_Z1 && op < DVI_Z1 + 4)
    return 1 + (op - DVI_Z1 + 1);
  if (op >= DVI_FNT1 && op < DVI_FNT1 + 4)
    return 1 + (op - DVI_FNT1 + 1);
  if (op >= DVI_XXX1 && op < DVI_XXX1 + 4)
  {
    int k = op - DVI_XXX1 + 1;
    NEED(1 + k);
    uint32_t len = be(p + 1, k);
    if (len > (1u << 30))
      return -1;
    NEED(1 + k + (long)len);
    return 1 + k + (long)len;
  }
  if (op >= DVI_FNT_DEF1 && op < DVI_FNT_DEF1 + 4)
  {
    int k = op - DVI_FNT_DEF1 + 1;
    NEED(1 + k + 14);
    int a = p[1 + k + 12], l = p[1 + k + 13];
    NEED(1 + k + 14 + a + l);
    return 1 + k + 14 + a + l;
  }
  if (op == DVI_PRE)
  {
    NEED(15);
    NEED(15 + p[14]);
    return 15 + p[14];
  }
  if (op == XDV_NATIVE_FONT_DEF)
  {
    /* k[4] size[4] flags[2] lenps[1] name[lenps] index[4] ... */
    NEED(12);
    uint32_t flags = be(p + 9, 2);
    int namelen = p[11];
    long len = 12 + namelen + 4;
    if (flags & 0x0200)
      len += 4; /* colored */
    if (flags & 0x1000)
      len += 4; /* extend */
    if (flags & 0x2000)
      len += 4; /* slant */
    if (flags & 0x4000)
      len += 4; /* embolden */
    NEED(len);
    return len;
  }
  if (op == XDV_GLYPHS)
  {
    NEED(7);
    uint32_t n = be(p + 5, 2);
    NEED(7 + 10 * (long)n);
    return 7 + 10 * (long)n;
  }
  if (op == XDV_TEXT_AND_GLYPHS)
  {
    NEED(3);
    uint32_t l = be(p + 1, 2);
    NEED(3 + 2 * l + 6);
    uint32_t n = be(p + 3 + 2 * l + 4, 2);
    NEED(3 + 2 * (long)l + 6 + 10 * (long)n);
    return 3 + 2 * (long)l + 6 + 10 * (long)n;
  }
#undef NEED
  return -1;
}

struct xdv_index {
  bool have_pre;
  uint32_t num, den, mag;
  size_t scan; /* offset up to which instructions have been parsed */
  bool in_page;
  size_t cur_bop;
  xdv_page *pages;
  int npages, cap;
  bool broken;
};

xdv_index *xdv_index_new(void)
{
  xdv_index *x = calloc(1, sizeof *x);
  if (!x)
    abort();
  return x;
}

void xdv_index_free(xdv_index *x)
{
  if (!x)
    return;
  free(x->pages);
  free(x);
}

void xdv_index_reset(xdv_index *x)
{
  xdv_page *pages = x->pages;
  int cap = x->cap;
  memset(x, 0, sizeof *x);
  x->pages = pages;
  x->cap = cap;
}

void xdv_index_update(xdv_index *x, const unsigned char *data, size_t len)
{
  if (len < x->scan)
  {
    /* Truncated by a rollback: drop pages that are no longer complete and
     * resume scanning after the last complete page. */
    while (x->npages > 0 && x->pages[x->npages - 1].eop_end > len)
      x->npages--;
    x->in_page = false;
    x->broken = false;
    if (x->npages > 0)
      x->scan = x->pages[x->npages - 1].eop_end;
    else
      xdv_index_reset(x);
  }
  if (x->broken)
    return;
  const unsigned char *end = data + len;
  while (x->scan < len)
  {
    const unsigned char *p = data + x->scan;
    long n = xdv_insn_length(p, end);
    if (n == 0)
      break;
    if (n < 0)
    {
      x->broken = true;
      break;
    }
    int op = p[0];
    if (!x->have_pre)
    {
      if (op != DVI_PRE)
      {
        x->broken = true;
        break;
      }
      x->num = be(p + 2, 4);
      x->den = be(p + 6, 4);
      x->mag = be(p + 10, 4);
      x->have_pre = true;
    }
    else if (op == DVI_BOP)
    {
      x->in_page = true;
      x->cur_bop = x->scan;
    }
    else if (op == DVI_EOP && x->in_page)
    {
      if (x->npages == x->cap)
      {
        x->cap = x->cap ? x->cap * 2 : 64;
        x->pages = realloc(x->pages, sizeof(xdv_page) * x->cap);
        if (!x->pages)
          abort();
      }
      x->pages[x->npages].bop = x->cur_bop;
      x->pages[x->npages].eop_end = x->scan + 1;
      x->npages++;
      x->in_page = false;
    }
    else if (op == DVI_POST)
    {
      /* Postamble: nothing more to index. */
      x->scan = len;
      break;
    }
    x->scan += (size_t)n;
  }
}

int xdv_index_page_count(const xdv_index *x) { return x->npages; }

const xdv_page *xdv_index_page(const xdv_index *x, int i)
{
  if (i < 0 || i >= x->npages)
    return NULL;
  return &x->pages[i];
}

bool xdv_index_output_started(const xdv_index *x) { return x->have_pre; }

bool xdv_index_preamble(const xdv_index *x, uint32_t *num, uint32_t *den, uint32_t *mag)
{
  if (!x->have_pre)
    return false;
  *num = x->num;
  *den = x->den;
  *mag = x->mag;
  return true;
}
