/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009, 2011 by Jonathan Kew

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

#ifndef XETEX_MAC
#include "tectonic_bridge_core.h"
#include "xetex-XeTeXFontMgr_FC.h"
#include <ft2build.h>
#include FT_FREETYPE_H
#include <unicode/ucnv.h>
#include <stddef.h>
#ifndef FC_FULLNAME
#define FC_FULLNAME "fullname"
#endif
extern FT_Library gFreeTypeLibrary;
static UConverter* macRomanConv = NULL;

struct PitexFcNameList { const char* const* values; size_t count; };
struct PitexFcNames {
    const char* postscript;
    PitexFcNameList full, family, style;
};
extern "C" {
void pitex_fc_read_names(FcPattern*, bool, void*, void (*)(void*, const PitexFcNames*));
void pitex_fc_search(void*, const FcFontSet*, bool*, const char*, bool,
                     bool (*)(void*, FcPattern*), void (*)(void*, FcPattern*, const PitexFcNames*));
void pitex_fc_style(FcPattern*, uint16_t*, uint16_t*, int16_t*);
// FreeType's flag macro is an opaque ABI field accessor, not a name algorithm.
bool pitex_fc_face_is_sfnt(FT_Face face) { return FT_IS_SFNT(face); }
}
template<class Names>
static void copy_names(Names* names, const PitexFcNames* view)
{
    names->m_psName = view->postscript;
    names->m_fullNames.clear();
    if (view->full.count)
        names->m_fullNames.assign(view->full.values, view->full.values + view->full.count);
    names->m_familyNames.clear();
    if (view->family.count)
        names->m_familyNames.assign(view->family.values, view->family.values + view->family.count);
    names->m_styleNames.clear();
    if (view->style.count)
        names->m_styleNames.assign(view->style.values, view->style.values + view->style.count);
}
XeTeXFontMgr::NameCollection*
XeTeXFontMgr_FC::readNames(FcPattern* pat)
{
    NameCollection* names = new NameCollection;
    pitex_fc_read_names(pat, macRomanConv != NULL, names,
        [](void* output, const PitexFcNames* view) { copy_names(static_cast<NameCollection*>(output), view); });
    return names;
}
void
XeTeXFontMgr_FC::getOpSizeRecAndStyleFlags(Font* font)
{
    XeTeXFontMgr::getOpSizeRecAndStyleFlags(font);
    pitex_fc_style(font->fontRef, &font->weight, &font->width, &font->slant);
}
void
XeTeXFontMgr_FC::searchForHostPlatformFonts(const std::string& name)
{
    pitex_fc_search(this, allFonts, &cachedAll, name.c_str(), macRomanConv != NULL,
        [](void* manager, FcPattern* pattern) {
            auto* mgr = static_cast<XeTeXFontMgr_FC*>(manager);
            return mgr->m_platformRefToFont.find(pattern) != mgr->m_platformRefToFont.end();
        },
        [](void* manager, FcPattern* pattern, const PitexFcNames* view) {
            NameCollection names;
            copy_names(&names, view);
            static_cast<XeTeXFontMgr_FC*>(manager)->addToMaps(pattern, &names);
        });
}
void
XeTeXFontMgr_FC::initialize()
{
    if (FcInit() == FcFalse) _tt_abort("fontconfig initialization failed");
    if (gFreeTypeLibrary == 0 && FT_Init_FreeType(&gFreeTypeLibrary) != 0)
        _tt_abort("FreeType initialization failed");
    UErrorCode err = U_ZERO_ERROR;
    /* Retain upstream optional-converter availability behavior on Alpine's
     * split ICU builds. Decoding and name ordering are implemented in Rust. */
    macRomanConv = ucnv_open("macintosh", &err);
    if (!U_SUCCESS(err)) macRomanConv = NULL;
    FcPattern* pat = FcNameParse((const FcChar8*)":outline=true");
    FcObjectSet* os = FcObjectSetBuild(FC_FAMILY, FC_STYLE, FC_FILE, FC_INDEX,
        FC_FULLNAME, FC_WEIGHT, FC_WIDTH, FC_SLANT, FC_FONTFORMAT, NULL);
    allFonts = FcFontList(FcConfigGetCurrent(), pat, os);
    FcObjectSetDestroy(os);
    FcPatternDestroy(pat);
    cachedAll = false;
}
void
XeTeXFontMgr_FC::terminate()
{
    if (allFonts != NULL) { FcFontSetDestroy(allFonts); allFonts = NULL; }
    if (macRomanConv != NULL) { ucnv_close(macRomanConv); macRomanConv = NULL; }
}
std::string
XeTeXFontMgr_FC::getPlatformFontDesc(PlatformFontRef font) const
{
    FcChar8* path;
    return FcPatternGetString(font, FC_FILE, 0, &path) == FcResultMatch
        ? reinterpret_cast<char*>(path) : "[unknown]";
}
#endif
