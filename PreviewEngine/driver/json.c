/* Pitex embedded preview engine — minimal JSON reader/writer.
 * Pitex-authored (PolyForm Shield 1.0.0). */
#include "json.h"

#include <stdlib.h>
#include <string.h>

typedef struct {
  const char *p, *end;
  int depth;
} jparser;

static void ws(jparser *j)
{
  while (j->p < j->end && (*j->p == ' ' || *j->p == '\t' || *j->p == '\n' || *j->p == '\r'))
    j->p++;
}

static json *jnew(json_type t)
{
  json *v = calloc(1, sizeof *v);
  if (!v)
    abort();
  v->type = t;
  return v;
}

static void put_utf8(pbuf *b, unsigned cp)
{
  if (cp < 0x80)
    pbuf_putc(b, (int)cp);
  else if (cp < 0x800)
  {
    pbuf_putc(b, 0xC0 | (cp >> 6));
    pbuf_putc(b, 0x80 | (cp & 0x3F));
  }
  else if (cp < 0x10000)
  {
    pbuf_putc(b, 0xE0 | (cp >> 12));
    pbuf_putc(b, 0x80 | ((cp >> 6) & 0x3F));
    pbuf_putc(b, 0x80 | (cp & 0x3F));
  }
  else
  {
    pbuf_putc(b, 0xF0 | (cp >> 18));
    pbuf_putc(b, 0x80 | ((cp >> 12) & 0x3F));
    pbuf_putc(b, 0x80 | ((cp >> 6) & 0x3F));
    pbuf_putc(b, 0x80 | (cp & 0x3F));
  }
}

static int hex4(const char *p, unsigned *out)
{
  unsigned v = 0;
  for (int i = 0; i < 4; i++)
  {
    int c = p[i], d;
    if (c >= '0' && c <= '9')
      d = c - '0';
    else if (c >= 'a' && c <= 'f')
      d = c - 'a' + 10;
    else if (c >= 'A' && c <= 'F')
      d = c - 'A' + 10;
    else
      return -1;
    v = v * 16 + (unsigned)d;
  }
  *out = v;
  return 0;
}

static json *parse_value(jparser *j);

static char *parse_string_raw(jparser *j, size_t *len)
{
  if (j->p >= j->end || *j->p != '"')
    return NULL;
  j->p++;
  pbuf b = { 0 };
  while (j->p < j->end && *j->p != '"')
  {
    unsigned char c = (unsigned char)*j->p++;
    if (c != '\\')
    {
      pbuf_putc(&b, c);
      continue;
    }
    if (j->p >= j->end)
      goto fail;
    char e = *j->p++;
    switch (e)
    {
      case '"': pbuf_putc(&b, '"'); break;
      case '\\': pbuf_putc(&b, '\\'); break;
      case '/': pbuf_putc(&b, '/'); break;
      case 'b': pbuf_putc(&b, '\b'); break;
      case 'f': pbuf_putc(&b, '\f'); break;
      case 'n': pbuf_putc(&b, '\n'); break;
      case 'r': pbuf_putc(&b, '\r'); break;
      case 't': pbuf_putc(&b, '\t'); break;
      case 'u':
      {
        unsigned cp;
        if (j->end - j->p < 4 || hex4(j->p, &cp))
          goto fail;
        j->p += 4;
        if (cp >= 0xD800 && cp < 0xDC00 && j->end - j->p >= 6 && j->p[0] == '\\' && j->p[1] == 'u')
        {
          unsigned lo;
          if (!hex4(j->p + 2, &lo) && lo >= 0xDC00 && lo < 0xE000)
          {
            j->p += 6;
            cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
          }
        }
        if (cp >= 0xD800 && cp < 0xE000)
          cp = 0xFFFD;
        put_utf8(&b, cp);
        break;
      }
      default:
        goto fail;
    }
  }
  if (j->p >= j->end)
    goto fail;
  j->p++;
  pbuf_putc(&b, 0);
  *len = b.len - 1;
  return (char *)b.data;
fail:
  pbuf_free(&b);
  return NULL;
}

