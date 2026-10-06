/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009 by Jonathan Kew

 SIL Author(s): Jonathan Kew

Permission is hereby granted, free of charge, to any person obtaining
a copy of this software and associated documentation files (the
"Software"), to deal in the Software without restriction, including
without limitation the rights to use, copy, modify, merge, publish,
distribute, sublicense, and/or sell copies of the Software, and to
permit persons to whom the Software is furnished to do so, subject to
the following conditions:

The above copyright notice and this permission notice shall be
included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE COPYRIGHT HOLDERS BE LIABLE
FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF
CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION
WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

Except as contained in this notice, the name of the copyright holders
shall not be used in advertising or otherwise to promote the sale,
use or other dealings in this Software without prior written
authorization from the copyright holders.
\****************************************************************************/

// Opaque class lifetime and SDK record field access only. File loading,
// HarfBuzz callbacks, table copying and glyph metric algorithms are Rust.
#include "tectonic_xetex_layout.h"
#include "xetex-XeTeXFontInst.h"
#include "pitex_font_freetype.h"

XeTeXFontInst::XeTeXFontInst(const char *path, int index, float size, int &status)
    : m_unitsPerEM(0), m_pointSize(size), m_ascent(0), m_descent(0),
      m_capHeight(0), m_xHeight(0), m_italicAngle(0), m_vertical(false),
      m_filename(nullptr), m_index(0), m_ftFace(nullptr), m_backingData(nullptr),
      m_backingData2(nullptr), m_hbFont(nullptr) {
    if (path) initialize(path, index, status);
}
XeTeXFontInst::~XeTeXFontInst() {
    if (m_ftFace) FT_Done_Face(m_ftFace);
    hb_font_destroy(m_hbFont);
    free(m_backingData); free(m_backingData2); free(m_filename);
}
void XeTeXFontInst::initialize(const char *path, int index, int &status) {
    PitexFtFontInit value{};
    pitex_ft_initialize(path, index, m_pointSize, &value, &status);
    m_ftFace = static_cast<FT_Face>(value.face);
    m_backingData = static_cast<FT_Byte *>(value.data);
    m_backingData2 = static_cast<FT_Byte *>(value.afm_data);
    m_hbFont = static_cast<hb_font_t *>(value.hb_font);
    m_filename = value.filename; m_index = value.index; m_unitsPerEM = value.units_per_em;
    m_ascent = value.ascent; m_descent = value.descent;
    m_capHeight = value.cap_height; m_xHeight = value.x_height; m_italicAngle = value.italic_angle;
}
void XeTeXFontInst::setLayoutDirVertical(bool vertical) { m_vertical = vertical; }
void *XeTeXFontInst::getFontTable(OTTag tag) const { return pitex_ft_table(m_ftFace, tag); }
void *XeTeXFontInst::getFontTable(FT_Sfnt_Tag tag) const { return FT_Get_Sfnt_Table(m_ftFace, tag); }
void XeTeXFontInst::getGlyphBounds(GlyphID glyph, GlyphBBox *bounds) { pitex_ft_bounds(m_ftFace, m_pointSize, m_unitsPerEM, glyph, bounds); }
GlyphID XeTeXFontInst::mapCharToGlyph(UChar32 character) const { return FT_Get_Char_Index(m_ftFace, character); }
uint16_t XeTeXFontInst::getNumGlyphs() const { return m_ftFace->num_glyphs; }
float XeTeXFontInst::getGlyphWidth(GlyphID glyph) { return unitsToPoints(pitex_ft_advance(m_ftFace, glyph, false)); }
void XeTeXFontInst::getGlyphHeightDepth(GlyphID glyph, float *height, float *depth) { pitex_ft_height_depth(m_ftFace, m_pointSize, m_unitsPerEM, glyph, height, depth); }
void XeTeXFontInst::getGlyphSidebearings(GlyphID glyph, float *left, float *right) { pitex_ft_sidebearings(m_ftFace, m_pointSize, m_unitsPerEM, glyph, left, right); }
float XeTeXFontInst::getGlyphItalCorr(GlyphID glyph) { return pitex_ft_italic_correction(m_ftFace, m_pointSize, m_unitsPerEM, glyph); }
GlyphID XeTeXFontInst::mapGlyphToIndex(const char *name) const { return FT_Get_Name_Index(m_ftFace, const_cast<char *>(name)); }
const char *XeTeXFontInst::getGlyphName(GlyphID glyph, int &length) { return pitex_ft_glyph_name(m_ftFace, glyph, &length); }
UChar32 XeTeXFontInst::getFirstCharCode() { FT_UInt glyph; return FT_Get_First_Char(m_ftFace, &glyph); }
UChar32 XeTeXFontInst::getLastCharCode() { return pitex_ft_last_character(m_ftFace); }

extern "C" void pitex_ft_face_info(void *opaque, PitexFtFaceInfo *result) {
    auto *face = static_cast<FT_Face>(opaque);
    *result = {};
    result->flags = face->face_flags; result->num_faces = face->num_faces; result->num_glyphs = face->num_glyphs;
    result->units_per_em = face->units_per_EM; result->ascender = face->ascender; result->descender = face->descender;
    if (auto *post = static_cast<TT_Postscript *>(FT_Get_Sfnt_Table(face, ft_sfnt_post))) {
        result->post_present = 1; result->italic_angle = post->italicAngle;
    }
    if (auto *os2 = static_cast<TT_OS2 *>(FT_Get_Sfnt_Table(face, ft_sfnt_os2))) {
        result->os2_present = 1; result->cap_height = os2->sCapHeight; result->x_height = os2->sxHeight;
    }
}
extern "C" void pitex_ft_loaded_glyph(void *opaque, PitexFtGlyphInfo *result) {
    auto *face = static_cast<FT_Face>(opaque); auto *glyph = face->glyph;
    result->slot = glyph; result->x_bearing = glyph->metrics.horiBearingX;
    result->y_bearing = glyph->metrics.horiBearingY; result->width = glyph->metrics.width; result->height = glyph->metrics.height;
    result->format = glyph->format; result->point_count = glyph->outline.n_points; result->points = glyph->outline.points;
}
extern "C" void pitex_ft_attach_memory(void *opaque, const void *data, unsigned long length) {
    FT_Open_Args args{}; args.flags = FT_OPEN_MEMORY;
    args.memory_base = static_cast<const FT_Byte *>(data); args.memory_size = length;
    FT_Attach_Stream(static_cast<FT_Face>(opaque), &args);
}
