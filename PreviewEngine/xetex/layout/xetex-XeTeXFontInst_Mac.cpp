/****************************************************************************\
 Part of the XeTeX typesetting system
 Copyright (c) 1994-2008 by SIL International
 Copyright (c) 2009 by Jonathan Kew
 Copyright (c) 2012, 2013 by Jiang Jiang
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

// Only the CoreText-backed C++ subclass lifetime remains here. CoreFoundation
// descriptor creation, filename/TTC discovery and metrics initialization are Rust.
#ifdef XETEX_MAC
#include "tectonic_bridge_core.h"
#include "xetex-XeTeXFontInst_Mac.h"
#include "pitex_font_freetype.h"
XeTeXFontInst_Mac::XeTeXFontInst_Mac(CTFontDescriptorRef descriptor, float size, int &status)
    : XeTeXFontInst(nullptr, 0, size, status), m_descriptor(descriptor), m_fontRef(nullptr) {
    initialize(status);
}
XeTeXFontInst_Mac::~XeTeXFontInst_Mac() {
    if (m_descriptor) CFRelease(m_descriptor);
    if (m_fontRef) CFRelease(m_fontRef);
}
void XeTeXFontInst_Mac::initialize(int &status) {
    PitexMacFontInit result{};
    pitex_mac_font_initialize(m_descriptor, m_pointSize, &result, &status);
    m_descriptor = static_cast<CTFontDescriptorRef>(result.descriptor);
    m_fontRef = static_cast<CTFontRef>(result.font);
    if (result.path) {
        XeTeXFontInst::initialize(result.path, result.index, status);
        free(result.path);
    }
}
#endif
