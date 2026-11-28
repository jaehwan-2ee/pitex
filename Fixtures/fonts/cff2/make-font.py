"""Writes PitexCFF2Test.otf, a tiny variable CFF2 font (axis wght 100-900, default 100),
and PitexCFF2Test-Default.otf, its default instance as a static CFF font.

The outlines are generated here (a stem and a bar per glyph), so the fonts carry
no third-party design. Rerun: python3 make-font.py (needs fontTools).
"""
import sys
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.t2CharStringPen import T2CharStringPen
from fontTools.designspaceLib import DesignSpaceDocument, AxisDescriptor, SourceDescriptor
from fontTools import varLib

letters = [chr(c) for c in range(65, 91)] + [chr(c) for c in range(97, 123)]
names = [".notdef", "space", "period"] + letters

def glyph(name, stem):
    pen = T2CharStringPen(600, None)
    if name == "space":
        return pen.getCharString()
    if name == "period":
        pen.moveTo((250, 0)); pen.lineTo((250 + stem, 0)); pen.lineTo((250 + stem, stem)); pen.lineTo((250, stem)); pen.closePath()
        return pen.getCharString()
    i = names.index(name)
    top = 700 if name.isupper() or name == ".notdef" else 500
    # A vertical stem plus a bar whose height encodes the glyph, so every glyph differs.
    pen.moveTo((80, 0)); pen.lineTo((80 + stem, 0)); pen.lineTo((80 + stem, top)); pen.lineTo((80, top)); pen.closePath()
    y = 20 + (i * 13) % (top - 60)
    pen.moveTo((80 + stem, y)); pen.lineTo((520, y)); pen.lineTo((520, y + 40)); pen.lineTo((80 + stem, y + 40)); pen.closePath()
    return pen.getCharString()

def master(stem, path):
    fb = FontBuilder(1000, isTTF=False)
    fb.setupHead(unitsPerEm=1000, created=0, modified=0)
    fb.setupGlyphOrder(names)
    fb.setupCharacterMap({ord(c): c for c in letters} | {32: "space", 46: "period"})
    fb.setupCFF("PitexCFF2Test", {"FullName": "Pitex CFF2 Test"}, {n: glyph(n, stem) for n in names}, {})
    fb.setupHorizontalMetrics({n: (600, 80) for n in names})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({"familyName": "Pitex CFF2 Test", "styleName": "Regular", "psName": "PitexCFF2Test-Regular"})
    fb.setupOS2(sTypoAscender=800, usWinAscent=800, usWinDescent=200)
    fb.setupPost()
    fb.save(path)

import os, tempfile
here = os.path.dirname(os.path.abspath(__file__))
tmp = tempfile.mkdtemp()
out = os.path.join(tmp, "v")
master(60, os.path.join(here, "PitexCFF2Test-Default.otf"))
master(60, out + ".light.otf"); master(200, out + ".bold.otf")
doc = DesignSpaceDocument()
axis = AxisDescriptor(); axis.tag = "wght"; axis.name = "Weight"; axis.minimum, axis.default, axis.maximum = 100, 100, 900
doc.addAxis(axis)
for loc, path in [(100, ".light.otf"), (900, ".bold.otf")]:
    s = SourceDescriptor(); s.path = out + path; s.location = {"Weight": loc}; doc.addSource(s)
font, _, _ = varLib.build(doc)
font["head"].created = font["head"].modified = 0
font.save(os.path.join(here, "PitexCFF2Test.otf"))
