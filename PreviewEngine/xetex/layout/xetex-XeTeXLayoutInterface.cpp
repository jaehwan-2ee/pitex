/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009-2012 by Jonathan Kew
 Copyright (c) 2012-2015 by Khaled Hosny

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

// Only opaque XeTeXFontInst / XeTeXFontMgr class construction and method
// access remain here. Shaping, enumeration, caches and Graphite logic are Rust.
#include "tectonic_xetex_layout.h"
#include "xetex-XeTeXFontInst.h"
#include "xetex-XeTeXFontMgr.h"
#ifdef XETEX_MAC
#include "xetex-XeTeXFontInst_Mac.h"
#endif

void terminate_font_manager() { XeTeXFontMgr::Terminate(); }
void destroy_font_manager() { XeTeXFontMgr::Destroy(); }
XeTeXFont createFont(PlatformFontRef ref, Fixed size) {
    int status = 0;
#ifdef XETEX_MAC
    auto *font = new XeTeXFontInst_Mac(ref, Fix2D(size), status);
#else
    FcChar8 *path = nullptr;
    FcPatternGetString(ref, FC_FILE, 0, &path);
    int index;
    FcPatternGetInteger(ref, FC_INDEX, 0, &index);
    auto *font = new XeTeXFontInst(reinterpret_cast<const char *>(path), index, Fix2D(size), status);
#endif
    if (status) { delete font; return nullptr; }
    return reinterpret_cast<XeTeXFont>(font);
}
XeTeXFont createFontFromFile(const char *path, int index, Fixed size) {
    int status = 0;
    auto *font = new XeTeXFontInst(path, index, Fix2D(size), status);
    if (status) { delete font; return nullptr; }
    return reinterpret_cast<XeTeXFont>(font);
}
void setFontLayoutDir(XeTeXFont font, int vertical) { reinterpret_cast<XeTeXFontInst *>(font)->setLayoutDirVertical(vertical != 0); }
PlatformFontRef findFontByName(const char *name, char *variation, double size) { return XeTeXFontMgr::GetFontManager()->findFont(name, variation, size); }
char getReqEngine() { return XeTeXFontMgr::GetFontManager()->getReqEngine(); }
void setReqEngine(char engine) { XeTeXFontMgr::GetFontManager()->setReqEngine(engine); }
const char *getFullName(PlatformFontRef ref) { return XeTeXFontMgr::GetFontManager()->getFullName(ref); }
double getDesignSize(XeTeXFont font) { return XeTeXFontMgr::GetFontManager()->getDesignSize(font); }
void deleteFont(XeTeXFont font) { delete reinterpret_cast<XeTeXFontInst *>(font); }
void *getFontTablePtr(XeTeXFont font, uint32_t tag) { return const_cast<void *>(reinterpret_cast<XeTeXFontInst *>(font)->getFontTable(tag)); }
float getGlyphWidth(XeTeXFont font, uint32_t glyph) { return reinterpret_cast<XeTeXFontInst *>(font)->getGlyphWidth(glyph); }
unsigned countGlyphs(XeTeXFont font) { return reinterpret_cast<XeTeXFontInst *>(font)->getNumGlyphs(); }
const char *getGlyphName(XeTeXFont font, uint16_t glyph, int *length) { return reinterpret_cast<XeTeXFontInst *>(font)->getGlyphName(glyph, *length); }
float ttxl_font_units_to_points(XeTeXFont font, float value) { return reinterpret_cast<XeTeXFontInst *>(font)->unitsToPoints(value); }
float ttxl_font_points_to_units(XeTeXFont font, float value) { return reinterpret_cast<XeTeXFontInst *>(font)->pointsToUnits(value); }
float ttxl_font_get_point_size(XeTeXFont font) { return reinterpret_cast<XeTeXFontInst *>(font)->getPointSize(); }
const char *ttxl_platfont_get_desc(PlatformFontRef ref) { return XeTeXFontMgr::GetFontManager()->getPlatformFontDesc(ref).c_str(); }

extern "C" {
hb_font_t *pitex_font_hb(XeTeXFont font) { return reinterpret_cast<XeTeXFontInst *>(font)->getHbFont(); }
bool pitex_font_vertical(XeTeXFont font) { return reinterpret_cast<XeTeXFontInst *>(font)->getLayoutDirVertical(); }
float pitex_font_metric(XeTeXFont font, int which) {
    auto *instance = reinterpret_cast<XeTeXFontInst *>(font);
    switch (which) {
    case 0: return instance->getAscent();
    case 1: return instance->getDescent();
    case 2: return instance->getCapHeight();
    case 3: return instance->getXHeight();
    case 4: return instance->getItalicAngle();
    default: return 0;
    }
}
char *pitex_font_filename(XeTeXFont font, uint32_t *index) { return xstrdup(reinterpret_cast<XeTeXFontInst *>(font)->getFilename(index)); }
float pitex_font_units_to_points_double(XeTeXFont font, double value) { return reinterpret_cast<XeTeXFontInst *>(font)->unitsToPoints(value); }
void pitex_font_bounds(XeTeXFont font, uint32_t glyph, GlyphBBox *bounds) { reinterpret_cast<XeTeXFontInst *>(font)->getGlyphBounds(glyph, bounds); }
void pitex_font_height_depth(XeTeXFont font, uint32_t glyph, float *height, float *depth) { reinterpret_cast<XeTeXFontInst *>(font)->getGlyphHeightDepth(glyph, height, depth); }
void pitex_font_sidebearings(XeTeXFont font, uint32_t glyph, float *left, float *right) { reinterpret_cast<XeTeXFontInst *>(font)->getGlyphSidebearings(glyph, left, right); }
float pitex_font_italic_correction(XeTeXFont font, uint32_t glyph) { return reinterpret_cast<XeTeXFontInst *>(font)->getGlyphItalCorr(glyph); }
uint32_t pitex_font_map_character(XeTeXFont font, uint32_t character) { return reinterpret_cast<XeTeXFontInst *>(font)->mapCharToGlyph(character); }
int pitex_font_map_glyph(XeTeXFont font, const char *name) { return reinterpret_cast<XeTeXFontInst *>(font)->mapGlyphToIndex(name); }
int pitex_font_character_range(XeTeXFont font, bool first) { auto *instance = reinterpret_cast<XeTeXFontInst *>(font); return first ? instance->getFirstCharCode() : instance->getLastCharCode(); }
}
