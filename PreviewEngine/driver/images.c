/* Pitex embedded preview engine — raster images to PDF image XObjects.
 * Independently written from the PNG, JPEG/JFIF and BMP specifications.
 * Pitex-authored (PolyForm Shield 1.0.0). */
#include "images.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "imginfo.h"

static uint32_t be32(const unsigned char *p) { return (uint32_t)p[0] << 24 | (uint32_t)p[1] << 16 | (uint32_t)p[2] << 8 | p[3]; }
static unsigned be16(const unsigned char *p) { return (unsigned)p[0] << 8 | p[1]; }
static uint32_t le32(const unsigned char *p) { return (uint32_t)p[3] << 24 | (uint32_t)p[2] << 16 | (uint32_t)p[1] << 8 | p[0]; }
static unsigned le16(const unsigned char *p) { return (unsigned)p[1] << 8 | p[0]; }

void image_free(pdf_image *img)
{
  pbuf_free(&img->data);
  pbuf_free(&img->smask);
  memset(img, 0, sizeof *img);
}

/* ------------------------------------------------------------------ */
/* PNG                                                                 */

static int paeth(int a, int b, int c)
{
  int p = a + b - c, pa = abs(p - a), pb = abs(p - b), pc = abs(p - c);
  return (pa <= pb && pa <= pc) ? a : (pb <= pc ? b : c);
}

/* Unfilter `h` rows of `stride` bytes starting at raw[*pos]. */
static bool unfilter(unsigned char *raw, size_t rawlen, size_t *pos, size_t stride, int h, int bpp, unsigned char *out)
{
  unsigned char *prev = NULL;
  for (int y = 0; y < h; y++)
  {
    if (*pos + 1 + stride > rawlen)
      return false;
    int type = raw[(*pos)++];
    unsigned char *src = raw + *pos, *dst = out + (size_t)y * stride;
    for (size_t i = 0; i < stride; i++)
    {
      int a = i >= (size_t)bpp ? dst[i - bpp] : 0;
      int b = prev ? prev[i] : 0;
      int c = (prev && i >= (size_t)bpp) ? prev[i - bpp] : 0;
      int x = src[i];
      switch (type)
      {
        case 1: x += a; break;
        case 2: x += b; break;
        case 3: x += (a + b) >> 1; break;
        case 4: x += paeth(a, b, c); break;
        default: break;
      }
      dst[i] = (unsigned char)x;
    }
    *pos += stride;
    prev = dst;
  }
  return true;
}

static unsigned sample(const unsigned char *row, int idx, int bd)
{
  if (bd == 8)
    return row[idx];
  if (bd == 16)
    return row[idx * 2]; /* high byte */
  int bit = idx * bd;
  return (row[bit >> 3] >> (8 - bd - (bit & 7))) & ((1u << bd) - 1);
}

static unsigned sample16(const unsigned char *row, int idx, int bd)
{
  if (bd == 16)
    return be16(row + idx * 2);
  return sample(row, idx, bd);
}

