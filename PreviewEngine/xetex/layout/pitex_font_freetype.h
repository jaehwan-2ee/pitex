/* Internal C ABI for the MIT XeTeX FontInst Rust translation. The complete
 * original notice is retained in src/shared/font_freetype.rs and the class
 * bridge. SDK-owned FreeType records remain opaque across this boundary. */
#ifndef PITEX_FONT_FREETYPE_H
#define PITEX_FONT_FREETYPE_H
#include <stdint.h>
#include <stdbool.h>
#include "tectonic_xetex_layout.h"
typedef struct {
    long flags, num_faces, num_glyphs;
    uint32_t units_per_em;
    int32_t ascender, descender, post_present, italic_angle;
    int32_t os2_present, cap_height, x_height;
} PitexFtFaceInfo;
typedef struct {
    void *slot;
    long x_bearing, y_bearing, width, height;
    uint32_t format;
    int32_t point_count;
    const void *points;
} PitexFtGlyphInfo;
typedef struct {
    void *face, *data, *afm_data, *hb_font;
    char *filename;
    uint32_t index, units_per_em;
    float ascent, descent, cap_height, x_height, italic_angle;
} PitexFtFontInit;
#ifdef __cplusplus
extern "C" {
#endif
void pitex_ft_face_info(void *face, PitexFtFaceInfo *result);
void pitex_ft_loaded_glyph(void *face, PitexFtGlyphInfo *result);
void pitex_ft_attach_memory(void *face, const void *data, unsigned long length);
void pitex_ft_initialize(const char *path, int index, float size, PitexFtFontInit *result, int *status);
void *pitex_ft_table(void *face, uint32_t tag);
long pitex_ft_advance(void *face, uint32_t glyph, bool vertical);
void pitex_ft_bounds(void *face, float size, uint32_t units, uint32_t glyph, GlyphBBox *bounds);
void pitex_ft_height_depth(void *face, float size, uint32_t units, uint32_t glyph, float *height, float *depth);
void pitex_ft_sidebearings(void *face, float size, uint32_t units, uint32_t glyph, float *left, float *right);
float pitex_ft_italic_correction(void *face, float size, uint32_t units, uint32_t glyph);
const char *pitex_ft_glyph_name(void *face, uint32_t glyph, int *length);
int32_t pitex_ft_last_character(void *face);
#ifdef XETEX_MAC
typedef struct { void *descriptor, *font; char *path; uint32_t index; } PitexMacFontInit;
void pitex_mac_font_initialize(const void *descriptor, float size, PitexMacFontInit *result, int *status);
#endif
#ifdef __cplusplus
}
#endif
#endif
