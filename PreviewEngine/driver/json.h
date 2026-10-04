/* Pitex embedded preview engine — minimal JSON reader/writer for the
 * driver's stdio protocol. Pitex-authored (PolyForm Shield 1.0.0). */
#ifndef PITEX_JSON_H
#define PITEX_JSON_H

#include <stdbool.h>
#include <stddef.h>
#include "pitex_buf.h"

typedef enum { JSON_NULL, JSON_BOOL, JSON_NUMBER, JSON_STRING, JSON_ARRAY, JSON_OBJECT } json_type;

typedef struct json json;
struct json {
  json_type type;
  bool b;
  double num;
  char *str;       /* STRING: UTF-8, NUL-terminated (may contain NUL: see len) */
  size_t len;
  json **items;    /* ARRAY/OBJECT values */
  char **keys;     /* OBJECT keys */
  int n;
};

/* Parses one JSON document; NULL on syntax error. */
json *json_parse(const char *text, size_t len);
void json_free(json *v);
json *json_get(const json *obj, const char *key);
const char *json_string(const json *v); /* NULL unless STRING */
bool json_number(const json *v, double *out);

/* Append a JSON string literal (with quotes) for UTF-8 bytes. */
void json_write_string(pbuf *out, const char *s, size_t len);

#endif
