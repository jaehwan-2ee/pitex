/* Pitex embedded preview engine — minimal PDF reader.
 * Independently written from the PDF 1.7 / ISO 32000 specification.
 * Pitex-authored (PolyForm Shield 1.0.0, see repository LICENSE).
 *
 * Used by the engine to size included PDF figures (page count, page boxes,
 * /Rotate) and by the driver to import a page as a Form XObject. Input is
 * untrusted: every access is bounds-checked and recursion is limited. */
#ifndef PITEX_PDFREAD_H
#define PITEX_PDFREAD_H

#include <stdbool.h>
#include <stddef.h>
#include "pitex_buf.h"

typedef enum {
  PR_NULL,
  PR_BOOL,
  PR_INT,
  PR_REAL,
  PR_NAME,
  PR_STRING,
  PR_ARRAY,
  PR_DICT,
  PR_REF,
  PR_STREAM
} pr_type;

typedef struct pr_obj pr_obj;
struct pr_obj {
  pr_type type;
  union {
    int b;
    long long i;
    double r;
    struct { char *s; size_t len; } str; /* NAME: NUL-terminated, #xx decoded */
    struct { pr_obj **items; int n; } arr;
    struct { char **keys; pr_obj **vals; int n; } dict;
    struct { int num, gen; } ref;
    struct { pr_obj *dict; size_t offset, length; } stream; /* raw span */
  } u;
};

typedef struct pr_doc pr_doc;

typedef struct {
  double box[4];     /* selected box, normalized llx lly urx ury */
  int rotate;        /* 0, 90, 180, 270 */
  pr_obj *page;      /* page dictionary */
  pr_obj *resources; /* inherited /Resources (may be NULL) */
} pr_page_info;

/* Box kinds follow XeTeX's pdfbox_* numbering. */
enum { PR_BOX_CROP = 1, PR_BOX_MEDIA = 2, PR_BOX_BLEED = 3, PR_BOX_TRIM = 4, PR_BOX_ART = 5 };

/* `data` must stay valid until pr_close. Returns NULL if unparseable. */
pr_doc *pr_open(const unsigned char *data, size_t len);
void pr_close(pr_doc *doc);
bool pr_is_encrypted(pr_doc *doc);
int pr_page_count(pr_doc *doc);
/* index0 is 0-based. */
bool pr_page(pr_doc *doc, int index0, int box_kind, pr_page_info *out);
/* XeTeX/dvipdfmx page argument normalization: 0 -> 1, negative from end. */
int pr_normalize_page(pr_doc *doc, int page_arg);

pr_obj *pr_resolve(pr_doc *doc, pr_obj *obj);
pr_obj *pr_load(pr_doc *doc, int num);
pr_obj *pr_dict_raw(pr_obj *dict, const char *key);   /* no resolution */
pr_obj *pr_get(pr_doc *doc, pr_obj *dict, const char *key); /* resolved */
bool pr_number(pr_obj *obj, double *out);

/* Raw (still encoded) stream bytes. */
const unsigned char *pr_stream_raw(pr_doc *doc, pr_obj *stream, size_t *len);
/* Decode FlateDecode (+PNG predictors), ASCIIHexDecode, ASCII85Decode
 * chains. Returns false for other filters or corrupt data. */
bool pr_stream_decode(pr_doc *doc, pr_obj *stream, pbuf *out);

/* Matrix mapping default user space to the displayed orientation for a
 * page /Rotate value (multiples of 90). */
void pr_rotation_matrix(int rotate, double m[6]);
/* Transform box by matrix and return the axis-aligned bounds. */
void pr_transform_box(const double m[6], const double box[4], double out[4]);

#endif
