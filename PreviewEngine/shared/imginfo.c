/* Pitex embedded preview engine — raster image header parsing.
 * Independently written from the PNG, JPEG/JFIF/Exif and BMP format
 * specifications. Pitex-authored (PolyForm Shield 1.0.0). */
#include "imginfo.h"

#include <stdint.h>
#include <string.h>

static unsigned be16(const unsigned char *p) { return (unsigned)p[0] << 8 | p[1]; }
static unsigned long be32(const unsigned char *p)
{
  return (unsigned long)p[0] << 24 | (unsigned long)p[1] << 16 | (unsigned long)p[2] << 8 | p[3];
}
static unsigned le16(const unsigned char *p) { return (unsigned)p[1] << 8 | p[0]; }
static unsigned long le32(const unsigned char *p)
{
  return (unsigned long)p[3] << 24 | (unsigned long)p[2] << 16 | (unsigned long)p[1] << 8 | p[0];
}

pitex_img_kind pitex_img_sniff(const unsigned char *d, size_t n)
{
  if (n >= 8 && memcmp(d, "\x89PNG\r\n\x1a\n", 8) == 0)
    return PITEX_IMG_PNG;
  if (n >= 3 && d[0] == 0xFF && d[1] == 0xD8 && d[2] == 0xFF)
    return PITEX_IMG_JPEG;
  if (n >= 26 && d[0] == 'B' && d[1] == 'M')
    return PITEX_IMG_BMP;
  return PITEX_IMG_NONE;
}

static int png_info(const unsigned char *d, size_t n, pitex_img_info *out)
{
  if (n < 33 || memcmp(d + 12, "IHDR", 4) != 0)
    return -1;
  out->width = (unsigned)be32(d + 16);
  out->height = (unsigned)be32(d + 20);
  size_t pos = 8;
  while (pos + 12 <= n)
  {
    unsigned long len = be32(d + pos);
    const unsigned char *type = d + pos + 4;
    if (len > n - pos - 12)
      break;
    if (memcmp(type, "IDAT", 4) == 0 || memcmp(type, "IEND", 4) == 0)
      break;
    if (memcmp(type, "pHYs", 4) == 0 && len >= 9)
    {
      const unsigned char *p = d + pos + 8;
      double px = (double)be32(p), py = (double)be32(p + 4);
      if (px > 0 && py > 0)
      {
        if (p[8] == 1)
        {
          out->xdpi = px * 0.0254;
          out->ydpi = py * 0.0254;
        }
        else
          out->ydpi = 72.0 * py / px; /* aspect ratio only */
      }
    }
    pos += 12 + len;
  }
  return out->width && out->height ? 0 : -1;
}

static void exif_resolution(const unsigned char *t, size_t n, pitex_img_info *out)
{
  if (n < 8)
    return;
  int le;
  if (t[0] == 'I' && t[1] == 'I')
    le = 1;
  else if (t[0] == 'M' && t[1] == 'M')
    le = 0;
  else
    return;
#define U16(p) (le ? le16(p) : be16(p))
#define U32(p) (le ? le32(p) : be32(p))
  unsigned long ifd = U32(t + 4);
  if (ifd + 2 > n)
    return;
  unsigned count = U16(t + ifd);
  double xr = 0, yr = 0;
  unsigned unit = 2;
  for (unsigned i = 0; i < count; i++)
  {
    size_t e = ifd + 2 + (size_t)i * 12;
    if (e + 12 > n)
      break;
    unsigned tag = U16(t + e), type = U16(t + e + 2);
    if ((tag == 0x011A || tag == 0x011B) && type == 5)
    {
      unsigned long off = U32(t + e + 8);
      if (off + 8 <= n)
      {
        unsigned long num = U32(t + off), den = U32(t + off + 4);
        double v = den ? (double)num / (double)den : 0;
        if (tag == 0x011A)
          xr = v;
        else
          yr = v;
      }
    }
    else if (tag == 0x0128 && type == 3)
      unit = U16(t + e + 8);
  }
#undef U16
#undef U32
  if (xr > 0 && yr > 0 && (unit == 2 || unit == 3))
  {
    double k = unit == 3 ? 2.54 : 1.0;
    out->xdpi = xr * k;
    out->ydpi = yr * k;
  }
}

static int jpeg_info(const unsigned char *d, size_t n, pitex_img_info *out)
{
  size_t pos = 2;
  int have_jfif_density = 0;
  while (pos + 4 <= n)
  {
    if (d[pos] != 0xFF)
    {
      pos++;
      continue;
    }
    unsigned m = d[pos + 1];
    if (m == 0xFF)
    {
      pos++;
      continue;
    }
    if (m == 0xD8 || (m >= 0xD0 && m <= 0xD7) || m == 0x01)
    {
      pos += 2;
      continue;
    }
    if (m == 0xD9 || m == 0xDA)
      break;
    unsigned len = be16(d + pos + 2);
    if (len < 2 || pos + 2 + len > n)
      break;
    const unsigned char *seg = d + pos + 4;
    size_t slen = len - 2;
    if (m == 0xE0 && slen >= 12 && memcmp(seg, "JFIF\0", 5) == 0)
    {
      unsigned units = seg[7], xd = be16(seg + 8), yd = be16(seg + 10);
      if (xd && yd)
      {
        if (units == 1 || units == 2)
        {
          double k = units == 2 ? 2.54 : 1.0;
          out->xdpi = xd * k;
          out->ydpi = yd * k;
          have_jfif_density = 1;
        }
        else if (units == 0)
          out->ydpi = 72.0 * yd / xd;
      }
    }
    else if (m == 0xE1 && slen >= 14 && memcmp(seg, "Exif\0\0", 6) == 0 && !have_jfif_density)
      exif_resolution(seg + 6, slen - 6, out);
    else if ((m >= 0xC0 && m <= 0xCF) && m != 0xC4 && m != 0xC8 && m != 0xCC && slen >= 6)
    {
      out->height = be16(seg + 1);
      out->width = be16(seg + 3);
      return out->width && out->height ? 0 : -1;
    }
    pos += 2 + len;
  }
  return -1;
}

static int bmp_info(const unsigned char *d, size_t n, pitex_img_info *out)
{
  unsigned long hsize = le32(d + 14);
  if (hsize == 12 && n >= 26)
  {
    out->width = le16(d + 18);
    out->height = le16(d + 20);
  }
  else if (hsize >= 40 && n >= 54)
  {
    long w = (long)(int32_t)le32(d + 18), h = (long)(int32_t)le32(d + 22);
    out->width = (unsigned)(w < 0 ? -w : w);
    out->height = (unsigned)(h < 0 ? -h : h);
    long xppm = (long)(int32_t)le32(d + 38), yppm = (long)(int32_t)le32(d + 42);
    if (xppm > 0 && yppm > 0)
    {
      out->xdpi = xppm * 0.0254;
      out->ydpi = yppm * 0.0254;
    }
  }
  else
    return -1;
  return out->width && out->height ? 0 : -1;
}

int pitex_img_info_read(const unsigned char *d, size_t n, pitex_img_info *out)
{
  memset(out, 0, sizeof *out);
  out->xdpi = out->ydpi = 72.0;
  out->kind = pitex_img_sniff(d, n);
  switch (out->kind)
  {
    case PITEX_IMG_PNG: return png_info(d, n, out);
    case PITEX_IMG_JPEG: return jpeg_info(d, n, out);
    case PITEX_IMG_BMP: return bmp_info(d, n, out);
    default: return -1;
  }
}
