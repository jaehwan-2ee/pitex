/* Pitex embedded preview engine — DVI/XDV stream index.
 * Independently written from the DVI format (Knuth, "dvitype") and the
 * XeTeX XDV extensions (opcodes 252-254, id byte 7).
 * Pitex-authored (PolyForm Shield 1.0.0). */
#ifndef PITEX_XDV_H
#define PITEX_XDV_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

enum {
  DVI_SET_CHAR_0 = 0, DVI_SET1 = 128, DVI_SET_RULE = 132, DVI_PUT1 = 133, DVI_PUT_RULE = 137,
  DVI_NOP = 138, DVI_BOP = 139, DVI_EOP = 140, DVI_PUSH = 141, DVI_POP = 142,
  DVI_RIGHT1 = 143, DVI_W0 = 147, DVI_W1 = 148, DVI_X0 = 152, DVI_X1 = 153,
  DVI_DOWN1 = 157, DVI_Y0 = 161, DVI_Y1 = 162, DVI_Z0 = 166, DVI_Z1 = 167,
  DVI_FNT_NUM_0 = 171, DVI_FNT1 = 235, DVI_XXX1 = 239, DVI_FNT_DEF1 = 243,
  DVI_PRE = 247, DVI_POST = 248, DVI_POST_POST = 249,
  XDV_NATIVE_FONT_DEF = 252, XDV_GLYPHS = 253, XDV_TEXT_AND_GLYPHS = 254
};

/* Length of the instruction at p (including opcode), or 0 if it is not
 * complete within [p, end), or -1 if the opcode is invalid. */
long xdv_insn_length(const unsigned char *p, const unsigned char *end);

typedef struct {
  size_t bop, eop_end; /* offsets: first byte of BOP, byte after EOP */
} xdv_page;

typedef struct xdv_index xdv_index;

xdv_index *xdv_index_new(void);
void xdv_index_free(xdv_index *x);
void xdv_index_reset(xdv_index *x);
/* Re-synchronize with the buffer, which may have grown or been truncated
 * (checkpoint rollback). */
void xdv_index_update(xdv_index *x, const unsigned char *data, size_t len);
int xdv_index_page_count(const xdv_index *x);
const xdv_page *xdv_index_page(const xdv_index *x, int i);
bool xdv_index_output_started(const xdv_index *x);
/* Preamble values (num, den, mag); false until the preamble is complete. */
bool xdv_index_preamble(const xdv_index *x, uint32_t *num, uint32_t *den, uint32_t *mag);

#endif