static int png_encode(const unsigned char *d, size_t n, pdf_image *img, pbuf *warnings, const char *name)
{
  if (n < 33)
    return -1;
  uint32_t w = be32(d + 16), h = be32(d + 20);
  int bd = d[24], ct = d[25], interlace = d[28];
  if (w == 0 || h == 0 || w > 30000 || h > 30000)
    return -1;
  int channels = ct == 0 ? 1 : ct == 2 ? 3 : ct == 3 ? 1 : ct == 4 ? 2 : ct == 6 ? 4 : 0;
  if (!channels || (bd != 1 && bd != 2 && bd != 4 && bd != 8 && bd != 16))
    return -1;
  pbuf idat = { 0 };
  unsigned char palette[256 * 3] = { 0 };
  unsigned char palpha[256];
  memset(palpha, 255, sizeof palpha);
  int npal = 0;
  bool has_trns = false;
  unsigned trns_key[3] = { 0 };
  size_t pos = 8;
  while (pos + 12 <= n)
  {
    uint32_t len = be32(d + pos);
    const unsigned char *type = d + pos + 4, *data = d + pos + 8;
    if (len > n - pos - 12)
      break;
    if (!memcmp(type, "IDAT", 4))
      pbuf_append(&idat, data, len);
    else if (!memcmp(type, "PLTE", 4))
    {
      npal = (int)(len / 3 > 256 ? 256 : len / 3);
      memcpy(palette, data, (size_t)npal * 3);
    }
    else if (!memcmp(type, "tRNS", 4))
    {
      has_trns = true;
      if (ct == 3)
        memcpy(palpha, data, len > 256 ? 256 : len);
      else if (ct == 0 && len >= 2)
        trns_key[0] = be16(data);
      else if (ct == 2 && len >= 6)
      {
        trns_key[0] = be16(data);
        trns_key[1] = be16(data + 2);
        trns_key[2] = be16(data + 4);
      }
    }
    else if (!memcmp(type, "IEND", 4))
      break;
    pos += 12 + len;
  }
  pbuf raw = { 0 };
  if (!idat.len || pbuf_inflate(&raw, idat.data, idat.len) != 0)
  {
    pbuf_free(&idat);
    pbuf_free(&raw);
    return -1;
  }
  pbuf_free(&idat);

  bool gray = ct == 0 || ct == 4;
  int outc = gray ? 1 : 3;
  bool alpha = ct == 4 || ct == 6 || has_trns;
  size_t npix = (size_t)w * h;
  unsigned char *color = malloc(npix * outc), *amask = alpha ? malloc(npix) : NULL;
  if (!color || (alpha && !amask))
    abort();
  if (amask)
    memset(amask, 255, npix);

  static const int ax0[7] = { 0, 4, 0, 2, 0, 1, 0 }, ay0[7] = { 0, 0, 4, 0, 2, 0, 1 };
  static const int adx[7] = { 8, 8, 4, 4, 2, 2, 1 }, ady[7] = { 8, 8, 8, 4, 4, 2, 2 };
  int passes = interlace ? 7 : 1;
  size_t rpos = 0;
  int bpp = (channels * bd + 7) / 8;
  bool ok = true;
  unsigned maxv = (1u << (bd == 16 ? 8 : bd)) - 1;
  for (int p = 0; p < passes && ok; p++)
  {
    int x0 = interlace ? ax0[p] : 0, y0 = interlace ? ay0[p] : 0;
    int dx = interlace ? adx[p] : 1, dy = interlace ? ady[p] : 1;
    int pw = (int)((w - x0 + dx - 1) / dx), ph = (int)((h - y0 + dy - 1) / dy);
    if ((uint32_t)x0 >= w || (uint32_t)y0 >= h || pw <= 0 || ph <= 0)
      continue;
    size_t stride = ((size_t)pw * channels * bd + 7) / 8;
    unsigned char *rows = malloc(stride * ph + 1);
    if (!rows)
      abort();
    ok = unfilter(raw.data, raw.len, &rpos, stride, ph, bpp, rows);
    for (int yy = 0; ok && yy < ph; yy++)
    {
      const unsigned char *row = rows + (size_t)yy * stride;
      for (int xx = 0; xx < pw; xx++)
      {
        size_t o = (size_t)(y0 + yy * dy) * w + (size_t)(x0 + xx * dx);
        if (ct == 3)
        {
          unsigned idx = sample(row, xx, bd);
          memcpy(color + o * 3, palette + (idx < 256 ? idx : 0) * 3, 3);
          if (amask)
            amask[o] = palpha[idx & 255];
        }
        else if (gray)
        {
          unsigned v = sample(row, xx * channels, bd);
          color[o] = (unsigned char)(bd >= 8 ? v : v * 255 / maxv);
          if (ct == 4)
            amask[o] = (unsigned char)sample(row, xx * channels + 1, bd);
          else if (has_trns && sample16(row, xx, bd) == trns_key[0])
            amask[o] = 0;
        }
        else
        {
          for (int c = 0; c < 3; c++)
            color[o * 3 + c] = (unsigned char)sample(row, xx * channels + c, bd);
          if (ct == 6)
            amask[o] = (unsigned char)sample(row, xx * channels + 3, bd);
          else if (has_trns && sample16(row, xx * 3, bd) == trns_key[0] &&
                   sample16(row, xx * 3 + 1, bd) == trns_key[1] && sample16(row, xx * 3 + 2, bd) == trns_key[2])
            amask[o] = 0;
        }
      }
    }
    free(rows);
  }
  pbuf_free(&raw);
  if (!ok)
  {
    pbuf_printf(warnings, "truncated PNG data: %s\n", name);
    free(color);
    free(amask);
    return -1;
  }
  if (amask)
  {
    bool opaque = true;
    for (size_t i = 0; i < npix && opaque; i++)
      opaque = amask[i] == 255;
    if (!opaque)
      pbuf_deflate(&img->smask, amask, npix, 6);
  }
  pbuf_deflate(&img->data, color, npix * outc, 6);
  free(color);
  free(amask);
  img->width = (int)w;
  img->height = (int)h;
  snprintf(img->dict, sizeof img->dict, "/Width %u/Height %u/ColorSpace/%s/BitsPerComponent 8", w, h,
           gray ? "DeviceGray" : "DeviceRGB");
  return 0;
}

/* ------------------------------------------------------------------ */
/* JPEG                                                                */