static json *parse_value(jparser *j)
{
  ws(j);
  if (j->p >= j->end || ++j->depth > 64)
    return NULL;
  json *v = NULL;
  char c = *j->p;
  if (c == '{' || c == '[')
  {
    bool obj = c == '{';
    j->p++;
    v = jnew(obj ? JSON_OBJECT : JSON_ARRAY);
    int cap = 0;
    ws(j);
    if (j->p < j->end && *j->p == (obj ? '}' : ']'))
    {
      j->p++;
      j->depth--;
      return v;
    }
    for (;;)
    {
      char *key = NULL;
      size_t klen;
      if (obj)
      {
        ws(j);
        key = parse_string_raw(j, &klen);
        ws(j);
        if (!key || j->p >= j->end || *j->p != ':')
        {
          free(key);
          goto fail;
        }
        j->p++;
      }
      json *item = parse_value(j);
      if (!item)
      {
        free(key);
        goto fail;
      }
      if (v->n == cap)
      {
        cap = cap ? cap * 2 : 8;
        v->items = realloc(v->items, sizeof(json *) * cap);
        if (obj)
          v->keys = realloc(v->keys, sizeof(char *) * cap);
        if (!v->items || (obj && !v->keys))
          abort();
      }
      v->items[v->n] = item;
      if (obj)
        v->keys[v->n] = key;
      v->n++;
      ws(j);
      if (j->p < j->end && *j->p == ',')
      {
        j->p++;
        continue;
      }
      if (j->p < j->end && *j->p == (obj ? '}' : ']'))
      {
        j->p++;
        break;
      }
      goto fail;
    }
  }
  else if (c == '"')
  {
    v = jnew(JSON_STRING);
    v->str = parse_string_raw(j, &v->len);
    if (!v->str)
      goto fail;
  }
  else if (c == 't' && j->end - j->p >= 4 && !memcmp(j->p, "true", 4))
  {
    v = jnew(JSON_BOOL);
    v->b = true;
    j->p += 4;
  }
  else if (c == 'f' && j->end - j->p >= 5 && !memcmp(j->p, "false", 5))
  {
    v = jnew(JSON_BOOL);
    j->p += 5;
  }
  else if (c == 'n' && j->end - j->p >= 4 && !memcmp(j->p, "null", 4))
  {
    v = jnew(JSON_NULL);
    j->p += 4;
  }
  else if (c == '-' || (c >= '0' && c <= '9'))
  {
    char tmp[64];
    size_t n = 0;
    while (j->p < j->end && n < sizeof tmp - 1 &&
           (strchr("+-.eE", *j->p) || (*j->p >= '0' && *j->p <= '9')))
      tmp[n++] = *j->p++;
    tmp[n] = 0;
    v = jnew(JSON_NUMBER);
    v->num = strtod(tmp, NULL);
  }
  else
    return NULL;
  j->depth--;
  return v;
fail:
  json_free(v);
  return NULL;
}

json *json_parse(const char *text, size_t len)
{
  jparser j = { text, text + len, 0 };
  json *v = parse_value(&j);
  if (!v)
    return NULL;
  ws(&j);
  if (j.p != j.end)
  {
    json_free(v);
    return NULL;
  }
  return v;
}

void json_free(json *v)
{
  if (!v)
    return;
  for (int i = 0; i < v->n; i++)
  {
    json_free(v->items[i]);
    if (v->keys)
      free(v->keys[i]);
  }
  free(v->items);
  free(v->keys);
  free(v->str);
  free(v);
}

json *json_get(const json *o, const char *key)
{
  if (!o || o->type != JSON_OBJECT)
    return NULL;
  for (int i = o->n - 1; i >= 0; i--)
    if (strcmp(o->keys[i], key) == 0)
      return o->items[i];
  return NULL;
}

const char *json_string(const json *v) { return v && v->type == JSON_STRING ? v->str : NULL; }

bool json_number(const json *v, double *out)
{
  if (!v || v->type != JSON_NUMBER)
    return false;
  *out = v->num;
  return true;
}

void json_write_string(pbuf *out, const char *s, size_t len)
{
  pbuf_putc(out, '"');
  for (size_t i = 0; i < len; i++)
  {
    unsigned char c = (unsigned char)s[i];
    switch (c)
    {
      case '"': pbuf_puts(out, "\\\""); break;
      case '\\': pbuf_puts(out, "\\\\"); break;
      case '\n': pbuf_puts(out, "\\n"); break;
      case '\r': pbuf_puts(out, "\\r"); break;
      case '\t': pbuf_puts(out, "\\t"); break;
      default:
        if (c < 0x20)
          pbuf_printf(out, "\\u%04x", c);
        else if (c < 0x80)
          pbuf_putc(out, c);
        else
        {
          /* Copy valid UTF-8 sequences; replace invalid bytes (TeX logs
           * may contain raw 8-bit bytes) with U+FFFD. */
          int n = c >= 0xF0 && c < 0xF5 ? 4 : c >= 0xE0 && c < 0xF0 ? 3 : c >= 0xC2 && c < 0xE0 ? 2 : 0;
          bool ok = n > 0 && i + (size_t)n <= len;
          for (int k = 1; ok && k < n; k++)
            ok = ((unsigned char)s[i + k] & 0xC0) == 0x80;
          if (ok)
          {
            pbuf_append(out, s + i, (size_t)n);
            i += (size_t)n - 1;
          }
          else
            pbuf_puts(out, "\\ufffd");
        }
    }
  }
  pbuf_putc(out, '"');
}
