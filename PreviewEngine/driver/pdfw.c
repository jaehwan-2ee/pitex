/* Pitex embedded preview engine — PDF object writer.
 * Written from the PDF 1.7 specification. Pitex-authored (PolyForm Shield). */
#include "pdfw.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct pdfw {
  pbuf *out;
  size_t *offsets;
  int count, cap;
};

pdfw *pdfw_new(pbuf *out)
{
  pdfw *w = calloc(1, sizeof *w);
  if (!w)
    abort();
  w->out = out;
  w->count = 1; /* object 0 is the free list head */
  pbuf_puts(out, "%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
  return w;
}

void pdfw_free(pdfw *w)
{
  if (!w)
    return;
  free(w->offsets);
  free(w);
}

pbuf *pdfw_out(pdfw *w) { return w->out; }

int pdfw_alloc(pdfw *w)
{
  if (w->count >= w->cap)
  {
    w->cap = w->cap ? w->cap * 2 : 256;
    w->offsets = realloc(w->offsets, sizeof(size_t) * w->cap);
    if (!w->offsets)
      abort();
  }
  w->offsets[w->count] = 0;
  return w->count++;
}

void pdfw_begin(pdfw *w, int num)
{
  w->offsets[num] = w->out->len;
  pbuf_printf(w->out, "%d 0 obj\n", num);
}

void pdfw_end(pdfw *w) { pbuf_puts(w->out, "\nendobj\n"); }

void pdfw_stream_deflated(pdfw *w, int num, const char *dict_body, const void *data, size_t len)
{
  pdfw_begin(w, num);
  pbuf_printf(w->out, "<<%s/Filter/FlateDecode/Length %zu>>\nstream\n", dict_body ? dict_body : "", len);
  pbuf_append(w->out, data, len);
  pbuf_puts(w->out, "\nendstream");
  pdfw_end(w);
}

void pdfw_stream(pdfw *w, int num, const char *dict_body, const void *data, size_t len, bool compress)
{
  if (compress && len > 64)
  {
    pbuf z = { 0 };
    if (pbuf_deflate(&z, data, len, 3) == 0)
    {
      pdfw_stream_deflated(w, num, dict_body, z.data, z.len);
      pbuf_free(&z);
      return;
    }
    pbuf_free(&z);
  }
  pdfw_begin(w, num);
  pbuf_printf(w->out, "<<%s/Length %zu>>\nstream\n", dict_body ? dict_body : "", len);
  pbuf_append(w->out, data, len);
  pbuf_puts(w->out, "\nendstream");
  pdfw_end(w);
}

void pdfw_finish(pdfw *w, int root, int info)
{
  size_t xref = w->out->len;
  pbuf_printf(w->out, "xref\n0 %d\n0000000000 65535 f \n", w->count);
  for (int i = 1; i < w->count; i++)
  {
    if (w->offsets[i])
      pbuf_printf(w->out, "%010zu 00000 n \n", w->offsets[i]);
    else
      pbuf_puts(w->out, "0000000000 00000 f \n");
  }
  pbuf_printf(w->out, "trailer\n<</Size %d/Root %d 0 R", w->count, root);
  if (info > 0)
    pbuf_printf(w->out, "/Info %d 0 R", info);
  pbuf_printf(w->out, ">>\nstartxref\n%zu\n%%%%EOF\n", xref);
}

void pdfw_name(pbuf *b, const char *name)
{
  pbuf_putc(b, '/');
  for (const unsigned char *p = (const unsigned char *)name; *p; p++)
  {
    if (*p < 0x21 || *p > 0x7e || strchr("#()<>[]{}/%", *p))
      pbuf_printf(b, "#%02X", *p);
    else
      pbuf_putc(b, *p);
  }
}

void pdfw_hexstring(pbuf *b, const unsigned char *s, size_t len)
{
  static const char hex[] = "0123456789ABCDEF";
  pbuf_putc(b, '<');
  for (size_t i = 0; i < len; i++)
  {
    pbuf_putc(b, hex[s[i] >> 4]);
    pbuf_putc(b, hex[s[i] & 15]);
  }
  pbuf_putc(b, '>');
}

/* ------------------------------------------------------------------ */
/* Import                                                              */

struct pdfw_import {
  pdfw *w;
  pr_doc *src;
  int *map;      /* source object number -> output number (0: none) */
  int nmap;
  int *queue;    /* source numbers waiting to be written */
  int nq, capq;
  int written;
};

pdfw_import *pdfw_import_begin(pdfw *w, pr_doc *src)
{
  pdfw_import *imp = calloc(1, sizeof *imp);
  if (!imp)
    abort();
  imp->w = w;
  imp->src = src;
  return imp;
}

static int map_ref(pdfw_import *imp, int num)
{
  if (num < 0 || num > 8000000)
    return 0;
  if (num >= imp->nmap)
  {
    int n = imp->nmap ? imp->nmap : 64;
    while (n <= num)
      n *= 2;
    imp->map = realloc(imp->map, sizeof(int) * n);
    if (!imp->map)
      abort();
    memset(imp->map + imp->nmap, 0, sizeof(int) * (n - imp->nmap));
    imp->nmap = n;
  }
  if (!imp->map[num])
  {
    imp->map[num] = pdfw_alloc(imp->w);
    if (imp->nq == imp->capq)
    {
      imp->capq = imp->capq ? imp->capq * 2 : 64;
      imp->queue = realloc(imp->queue, sizeof(int) * imp->capq);
      if (!imp->queue)
        abort();
    }
    imp->queue[imp->nq++] = num;
  }
  return imp->map[num];
}

static bool skip_key(const char *k)
{
  return !strcmp(k, "Parent") || !strcmp(k, "PieceInfo") || !strcmp(k, "Metadata") || !strcmp(k, "Thumb") ||
         !strcmp(k, "StructParent") || !strcmp(k, "StructParents") || !strcmp(k, "Annots") || !strcmp(k, "B");
}

static void import_dict_body(pdfw_import *imp, pbuf *b, pr_obj *d, bool stream)
{
  for (int i = 0; i < d->u.dict.n; i++)
  {
    const char *k = d->u.dict.keys[i];
    if (skip_key(k))
      continue;
    if (stream && !strcmp(k, "Length"))
      continue;
    pdfw_name(b, k);
    pbuf_putc(b, ' ');
    pdfw_import_value(imp, b, d->u.dict.vals[i]);
  }
}

void pdfw_import_value(pdfw_import *imp, pbuf *b, pr_obj *o)
{
  if (!o)
  {
    pbuf_puts(b, "null");
    return;
  }
  switch (o->type)
  {
    case PR_NULL: pbuf_puts(b, "null"); break;
    case PR_BOOL: pbuf_puts(b, o->u.b ? "true" : "false"); break;
    case PR_INT: pbuf_printf(b, "%lld", o->u.i); break;
    case PR_REAL: pbuf_real(b, o->u.r); break;
    case PR_NAME: pdfw_name(b, o->u.str.s); break;
    case PR_STRING: pdfw_hexstring(b, (unsigned char *)o->u.str.s, o->u.str.len); break;
    case PR_ARRAY:
      pbuf_putc(b, '[');
      for (int i = 0; i < o->u.arr.n; i++)
      {
        if (i)
          pbuf_putc(b, ' ');
        pdfw_import_value(imp, b, o->u.arr.items[i]);
      }
      pbuf_putc(b, ']');
      break;
    case PR_DICT:
      pbuf_puts(b, "<<");
      import_dict_body(imp, b, o, false);
      pbuf_puts(b, ">>");
      break;
    case PR_REF:
    {
      pr_obj *t = pr_load(imp->src, o->u.ref.num);
      if (!t || t->type == PR_NULL)
      {
        pbuf_puts(b, "null");
        break;
      }
      pbuf_printf(b, "%d 0 R", map_ref(imp, o->u.ref.num));
      break;
    }
    case PR_STREAM:
      /* Direct streams cannot occur in valid PDF; write the dictionary. */
      pbuf_puts(b, "null");
      break;
  }
}

void pdfw_import_flush(pdfw_import *imp)
{
  while (imp->written < imp->nq)
  {
    int src = imp->queue[imp->written++];
    int dst = imp->map[src];
    pr_obj *o = pr_load(imp->src, src);
    pbuf body = { 0 };
    if (o && o->type == PR_STREAM)
    {
      size_t len = 0;
      const unsigned char *raw = pr_stream_raw(imp->src, o, &len);
      import_dict_body(imp, &body, o->u.stream.dict, true);
      pbuf_putc(&body, 0);
      pdfw_begin(imp->w, dst);
      pbuf_printf(imp->w->out, "<<%s/Length %zu>>\nstream\n", (char *)body.data, len);
      pbuf_append(imp->w->out, raw, len);
      pbuf_puts(imp->w->out, "\nendstream");
      pdfw_end(imp->w);
    }
    else
    {
      pdfw_import_value(imp, &body, o);
      pdfw_begin(imp->w, dst);
      pbuf_append(imp->w->out, body.data, body.len);
      pdfw_end(imp->w);
    }
    pbuf_free(&body);
  }
}

void pdfw_import_end(pdfw_import *imp)
{
  if (!imp)
    return;
  pdfw_import_flush(imp);
  free(imp->map);
  free(imp->queue);
  free(imp);
}
