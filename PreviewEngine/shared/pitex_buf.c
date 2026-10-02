/* Pitex embedded preview engine — growable byte buffer.
 * Pitex-authored (PolyForm Shield 1.0.0, see repository LICENSE). */
#include "pitex_buf.h"

#include <math.h>
#include <stdio.h>
#include <zlib.h>

void pbuf_printf(pbuf *b, const char *fmt, ...)
{
  va_list ap;
  va_start(ap, fmt);
  char small[256];
  int n = vsnprintf(small, sizeof small, fmt, ap);
  va_end(ap);
  if (n < 0)
    return;
  if ((size_t)n < sizeof small)
  {
    pbuf_append(b, small, (size_t)n);
    return;
  }
  pbuf_reserve(b, (size_t)n + 1);
  va_start(ap, fmt);
  vsnprintf((char *)b->data + b->len, (size_t)n + 1, fmt, ap);
  va_end(ap);
  b->len += (size_t)n;
}

void pbuf_real(pbuf *b, double v)
{
  if (!isfinite(v))
    v = 0;
  if (fabs(v) < 5e-7)
    v = 0;
  char tmp[64];
  int n = snprintf(tmp, sizeof tmp, "%.6f", v);
  /* strip trailing zeros and dot */
  while (n > 0 && tmp[n - 1] == '0')
    n--;
  if (n > 0 && tmp[n - 1] == '.')
    n--;
  if (n == 0 || (n == 1 && tmp[0] == '-'))
  {
    tmp[0] = '0';
    n = 1;
  }
  if (n == 2 && tmp[0] == '-' && tmp[1] == '0')
  {
    tmp[0] = '0';
    n = 1;
  }
  pbuf_append(b, tmp, (size_t)n);
}

int pbuf_deflate(pbuf *out, const unsigned char *data, size_t len, int level)
{
  z_stream zs;
  memset(&zs, 0, sizeof zs);
  if (deflateInit(&zs, level) != Z_OK)
    return -1;
  pbuf_reserve(out, deflateBound(&zs, (uLong)len));
  zs.next_in = (Bytef *)data;
  zs.avail_in = (uInt)len;
  zs.next_out = out->data + out->len;
  zs.avail_out = (uInt)(out->cap - out->len);
  int r = deflate(&zs, Z_FINISH);
  out->len += zs.total_out;
  deflateEnd(&zs);
  return r == Z_STREAM_END ? 0 : -1;
}

int pbuf_inflate(pbuf *out, const unsigned char *data, size_t len)
{
  z_stream zs;
  memset(&zs, 0, sizeof zs);
  if (inflateInit(&zs) != Z_OK)
    return -1;
  zs.next_in = (Bytef *)data;
  zs.avail_in = (uInt)len;
  int r = Z_OK;
  size_t start = out->len;
  while (r == Z_OK)
  {
    if (out->len - start > ((size_t)1 << 30))
      break;
    pbuf_reserve(out, len > 4096 ? len : 4096);
    zs.next_out = out->data + out->len;
    zs.avail_out = (uInt)(out->cap - out->len);
    size_t before = zs.total_out;
    r = inflate(&zs, Z_NO_FLUSH);
    out->len += zs.total_out - before;
    if (r == Z_BUF_ERROR && zs.avail_in == 0)
      break; /* truncated stream: keep what we decoded */
  }
  inflateEnd(&zs);
  /* Many producers emit slightly truncated or trailing-garbage streams;
   * accept any stream that produced output. */
  return (r == Z_STREAM_END || out->len > start) ? 0 : -1;
}