static int jpeg_encode(const unsigned char *d, size_t n, pdf_image *img)
{
  size_t pos = 2;
  bool adobe = false;
  int comps = 0;
  while (pos + 4 <= n)
  {
    if (d[pos] != 0xFF)
    {
      pos++;
      continue;
    }
    unsigned m = d[pos + 1];
    if (m == 0xFF || m == 0xD8 || (m >= 0xD0 && m <= 0xD7) || m == 0x01)
    {
      pos += m == 0xFF ? 1 : 2;
      continue;
    }
    if (m == 0xD9 || m == 0xDA)
      break;
    unsigned len = be16(d + pos + 2);
    if (len < 2 || pos + 2 + len > n)
      break;
    const unsigned char *seg = d + pos + 4;
    if (m == 0xEE && len >= 7 && memcmp(seg, "Adobe", 5) == 0)
      adobe = true;
    if (m >= 0xC0 && m <= 0xCF && m != 0xC4 && m != 0xC8 && m != 0xCC && len >= 8)
    {
      img->height = (int)be16(seg + 1);
      img->width = (int)be16(seg + 3);
      comps = seg[5];
    }
    pos += 2 + len;
  }
  if (!img->width || !img->height || (comps != 1 && comps != 3 && comps != 4))
    return -1;
  snprintf(img->dict, sizeof img->dict, "/Width %d/Height %d/ColorSpace/%s/BitsPerComponent 8%s", img->width,
           img->height, comps == 1 ? "DeviceGray" : comps == 3 ? "DeviceRGB" : "DeviceCMYK",
           (comps == 4 && adobe) ? "/Decode[1 0 1 0 1 0 1 0]" : "");
  img->dct = true;
  pbuf_append(&img->data, d, n);
  return 0;
}

/* ------------------------------------------------------------------ */
/* BMP (uncompressed 8/24/32-bit)                                      */

static int bmp_encode(const unsigned char *d, size_t n, pdf_image *img, pbuf *warnings, const char *name)
{
  if (n < 54)
    return -1;
  uint32_t off = le32(d + 10), hsize = le32(d + 14);
  if (hsize < 40)
  {
    pbuf_printf(warnings, "unsupported BMP header: %s\n", name);
    return -1;
  }
  int32_t w = (int32_t)le32(d + 18), h = (int32_t)le32(d + 22);
  unsigned bpp = le16(d + 28);
  uint32_t comp = le32(d + 30), ncol = le32(d + 46);
  bool topdown = h < 0;
  if (h < 0)
    h = -h;
  if (w <= 0 || h <= 0 || w > 30000 || h > 30000 || comp != 0 || (bpp != 8 && bpp != 24 && bpp != 32))
  {
    pbuf_printf(warnings, "unsupported BMP variant (compressed or %u-bit): %s\n", bpp, name);
    return -1;
  }
  size_t stride = (((size_t)w * bpp + 31) / 32) * 4;
  if (off > n || stride * (size_t)h > n - off)
    return -1;
  const unsigned char *pal = d + 14 + hsize;
  if (bpp == 8 && ncol == 0)
    ncol = 256;
  unsigned char *rgb = malloc((size_t)w * h * 3);
  if (!rgb)
    abort();
  for (int y = 0; y < h; y++)
  {
    const unsigned char *row = d + off + stride * (size_t)(topdown ? y : h - 1 - y);
    for (int x = 0; x < w; x++)
    {
      unsigned char *o = rgb + ((size_t)y * w + x) * 3;
      if (bpp == 8)
      {
        unsigned i = row[x];
        const unsigned char *c = pal + 4 * (i < ncol ? i : 0);
        if (c + 3 > d + n)
          c = pal;
        o[0] = c[2];
        o[1] = c[1];
        o[2] = c[0];
      }
      else
      {
        const unsigned char *c = row + x * (bpp / 8);
        o[0] = c[2];
        o[1] = c[1];
        o[2] = c[0];
      }
    }
  }
  pbuf_deflate(&img->data, rgb, (size_t)w * h * 3, 6);
  free(rgb);
  img->width = w;
  img->height = h;
  snprintf(img->dict, sizeof img->dict, "/Width %d/Height %d/ColorSpace/DeviceRGB/BitsPerComponent 8", w, h);
  return 0;
}

int image_encode(const unsigned char *data, size_t len, const char *name, pdf_image *out, pbuf *warnings)
{
  memset(out, 0, sizeof *out);
  pitex_img_info info;
  if (pitex_img_info_read(data, len, &info) != 0)
  {
    pbuf_printf(warnings, "unrecognized image format: %s\n", name);
    return -1;
  }
  out->xdpi = info.xdpi;
  out->ydpi = info.ydpi;
  int r = -1;
  switch (info.kind)
  {
    case PITEX_IMG_PNG: r = png_encode(data, len, out, warnings, name); break;
    case PITEX_IMG_JPEG: r = jpeg_encode(data, len, out); break;
    case PITEX_IMG_BMP: r = bmp_encode(data, len, out, warnings, name); break;
    default: break;
  }
  if (r != 0)
  {
    image_free(out);
    pbuf_printf(warnings, "could not decode image: %s\n", name);
    return -1;
  }
  out->ok = true;
  return 0;
}
