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
// Translated from xetex/engine/xetex-macos.c with C2Rust 0.22.1.
use core::ffi::{c_int, c_void};

extern "C" {
    static __CFConstantStringClassReference: [c_int; 0];
}

// Matches Clang's __NSConstantString_tag for macOS arm64 and x86_64.
#[repr(C)]
struct CFConstantString {
    isa: *const c_int,
    flags: c_int,
    padding: c_int,
    bytes: *const u8,
    length: isize,
}

#[link_section = "__TEXT,__cstring,cstring_literals"]
static XE_BYTES: [u8; 14] = *b"XeTeXEmbolden\0";
#[link_section = "__TEXT,__cstring,cstring_literals"]
static LAST_RESORT_BYTES: [u8; 11] = *b"LastResort\0";

#[link_section = "__DATA,__cfstring"]
static mut XE_CFSTRING: CFConstantString = CFConstantString {
    isa: &raw const __CFConstantStringClassReference as *const c_int,
    flags: 1992,
    padding: 0,
    bytes: XE_BYTES.as_ptr(),
    length: 13,
};
#[link_section = "__DATA,__cfstring"]
static mut LAST_RESORT_CFSTRING: CFConstantString = CFConstantString {
    isa: &raw const __CFConstantStringClassReference as *const c_int,
    flags: 1992,
    padding: 0,
    bytes: LAST_RESORT_BYTES.as_ptr(),
    length: 10,
};


extern "C" {
    pub type __CFString;
    pub type __CFAllocator;
    pub type __CFArray;
    pub type __CFDictionary;
    pub type __CFBoolean;
    pub type __CFNumber;
    pub type __CFAttributedString;
    pub type CGColor;
    pub type CGFont;
    pub type __CTFontDescriptor;
    pub type __CTFont;
    pub type __CTLine;
    pub type __CTRun;
    pub type __CTTypesetter;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn _tt_abort(format: *const ::core::ffi::c_char, ...) -> !;
    static kCFAllocatorDefault: CFAllocatorRef;
    static kCFAllocatorNull: CFAllocatorRef;
    fn CFRelease(cf: CFTypeRef);
    fn CFEqual(cf1: CFTypeRef, cf2: CFTypeRef) -> Boolean;
    static kCFTypeArrayCallBacks: CFArrayCallBacks;
    fn CFArrayCreateMutable(
        allocator: CFAllocatorRef,
        capacity: CFIndex,
        callBacks: *const CFArrayCallBacks,
    ) -> CFMutableArrayRef;
    fn CFArrayGetCount(theArray: CFArrayRef) -> CFIndex;
    fn CFArrayGetValueAtIndex(theArray: CFArrayRef, idx: CFIndex) -> *const ::core::ffi::c_void;
    fn CFArrayAppendValue(theArray: CFMutableArrayRef, value: *const ::core::ffi::c_void);
    static kCFTypeDictionaryKeyCallBacks: CFDictionaryKeyCallBacks;
    static kCFTypeDictionaryValueCallBacks: CFDictionaryValueCallBacks;
    fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *mut *const ::core::ffi::c_void,
        values: *mut *const ::core::ffi::c_void,
        numValues: CFIndex,
        keyCallBacks: *const CFDictionaryKeyCallBacks,
        valueCallBacks: *const CFDictionaryValueCallBacks,
    ) -> CFDictionaryRef;
    fn CFDictionaryCreateMutable(
        allocator: CFAllocatorRef,
        capacity: CFIndex,
        keyCallBacks: *const CFDictionaryKeyCallBacks,
        valueCallBacks: *const CFDictionaryValueCallBacks,
    ) -> CFMutableDictionaryRef;
    fn CFDictionaryGetValue(
        theDict: CFDictionaryRef,
        key: *const ::core::ffi::c_void,
    ) -> *const ::core::ffi::c_void;
    fn CFDictionaryAddValue(
        theDict: CFMutableDictionaryRef,
        key: *const ::core::ffi::c_void,
        value: *const ::core::ffi::c_void,
    );
    static kCFBooleanTrue: CFBooleanRef;
    fn CFNumberCreate(
        allocator: CFAllocatorRef,
        theType: CFNumberType,
        valuePtr: *const ::core::ffi::c_void,
    ) -> CFNumberRef;
    fn CFNumberGetValue(
        number: CFNumberRef,
        theType: CFNumberType,
        valuePtr: *mut ::core::ffi::c_void,
    ) -> Boolean;
    fn CFNumberCompare(
        number: CFNumberRef,
        otherNumber: CFNumberRef,
        context: *mut ::core::ffi::c_void,
    ) -> CFComparisonResult;
    fn CFStringCreateWithBytes(
        alloc: CFAllocatorRef,
        bytes: *const UInt8,
        numBytes: CFIndex,
        encoding: CFStringEncoding,
        isExternalRepresentation: Boolean,
    ) -> CFStringRef;
    fn CFStringCreateWithCStringNoCopy(
        alloc: CFAllocatorRef,
        cStr: *const ::core::ffi::c_char,
        encoding: CFStringEncoding,
        contentsDeallocator: CFAllocatorRef,
    ) -> CFStringRef;
    fn CFStringCreateWithCharactersNoCopy(
        alloc: CFAllocatorRef,
        chars: *const UniChar,
        numChars: CFIndex,
        contentsDeallocator: CFAllocatorRef,
    ) -> CFStringRef;
    fn CFStringGetCString(
        theString: CFStringRef,
        buffer: *mut ::core::ffi::c_char,
        bufferSize: CFIndex,
        encoding: CFStringEncoding,
    ) -> Boolean;
    fn CFStringCompare(
        theString1: CFStringRef,
        theString2: CFStringRef,
        compareOptions: CFStringCompareFlags,
    ) -> CFComparisonResult;
    fn CFAttributedStringCreate(
        alloc: CFAllocatorRef,
        str: CFStringRef,
        attributes: CFDictionaryRef,
    ) -> CFAttributedStringRef;
    fn CGRectIsNull(rect: CGRect) -> bool;
    static CGAffineTransformIdentity: CGAffineTransform;
    fn CGColorCreateGenericRGB(
        red: CGFloat,
        green: CGFloat,
        blue: CGFloat,
        alpha: CGFloat,
    ) -> CGColorRef;
    fn CGColorRelease(color: CGColorRef);
    fn CGFontRelease(font: CGFontRef);
    fn CGFontGetNumberOfGlyphs(font: CGFontRef) -> size_t;
    fn CGFontCopyGlyphNameForGlyph(font: CGFontRef, glyph: CGGlyph) -> CFStringRef;
    static kCTFontCascadeListAttribute: CFStringRef;
    static kCTFontFeatureSettingsAttribute: CFStringRef;
    static kCTFontOrientationAttribute: CFStringRef;
    fn CTFontDescriptorCreateWithNameAndSize(
        name: CFStringRef,
        size: CGFloat,
    ) -> CTFontDescriptorRef;
    fn CTFontDescriptorCreateWithAttributes(attributes: CFDictionaryRef) -> CTFontDescriptorRef;
    fn CTFontCreateWithFontDescriptor(
        descriptor: CTFontDescriptorRef,
        size: CGFloat,
        matrix: *const CGAffineTransform,
    ) -> CTFontRef;
    fn CTFontCreateCopyWithAttributes(
        font: CTFontRef,
        size: CGFloat,
        matrix: *const CGAffineTransform,
        attributes: CTFontDescriptorRef,
    ) -> CTFontRef;
    fn CTFontGetGlyphsForCharacters(
        font: CTFontRef,
        characters: *const UniChar,
        glyphs: *mut CGGlyph,
        count: CFIndex,
    ) -> bool;
    fn CTFontGetGlyphWithName(font: CTFontRef, glyphName: CFStringRef) -> CGGlyph;
    fn CTFontGetBoundingRectsForGlyphs(
        font: CTFontRef,
        orientation: CTFontOrientation,
        glyphs: *const CGGlyph,
        boundingRects: *mut CGRect,
        count: CFIndex,
    ) -> CGRect;
    fn CTFontGetAdvancesForGlyphs(
        font: CTFontRef,
        orientation: CTFontOrientation,
        glyphs: *const CGGlyph,
        advances: *mut CGSize,
        count: CFIndex,
    ) -> ::core::ffi::c_double;
    static kCTFontFeatureTypeIdentifierKey: CFStringRef;
    static kCTFontFeatureTypeNameKey: CFStringRef;
    static kCTFontFeatureTypeSelectorsKey: CFStringRef;
    static kCTFontFeatureSelectorIdentifierKey: CFStringRef;
    static kCTFontFeatureSelectorNameKey: CFStringRef;
    fn CTFontCopyFeatures(font: CTFontRef) -> CFArrayRef;
    fn CTFontCopyGraphicsFont(font: CTFontRef, attributes: *mut CTFontDescriptorRef) -> CGFontRef;
    fn CTLineCreateJustifiedLine(
        line: CTLineRef,
        justificationFactor: CGFloat,
        justificationWidth: ::core::ffi::c_double,
    ) -> CTLineRef;
    fn CTLineGetGlyphCount(line: CTLineRef) -> CFIndex;
    fn CTLineGetGlyphRuns(line: CTLineRef) -> CFArrayRef;
    fn CTRunGetGlyphCount(run: CTRunRef) -> CFIndex;
    fn CTRunGetAttributes(run: CTRunRef) -> CFDictionaryRef;
    fn CTRunGetGlyphs(run: CTRunRef, range: CFRange, buffer: *mut CGGlyph);
    fn CTRunGetPositions(run: CTRunRef, range: CFRange, buffer: *mut CGPoint);
    fn CTRunGetAdvances(run: CTRunRef, range: CFRange, buffer: *mut CGSize);
    fn CTRunGetTypographicBounds(
        run: CTRunRef,
        range: CFRange,
        ascent: *mut CGFloat,
        descent: *mut CGFloat,
        leading: *mut CGFloat,
    ) -> ::core::ffi::c_double;
    fn CTTypesetterCreateWithAttributedString(string: CFAttributedStringRef) -> CTTypesetterRef;
    fn CTTypesetterCreateLine(typesetter: CTTypesetterRef, stringRange: CFRange) -> CTLineRef;
    static kCTFontAttributeName: CFStringRef;
    static kCTKernAttributeName: CFStringRef;
    static kCTForegroundColorAttributeName: CFStringRef;
    static kCTVerticalFormsAttributeName: CFStringRef;
    fn readCommonFeatures(
        feat: *const ::core::ffi::c_char,
        end: *const ::core::ffi::c_char,
        extend: *mut ::core::ffi::c_float,
        slant: *mut ::core::ffi::c_float,
        embolden: *mut ::core::ffi::c_float,
        letterspace: *mut ::core::ffi::c_float,
        rgbValue: *mut uint32_t,
    ) -> ::core::ffi::c_int;
    fn read_double(s: *mut *const ::core::ffi::c_char) -> ::core::ffi::c_double;
    fn Fix2D(f: Fixed) -> ::core::ffi::c_double;
    fn D2Fix(d: ::core::ffi::c_double) -> Fixed;
    static mut font_area: *mut str_number;
    static mut font_layout_engine: *mut *mut ::core::ffi::c_void;
    static mut font_letter_space: *mut scaled_t;
    static mut loaded_font_flags: ::core::ffi::c_char;
    static mut loaded_font_letter_space: scaled_t;
    static mut native_font_type_flag: int32_t;
    fn font_feature_warning(
        featureNameP: *const ::core::ffi::c_void,
        featLen: int32_t,
        settingNameP: *const ::core::ffi::c_void,
        setLen: int32_t,
    );
}
pub type __darwin_size_t = usize;
pub type size_t = __darwin_size_t;
pub type int32_t = i32;
pub type uint16_t = u16;
pub type uint32_t = u32;
pub type scaled_t = int32_t;
pub type UInt8 = ::core::ffi::c_uchar;
pub type UInt16 = ::core::ffi::c_ushort;
pub type UInt32 = ::core::ffi::c_uint;
pub type SInt32 = ::core::ffi::c_int;
pub type Fixed = SInt32;
pub type Fract = SInt32;
pub type Boolean = ::core::ffi::c_uchar;
pub type UniChar = UInt16;
#[derive(Copy, Clone)]
#[repr(C, packed(2))]
pub struct FixedPoint {
    pub x: Fixed,
    pub y: Fixed,
}
pub type CFOptionFlags = ::core::ffi::c_ulong;
pub type CFHashCode = ::core::ffi::c_ulong;
pub type CFIndex = ::core::ffi::c_long;
pub type CFTypeRef = *const ::core::ffi::c_void;
pub type CFStringRef = *const __CFString;
pub type CFComparisonResult = CFIndex;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CFRange {
    pub location: CFIndex,
    pub length: CFIndex,
}
pub type CFAllocatorRef = *const __CFAllocator;
pub type CFArrayRetainCallBack = Option<
    unsafe extern "C" fn(CFAllocatorRef, *const ::core::ffi::c_void) -> *const ::core::ffi::c_void,
>;
pub type CFArrayReleaseCallBack =
    Option<unsafe extern "C" fn(CFAllocatorRef, *const ::core::ffi::c_void) -> ()>;
pub type CFArrayCopyDescriptionCallBack =
    Option<unsafe extern "C" fn(*const ::core::ffi::c_void) -> CFStringRef>;
pub type CFArrayEqualCallBack =
    Option<unsafe extern "C" fn(*const ::core::ffi::c_void, *const ::core::ffi::c_void) -> Boolean>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CFArrayCallBacks {
    pub version: CFIndex,
    pub retain: CFArrayRetainCallBack,
    pub release: CFArrayReleaseCallBack,
    pub copyDescription: CFArrayCopyDescriptionCallBack,
    pub equal: CFArrayEqualCallBack,
}
pub type CFArrayRef = *const __CFArray;
pub type CFMutableArrayRef = *mut __CFArray;
pub type CFDictionaryRetainCallBack = Option<
    unsafe extern "C" fn(CFAllocatorRef, *const ::core::ffi::c_void) -> *const ::core::ffi::c_void,
>;
pub type CFDictionaryReleaseCallBack =
    Option<unsafe extern "C" fn(CFAllocatorRef, *const ::core::ffi::c_void) -> ()>;
pub type CFDictionaryCopyDescriptionCallBack =
    Option<unsafe extern "C" fn(*const ::core::ffi::c_void) -> CFStringRef>;
pub type CFDictionaryEqualCallBack =
    Option<unsafe extern "C" fn(*const ::core::ffi::c_void, *const ::core::ffi::c_void) -> Boolean>;
pub type CFDictionaryHashCallBack =
    Option<unsafe extern "C" fn(*const ::core::ffi::c_void) -> CFHashCode>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CFDictionaryKeyCallBacks {
    pub version: CFIndex,
    pub retain: CFDictionaryRetainCallBack,
    pub release: CFDictionaryReleaseCallBack,
    pub copyDescription: CFDictionaryCopyDescriptionCallBack,
    pub equal: CFDictionaryEqualCallBack,
    pub hash: CFDictionaryHashCallBack,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CFDictionaryValueCallBacks {
    pub version: CFIndex,
    pub retain: CFDictionaryRetainCallBack,
    pub release: CFDictionaryReleaseCallBack,
    pub copyDescription: CFDictionaryCopyDescriptionCallBack,
    pub equal: CFDictionaryEqualCallBack,
}
pub type CFDictionaryRef = *const __CFDictionary;
pub type CFMutableDictionaryRef = *mut __CFDictionary;
pub type CFBooleanRef = *const __CFBoolean;
pub type CFNumberType = CFIndex;
pub type C2RustUnnamed = ::core::ffi::c_uint;
pub const kCFNumberMaxType: C2RustUnnamed = 16;
pub const kCFNumberCGFloatType: C2RustUnnamed = 16;
pub const kCFNumberNSIntegerType: C2RustUnnamed = 15;
pub const kCFNumberCFIndexType: C2RustUnnamed = 14;
pub const kCFNumberDoubleType: C2RustUnnamed = 13;
pub const kCFNumberFloatType: C2RustUnnamed = 12;
pub const kCFNumberLongLongType: C2RustUnnamed = 11;
pub const kCFNumberLongType: C2RustUnnamed = 10;
pub const kCFNumberIntType: C2RustUnnamed = 9;
pub const kCFNumberShortType: C2RustUnnamed = 8;
pub const kCFNumberCharType: C2RustUnnamed = 7;
pub const kCFNumberFloat64Type: C2RustUnnamed = 6;
pub const kCFNumberFloat32Type: C2RustUnnamed = 5;
pub const kCFNumberSInt64Type: C2RustUnnamed = 4;
pub const kCFNumberSInt32Type: C2RustUnnamed = 3;
pub const kCFNumberSInt16Type: C2RustUnnamed = 2;
pub const kCFNumberSInt8Type: C2RustUnnamed = 1;
pub type CFNumberRef = *const __CFNumber;
pub type CFStringEncoding = UInt32;
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const kCFStringEncodingUTF32LE: C2RustUnnamed_0 = 469762304;
pub const kCFStringEncodingUTF32BE: C2RustUnnamed_0 = 402653440;
pub const kCFStringEncodingUTF32: C2RustUnnamed_0 = 201326848;
pub const kCFStringEncodingUTF16LE: C2RustUnnamed_0 = 335544576;
pub const kCFStringEncodingUTF16BE: C2RustUnnamed_0 = 268435712;
pub const kCFStringEncodingUTF16: C2RustUnnamed_0 = 256;
pub const kCFStringEncodingNonLossyASCII: C2RustUnnamed_0 = 3071;
pub const kCFStringEncodingUTF8: C2RustUnnamed_0 = 134217984;
pub const kCFStringEncodingUnicode: C2RustUnnamed_0 = 256;
pub const kCFStringEncodingASCII: C2RustUnnamed_0 = 1536;
pub const kCFStringEncodingNextStepLatin: C2RustUnnamed_0 = 2817;
pub const kCFStringEncodingISOLatin1: C2RustUnnamed_0 = 513;
pub const kCFStringEncodingWindowsLatin1: C2RustUnnamed_0 = 1280;
pub const kCFStringEncodingMacRoman: C2RustUnnamed_0 = 0;
pub type CFStringCompareFlags = CFOptionFlags;
pub type C2RustUnnamed_1 = ::core::ffi::c_uint;
pub const kCFCompareForcedOrdering: C2RustUnnamed_1 = 512;
pub const kCFCompareWidthInsensitive: C2RustUnnamed_1 = 256;
pub const kCFCompareDiacriticInsensitive: C2RustUnnamed_1 = 128;
pub const kCFCompareNumerically: C2RustUnnamed_1 = 64;
pub const kCFCompareLocalized: C2RustUnnamed_1 = 32;
pub const kCFCompareNonliteral: C2RustUnnamed_1 = 16;
pub const kCFCompareAnchored: C2RustUnnamed_1 = 8;
pub const kCFCompareBackwards: C2RustUnnamed_1 = 4;
pub const kCFCompareCaseInsensitive: C2RustUnnamed_1 = 1;
pub type CFAttributedStringRef = *const __CFAttributedString;
pub type CGFloat = ::core::ffi::c_double;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CGPoint {
    pub x: CGFloat,
    pub y: CGFloat,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CGSize {
    pub width: CGFloat,
    pub height: CGFloat,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CGRect {
    pub origin: CGPoint,
    pub size: CGSize,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct CGAffineTransform {
    pub a: CGFloat,
    pub b: CGFloat,
    pub c: CGFloat,
    pub d: CGFloat,
    pub tx: CGFloat,
    pub ty: CGFloat,
}
pub type CGColorRef = *mut CGColor;
pub type CGFontRef = *mut CGFont;
pub type CGFontIndex = ::core::ffi::c_ushort;
pub type CGGlyph = CGFontIndex;
pub type CTFontDescriptorRef = *const __CTFontDescriptor;
pub type CTFontOrientation = uint32_t;
pub type C2RustUnnamed_2 = ::core::ffi::c_uint;
pub const kCTFontVerticalOrientation: C2RustUnnamed_2 = 2;
pub const kCTFontHorizontalOrientation: C2RustUnnamed_2 = 1;
pub const kCTFontDefaultOrientation: C2RustUnnamed_2 = 0;
pub const kCTFontOrientationVertical: C2RustUnnamed_2 = 2;
pub const kCTFontOrientationHorizontal: C2RustUnnamed_2 = 1;
pub const kCTFontOrientationDefault: C2RustUnnamed_2 = 0;
pub type CTFontRef = *const __CTFont;
pub type CTLineRef = *const __CTLine;
pub type CTRunRef = *const __CTRun;
pub type CTTypesetterRef = *const __CTTypesetter;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct GlyphBBox {
    pub xMin: ::core::ffi::c_float,
    pub yMin: ::core::ffi::c_float,
    pub xMax: ::core::ffi::c_float,
    pub yMax: ::core::ffi::c_float,
}
pub type b32x2 = b32x2_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b32x2_le_t {
    pub s0: int32_t,
    pub s1: int32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub union memory_word {
    pub b32: b32x2,
    pub b16: b16x4,
    pub gr: ::core::ffi::c_double,
    pub ptr: *mut ::core::ffi::c_void,
}
pub type b16x4 = b16x4_le_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct b16x4_le_t {
    pub s0: uint16_t,
    pub s1: uint16_t,
    pub s2: uint16_t,
    pub s3: uint16_t,
}
pub type str_number = int32_t;
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
pub const fract1: Fract = 0x40000000 as ::core::ffi::c_long as Fract;
#[inline]
unsafe extern "C" fn strstartswith(
    mut s: *const ::core::ffi::c_char,
    mut prefix: *const ::core::ffi::c_char,
) -> *const ::core::ffi::c_char {
    let mut length: size_t = 0;
    length = strlen(prefix);
    if strncmp(s, prefix, length) == 0 as ::core::ffi::c_int {
        return s.offset(length as isize);
    }
    return ::core::ptr::null::<::core::ffi::c_char>();
}
pub const FONT_FLAGS_COLORED: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
#[inline(always)]
unsafe extern "C" fn CFRangeMake(mut loc: CFIndex, mut len: CFIndex) -> CFRange {
    let mut range: CFRange = CFRange {
        location: 0,
        length: 0,
    };
    range.location = loc;
    range.length = len;
    return range;
}
#[inline]
unsafe extern "C" fn CGSizeMake(mut width: CGFloat, mut height: CGFloat) -> CGSize {
    let mut size: CGSize = CGSize {
        width: 0.,
        height: 0.,
    };
    size.width = width;
    size.height = height;
    return size;
}
#[inline]
unsafe extern "C" fn __CGAffineTransformMake(
    mut a: CGFloat,
    mut b: CGFloat,
    mut c: CGFloat,
    mut d: CGFloat,
    mut tx: CGFloat,
    mut ty: CGFloat,
) -> CGAffineTransform {
    let mut t: CGAffineTransform = CGAffineTransform {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        tx: 0.,
        ty: 0.,
    };
    t.a = a;
    t.b = b;
    t.c = c;
    t.d = d;
    t.tx = tx;
    t.ty = ty;
    return t;
}
pub const NATIVE_NODE_SIZE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
pub const AAT_FONT_FLAG: ::core::ffi::c_uint = 0xffff as ::core::ffi::c_uint;
pub const width_offset: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const native_info_offset: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const native_glyph_info_offset: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const native_glyph_info_size: ::core::ffi::c_int = 10 as ::core::ffi::c_int;
#[inline]
unsafe extern "C" fn TeXtoPSPoints(mut pts: ::core::ffi::c_double) -> ::core::ffi::c_double {
    return pts * 72.0f64 / 72.27f64;
}
#[inline]
unsafe extern "C" fn PStoTeXPoints(mut pts: ::core::ffi::c_double) -> ::core::ffi::c_double {
    return pts * 72.27f64 / 72.0f64;
}
#[inline]
unsafe extern "C" fn FixedPStoTeXPoints(mut pts: ::core::ffi::c_double) -> Fixed {
    return D2Fix(PStoTeXPoints(pts));
}
#[no_mangle]
pub unsafe extern "C" fn fontFromAttributes(mut attributes: CFDictionaryRef) -> CTFontRef {
    return CFDictionaryGetValue(
        attributes,
        kCTFontAttributeName as *const ::core::ffi::c_void,
    ) as CTFontRef;
}
#[no_mangle]
pub unsafe extern "C" fn fontFromInteger(mut font: int32_t) -> CTFontRef {
    let mut attributes: CFDictionaryRef =
        *font_layout_engine.offset(font as isize) as CFDictionaryRef;
    return fontFromAttributes(attributes);
}
#[no_mangle]
pub unsafe extern "C" fn DoAATLayout(
    mut p: *mut ::core::ffi::c_void,
    mut justify: ::core::ffi::c_int,
) {
    let mut glyphRuns: CFArrayRef = ::core::ptr::null::<__CFArray>();
    let mut i: CFIndex = 0;
    let mut j: CFIndex = 0;
    let mut runCount: CFIndex = 0;
    let mut totalGlyphCount: CFIndex = 0 as CFIndex;
    let mut glyphIDs: *mut UInt16 = ::core::ptr::null_mut::<UInt16>();
    let mut glyphAdvances: *mut Fixed = ::core::ptr::null_mut::<Fixed>();
    let mut glyph_info: *mut ::core::ffi::c_void = NULL;
    let mut locations: *mut FixedPoint = ::core::ptr::null_mut::<FixedPoint>();
    let mut width: CGFloat = 0.;
    let mut txtLen: ::core::ffi::c_long = 0;
    let mut txtPtr: *const UniChar = ::core::ptr::null::<UniChar>();
    let mut attributes: CFDictionaryRef = ::core::ptr::null::<__CFDictionary>();
    let mut string: CFStringRef = ::core::ptr::null::<__CFString>();
    let mut attrString: CFAttributedStringRef = ::core::ptr::null::<__CFAttributedString>();
    let mut typesetter: CTTypesetterRef = ::core::ptr::null::<__CTTypesetter>();
    let mut line: CTLineRef = ::core::ptr::null::<__CTLine>();
    let mut node: *mut memory_word = p as *mut memory_word;
    let mut f: ::core::ffi::c_uint =
        (*node.offset(native_info_offset as isize)).b16.s2 as ::core::ffi::c_uint;
    if *font_area.offset(f as isize) as ::core::ffi::c_uint != AAT_FONT_FLAG {
        _tt_abort(
            b"DoAATLayout called for non-AAT font\0" as *const u8 as *const ::core::ffi::c_char,
        );
    }
    txtLen = (*node.offset(native_info_offset as isize)).b16.s1 as ::core::ffi::c_long;
    txtPtr = node.offset(NATIVE_NODE_SIZE as isize) as *mut UniChar;
    attributes = *font_layout_engine
        .offset((*node.offset(native_info_offset as isize)).b16.s2 as isize)
        as CFDictionaryRef;
    string = CFStringCreateWithCharactersNoCopy(
        ::core::ptr::null::<__CFAllocator>(),
        txtPtr,
        txtLen as CFIndex,
        kCFAllocatorNull,
    );
    attrString = CFAttributedStringCreate(::core::ptr::null::<__CFAllocator>(), string, attributes);
    CFRelease(string as CFTypeRef);
    typesetter = CTTypesetterCreateWithAttributedString(attrString);
    CFRelease(attrString as CFTypeRef);
    line = CTTypesetterCreateLine(typesetter, CFRangeMake(0 as CFIndex, txtLen as CFIndex));
    if justify != 0 {
        let mut lineWidth: CGFloat =
            TeXtoPSPoints(Fix2D((*node.offset(width_offset as isize)).b32.s1 as Fixed)) as CGFloat;
        let mut justifiedLine: CTLineRef = CTLineCreateJustifiedLine(
            line,
            TeXtoPSPoints(Fix2D(fract1)) as CGFloat,
            lineWidth as ::core::ffi::c_double,
        );
        if !justifiedLine.is_null() {
            CFRelease(line as CFTypeRef);
            line = justifiedLine;
        }
    }
    glyphRuns = CTLineGetGlyphRuns(line);
    runCount = CFArrayGetCount(glyphRuns);
    totalGlyphCount = CTLineGetGlyphCount(line);
    if totalGlyphCount > 0 as CFIndex {
        glyph_info = malloc((totalGlyphCount * native_glyph_info_size as CFIndex) as size_t);
        locations = glyph_info as *mut FixedPoint;
        glyphIDs = locations.offset(totalGlyphCount as isize) as *mut UInt16;
        glyphAdvances = malloc(
            (totalGlyphCount as size_t).wrapping_mul(::core::mem::size_of::<Fixed>() as size_t),
        ) as *mut Fixed;
        totalGlyphCount = 0 as CFIndex;
        width = 0 as ::core::ffi::c_int as CGFloat;
        i = 0 as CFIndex;
        while i < runCount {
            let mut run: CTRunRef = CFArrayGetValueAtIndex(glyphRuns, i) as CTRunRef;
            let mut count: CFIndex = CTRunGetGlyphCount(run);
            let mut runAttributes: CFDictionaryRef = CTRunGetAttributes(run);
            let mut vertical: CFBooleanRef = CFDictionaryGetValue(
                runAttributes,
                kCTVerticalFormsAttributeName as *const ::core::ffi::c_void,
            ) as CFBooleanRef;
            let mut glyphs: *mut CGGlyph =
                malloc((count as size_t).wrapping_mul(::core::mem::size_of::<CGGlyph>() as size_t))
                    as *mut CGGlyph;
            let mut positions: *mut CGPoint =
                malloc((count as size_t).wrapping_mul(::core::mem::size_of::<CGPoint>() as size_t))
                    as *mut CGPoint;
            let mut advances: *mut CGSize =
                malloc((count as size_t).wrapping_mul(::core::mem::size_of::<CGSize>() as size_t))
                    as *mut CGSize;
            let mut runWidth: CGFloat = CTRunGetTypographicBounds(
                run,
                CFRangeMake(0 as CFIndex, 0 as CFIndex),
                ::core::ptr::null_mut::<CGFloat>(),
                ::core::ptr::null_mut::<CGFloat>(),
                ::core::ptr::null_mut::<CGFloat>(),
            ) as CGFloat;
            CTRunGetGlyphs(
                run,
                CFRangeMake(0 as CFIndex, 0 as CFIndex),
                glyphs as *mut CGGlyph,
            );
            CTRunGetPositions(
                run,
                CFRangeMake(0 as CFIndex, 0 as CFIndex),
                positions as *mut CGPoint,
            );
            CTRunGetAdvances(
                run,
                CFRangeMake(0 as CFIndex, 0 as CFIndex),
                advances as *mut CGSize,
            );
            j = 0 as CFIndex;
            while j < count {
                if CFEqual(
                    fontFromAttributes(attributes) as CFTypeRef,
                    fontFromAttributes(runAttributes) as CFTypeRef,
                ) == 0
                {
                    *glyphIDs.offset(totalGlyphCount as isize) = 0 as UInt16;
                } else {
                    *glyphIDs.offset(totalGlyphCount as isize) =
                        *glyphs.offset(j as isize) as UInt16;
                }
                if vertical == kCFBooleanTrue {
                    (*locations.offset(totalGlyphCount as isize)).x = -FixedPStoTeXPoints(
                        (*positions.offset(j as isize)).y as ::core::ffi::c_double,
                    );
                    (*locations.offset(totalGlyphCount as isize)).y = FixedPStoTeXPoints(
                        (*positions.offset(j as isize)).x as ::core::ffi::c_double,
                    );
                } else {
                    (*locations.offset(totalGlyphCount as isize)).x = FixedPStoTeXPoints(
                        (*positions.offset(j as isize)).x as ::core::ffi::c_double,
                    );
                    (*locations.offset(totalGlyphCount as isize)).y = -FixedPStoTeXPoints(
                        (*positions.offset(j as isize)).y as ::core::ffi::c_double,
                    );
                }
                *glyphAdvances.offset(totalGlyphCount as isize) =
                    (*advances.offset(j as isize)).width as Fixed;
                totalGlyphCount += 1;
                j += 1;
            }
            width += FixedPStoTeXPoints(runWidth as ::core::ffi::c_double) as CGFloat;
            free(glyphs as *mut ::core::ffi::c_void);
            free(positions as *mut ::core::ffi::c_void);
            free(advances as *mut ::core::ffi::c_void);
            i += 1;
        }
    }
    (*node.offset(native_info_offset as isize)).b16.s0 = totalGlyphCount as uint16_t;
    let ref mut fresh0 = (*node.offset(native_glyph_info_offset as isize)).ptr;
    *fresh0 = glyph_info;
    if justify == 0 {
        (*node.offset(width_offset as isize)).b32.s1 = width as int32_t;
        if totalGlyphCount > 0 as CFIndex {
            if *font_letter_space.offset(f as isize) != 0 as scaled_t {
                let mut lsDelta: Fixed = 0 as Fixed;
                let mut lsUnit: Fixed = *font_letter_space.offset(f as isize) as Fixed;
                let mut i_0: ::core::ffi::c_int = 0;
                i_0 = 0 as ::core::ffi::c_int;
                while (i_0 as CFIndex) < totalGlyphCount {
                    if *glyphAdvances.offset(i_0 as isize) == 0 as ::core::ffi::c_int
                        && lsDelta != 0 as ::core::ffi::c_int
                    {
                        lsDelta -= lsUnit;
                    }
                    (*locations.offset(i_0 as isize)).x += lsDelta;
                    lsDelta += lsUnit;
                    i_0 += 1;
                }
                if lsDelta != 0 as ::core::ffi::c_int {
                    lsDelta -= lsUnit;
                    let ref mut fresh1 = (*node.offset(width_offset as isize)).b32.s1;
                    *fresh1 =
                        (*fresh1 as ::core::ffi::c_int + lsDelta as ::core::ffi::c_int) as int32_t;
                }
            }
        }
    }
    free(glyphAdvances as *mut ::core::ffi::c_void);
    CFRelease(line as CFTypeRef);
    CFRelease(typesetter as CFTypeRef);
}
unsafe extern "C" fn getGlyphBBoxFromCTFont(
    mut font: CTFontRef,
    mut gid: UInt16,
    mut bbox: *mut GlyphBBox,
) {
    let mut rect: CGRect = CGRect {
        origin: CGPoint { x: 0., y: 0. },
        size: CGSize {
            width: 0.,
            height: 0.,
        },
    };
    (*bbox).xMin = 65536.0f32;
    (*bbox).yMin = 65536.0f32;
    (*bbox).xMax = -65536.0f64 as ::core::ffi::c_float;
    (*bbox).yMax = -65536.0f64 as ::core::ffi::c_float;
    rect = CTFontGetBoundingRectsForGlyphs(
        font,
        0 as CTFontOrientation,
        &raw mut gid as *const CGGlyph,
        ::core::ptr::null_mut::<CGRect>(),
        1 as CFIndex,
    );
    if CGRectIsNull(rect) {
        (*bbox).yMax = 0 as ::core::ffi::c_int as ::core::ffi::c_float;
        (*bbox).xMax = (*bbox).yMax;
        (*bbox).yMin = (*bbox).xMax;
        (*bbox).xMin = (*bbox).yMin;
    } else {
        (*bbox).yMin =
            PStoTeXPoints(rect.origin.y as ::core::ffi::c_double) as ::core::ffi::c_float;
        (*bbox).yMax = PStoTeXPoints(
            rect.origin.y as ::core::ffi::c_double + rect.size.height as ::core::ffi::c_double,
        ) as ::core::ffi::c_float;
        (*bbox).xMin =
            PStoTeXPoints(rect.origin.x as ::core::ffi::c_double) as ::core::ffi::c_float;
        (*bbox).xMax = PStoTeXPoints(
            rect.origin.x as ::core::ffi::c_double + rect.size.width as ::core::ffi::c_double,
        ) as ::core::ffi::c_float;
    };
}
#[no_mangle]
pub unsafe extern "C" fn GetGlyphBBox_AAT(
    mut attributes: CFDictionaryRef,
    mut gid: UInt16,
    mut bbox: *mut GlyphBBox,
) {
    let mut font: CTFontRef = fontFromAttributes(attributes);
    return getGlyphBBoxFromCTFont(font, gid, bbox);
}
unsafe extern "C" fn getGlyphWidthFromCTFont(
    mut font: CTFontRef,
    mut gid: UInt16,
) -> ::core::ffi::c_double {
    return PStoTeXPoints(CTFontGetAdvancesForGlyphs(
        font,
        kCTFontOrientationHorizontal as ::core::ffi::c_int as CTFontOrientation,
        &raw mut gid as *const CGGlyph,
        ::core::ptr::null_mut::<CGSize>(),
        1 as CFIndex,
    ));
}
#[no_mangle]
pub unsafe extern "C" fn GetGlyphWidth_AAT(
    mut attributes: CFDictionaryRef,
    mut gid: UInt16,
) -> ::core::ffi::c_double {
    let mut font: CTFontRef = fontFromAttributes(attributes);
    return getGlyphWidthFromCTFont(font, gid);
}
#[no_mangle]
pub unsafe extern "C" fn GetGlyphHeightDepth_AAT(
    mut attributes: CFDictionaryRef,
    mut gid: UInt16,
    mut ht: *mut ::core::ffi::c_float,
    mut dp: *mut ::core::ffi::c_float,
) {
    let mut bbox: GlyphBBox = GlyphBBox {
        xMin: 0.,
        yMin: 0.,
        xMax: 0.,
        yMax: 0.,
    };
    GetGlyphBBox_AAT(attributes, gid, &raw mut bbox);
    *ht = bbox.yMax;
    *dp = -bbox.yMin;
}
#[no_mangle]
pub unsafe extern "C" fn GetGlyphSidebearings_AAT(
    mut attributes: CFDictionaryRef,
    mut gid: UInt16,
    mut lsb: *mut ::core::ffi::c_float,
    mut rsb: *mut ::core::ffi::c_float,
) {
    let mut font: CTFontRef = fontFromAttributes(attributes);
    let mut advances: [CGSize; 1] = [CGSizeMake(
        0 as ::core::ffi::c_int as CGFloat,
        0 as ::core::ffi::c_int as CGFloat,
    )];
    let mut advance: ::core::ffi::c_double = CTFontGetAdvancesForGlyphs(
        font,
        0 as CTFontOrientation,
        &raw mut gid as *const CGGlyph,
        &raw mut advances as *mut CGSize,
        1 as CFIndex,
    );
    let mut bbox: GlyphBBox = GlyphBBox {
        xMin: 0.,
        yMin: 0.,
        xMax: 0.,
        yMax: 0.,
    };
    getGlyphBBoxFromCTFont(font, gid, &raw mut bbox);
    *lsb = bbox.xMin;
    *rsb = (PStoTeXPoints(advance) - bbox.xMax as ::core::ffi::c_double) as ::core::ffi::c_float;
}
#[no_mangle]
pub unsafe extern "C" fn GetGlyphItalCorr_AAT(
    mut attributes: CFDictionaryRef,
    mut gid: UInt16,
) -> ::core::ffi::c_double {
    let mut font: CTFontRef = fontFromAttributes(attributes);
    let mut advances: [CGSize; 1] = [CGSizeMake(
        0 as ::core::ffi::c_int as CGFloat,
        0 as ::core::ffi::c_int as CGFloat,
    )];
    let mut advance: ::core::ffi::c_double = CTFontGetAdvancesForGlyphs(
        font,
        0 as CTFontOrientation,
        &raw mut gid as *const CGGlyph,
        &raw mut advances as *mut CGSize,
        1 as CFIndex,
    );
    let mut bbox: GlyphBBox = GlyphBBox {
        xMin: 0.,
        yMin: 0.,
        xMax: 0.,
        yMax: 0.,
    };
    getGlyphBBoxFromCTFont(font, gid, &raw mut bbox);
    if bbox.xMax as ::core::ffi::c_double > PStoTeXPoints(advance) {
        return bbox.xMax as ::core::ffi::c_double - PStoTeXPoints(advance);
    }
    return 0 as ::core::ffi::c_int as ::core::ffi::c_double;
}
unsafe extern "C" fn mapCharToGlyphFromCTFont(
    mut font: CTFontRef,
    mut ch: UInt32,
) -> ::core::ffi::c_int {
    let mut glyphs: [CGGlyph; 2] = [0 as ::core::ffi::c_int as CGGlyph, 0];
    let mut txt: [UniChar; 2] = [0; 2];
    let mut len: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    if ch > 0xffff as UInt32 {
        ch = ch.wrapping_sub(0x10000 as ::core::ffi::c_int as UInt32);
        txt[0 as ::core::ffi::c_int as usize] =
            (0xd800 as UInt32).wrapping_add(ch.wrapping_div(1024 as UInt32)) as UniChar;
        txt[1 as ::core::ffi::c_int as usize] =
            (0xdc00 as UInt32).wrapping_add(ch.wrapping_rem(1024 as UInt32)) as UniChar;
        len = 2 as ::core::ffi::c_int;
    } else {
        txt[0 as ::core::ffi::c_int as usize] = ch as UniChar;
    }
    if CTFontGetGlyphsForCharacters(
        font,
        &raw mut txt as *mut UniChar as *const UniChar,
        &raw mut glyphs as *mut CGGlyph,
        len as CFIndex,
    ) {
        return glyphs[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn MapCharToGlyph_AAT(
    mut attributes: CFDictionaryRef,
    mut ch: UInt32,
) -> ::core::ffi::c_int {
    let mut font: CTFontRef = fontFromAttributes(attributes);
    return mapCharToGlyphFromCTFont(font, ch);
}
unsafe extern "C" fn GetGlyphIDFromCTFont(
    mut ctFontRef: CTFontRef,
    mut glyphName: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut glyphname: CFStringRef = CFStringCreateWithCStringNoCopy(
        kCFAllocatorDefault,
        glyphName,
        kCFStringEncodingUTF8 as ::core::ffi::c_int as CFStringEncoding,
        kCFAllocatorNull,
    );
    let mut rval: ::core::ffi::c_int =
        CTFontGetGlyphWithName(ctFontRef, glyphname) as ::core::ffi::c_int;
    CFRelease(glyphname as CFTypeRef);
    return rval;
}
#[no_mangle]
pub unsafe extern "C" fn MapGlyphToIndex_AAT(
    mut attributes: CFDictionaryRef,
    mut glyphName: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    let mut font: CTFontRef = fontFromAttributes(attributes);
    return GetGlyphIDFromCTFont(font, glyphName);
}
#[no_mangle]
pub unsafe extern "C" fn GetGlyphNameFromCTFont(
    mut ctFontRef: CTFontRef,
    mut gid: UInt16,
    mut len: *mut ::core::ffi::c_int,
) -> *mut ::core::ffi::c_char {
    let mut cgfont: CGFontRef = ::core::ptr::null_mut::<CGFont>();
    static mut buffer: [::core::ffi::c_char; 256] = [0; 256];
    buffer[0 as ::core::ffi::c_int as usize] = 0 as ::core::ffi::c_char;
    *len = 0 as ::core::ffi::c_int;
    cgfont = CTFontCopyGraphicsFont(ctFontRef, ::core::ptr::null_mut::<CTFontDescriptorRef>());
    if !cgfont.is_null() && (gid as size_t) < CGFontGetNumberOfGlyphs(cgfont) {
        let mut glyphname: CFStringRef = CGFontCopyGlyphNameForGlyph(cgfont, gid as CGGlyph);
        if !glyphname.is_null() {
            if CFStringGetCString(
                glyphname,
                &raw mut buffer as *mut ::core::ffi::c_char,
                256 as CFIndex,
                kCFStringEncodingUTF8 as ::core::ffi::c_int as CFStringEncoding,
            ) != 0
            {
                *len = strlen(&raw mut buffer as *mut ::core::ffi::c_char) as ::core::ffi::c_int;
            }
            CFRelease(glyphname as CFTypeRef);
        }
        CGFontRelease(cgfont);
    }
    return (&raw mut buffer as *mut ::core::ffi::c_char).offset(0 as ::core::ffi::c_int as isize)
        as *mut ::core::ffi::c_char;
}
#[no_mangle]
pub unsafe extern "C" fn GetFontCharRange_AAT(
    mut attributes: CFDictionaryRef,
    mut reqFirst: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if reqFirst != 0 {
        let mut ch: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while MapCharToGlyph_AAT(attributes, ch as UInt32) == 0 as ::core::ffi::c_int
            && ch < 0x10ffff as ::core::ffi::c_int
        {
            ch += 1;
        }
        return ch;
    } else {
        let mut ch_0: ::core::ffi::c_int = 0x10ffff as ::core::ffi::c_int;
        while MapCharToGlyph_AAT(attributes, ch_0 as UInt32) == 0 as ::core::ffi::c_int
            && ch_0 > 0 as ::core::ffi::c_int
        {
            ch_0 -= 1;
        }
        return ch_0;
    };
}
#[no_mangle]
pub unsafe extern "C" fn findDictionaryInArrayWithIdentifier(
    mut array: CFArrayRef,
    mut identifierKey: *const ::core::ffi::c_void,
    mut identifier: ::core::ffi::c_int,
) -> CFDictionaryRef {
    let mut dict: CFDictionaryRef = ::core::ptr::null::<__CFDictionary>();
    if !array.is_null() {
        let mut value: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
        let mut i: CFIndex = 0;
        i = 0 as CFIndex;
        while i < CFArrayGetCount(array) {
            let mut item: CFDictionaryRef = CFArrayGetValueAtIndex(array, i) as CFDictionaryRef;
            let mut itemId: CFNumberRef = CFDictionaryGetValue(item, identifierKey) as CFNumberRef;
            if !itemId.is_null() {
                CFNumberGetValue(
                    itemId,
                    kCFNumberIntType as ::core::ffi::c_int as CFNumberType,
                    &raw mut value as *mut ::core::ffi::c_void,
                );
                if value == identifier {
                    dict = item;
                    break;
                }
            }
            i += 1;
        }
    }
    return dict;
}
#[no_mangle]
pub unsafe extern "C" fn findDictionaryInArray(
    mut array: CFArrayRef,
    mut nameKey: *const ::core::ffi::c_void,
    mut name: *const ::core::ffi::c_char,
    mut nameLength: ::core::ffi::c_int,
) -> CFDictionaryRef {
    let mut dict: CFDictionaryRef = ::core::ptr::null::<__CFDictionary>();
    if !array.is_null() {
        let mut itemName: CFStringRef = ::core::ptr::null::<__CFString>();
        let mut i: CFIndex = 0;
        itemName = CFStringCreateWithBytes(
            ::core::ptr::null::<__CFAllocator>(),
            name as *mut UInt8,
            nameLength as CFIndex,
            kCFStringEncodingUTF8 as ::core::ffi::c_int as CFStringEncoding,
            false_0 as Boolean,
        );
        i = 0 as CFIndex;
        while i < CFArrayGetCount(array) {
            let mut item: CFDictionaryRef = CFArrayGetValueAtIndex(array, i) as CFDictionaryRef;
            let mut iName: CFStringRef = CFDictionaryGetValue(item, nameKey) as CFStringRef;
            if !iName.is_null()
                && CFStringCompare(
                    itemName,
                    iName,
                    kCFCompareCaseInsensitive as ::core::ffi::c_int as CFStringCompareFlags,
                ) == 0
            {
                dict = item;
                break;
            } else {
                i += 1;
            }
        }
        CFRelease(itemName as CFTypeRef);
    }
    return dict;
}
#[no_mangle]
pub unsafe extern "C" fn findSelectorByName(
    mut feature: CFDictionaryRef,
    mut name: *const ::core::ffi::c_char,
    mut nameLength: ::core::ffi::c_int,
) -> CFNumberRef {
    let mut selector: CFNumberRef = ::core::ptr::null::<__CFNumber>();
    let mut selectors: CFArrayRef = CFDictionaryGetValue(
        feature,
        kCTFontFeatureTypeSelectorsKey as *const ::core::ffi::c_void,
    ) as CFArrayRef;
    if !selectors.is_null() {
        let mut s: CFDictionaryRef = findDictionaryInArray(
            selectors,
            kCTFontFeatureSelectorNameKey as *const ::core::ffi::c_void,
            name,
            nameLength,
        );
        if !s.is_null() {
            selector = CFDictionaryGetValue(
                s,
                kCTFontFeatureSelectorIdentifierKey as *const ::core::ffi::c_void,
            ) as CFNumberRef;
        }
    }
    return selector;
}
unsafe extern "C" fn createFeatureSettingDictionary(
    mut featureTypeIdentifier: CFNumberRef,
    mut featureSelectorIdentifier: CFNumberRef,
) -> CFDictionaryRef {
    let mut settingKeys: [*const ::core::ffi::c_void; 2] = [
        kCTFontFeatureTypeIdentifierKey as *const ::core::ffi::c_void,
        kCTFontFeatureSelectorIdentifierKey as *const ::core::ffi::c_void,
    ];
    let mut settingValues: [*const ::core::ffi::c_void; 2] = [
        featureTypeIdentifier as *const ::core::ffi::c_void,
        featureSelectorIdentifier as *const ::core::ffi::c_void,
    ];
    return CFDictionaryCreate(
        kCFAllocatorDefault,
        &raw mut settingKeys as *mut *const ::core::ffi::c_void,
        &raw mut settingValues as *mut *const ::core::ffi::c_void,
        2 as CFIndex,
        &raw const kCFTypeDictionaryKeyCallBacks,
        &raw const kCFTypeDictionaryValueCallBacks,
    );
}
#[no_mangle]
pub static mut kXeTeXEmboldenAttributeName: CFStringRef = &raw const XE_CFSTRING as CFStringRef;
#[no_mangle]
pub unsafe extern "C" fn loadAATfont(
    mut descriptor: CTFontDescriptorRef,
    mut scaled_size: int32_t,
    mut cp1: *const ::core::ffi::c_char,
) -> *mut ::core::ffi::c_void {
    let mut current_block: u64;
    let mut font: CTFontRef = ::core::ptr::null::<__CTFont>();
    let mut actualFont: CTFontRef = ::core::ptr::null::<__CTFont>();
    let mut ctSize: CGFloat = 0.;
    let mut stringAttributes: CFMutableDictionaryRef = ::core::ptr::null_mut::<__CFDictionary>();
    let mut attributes: CFMutableDictionaryRef = ::core::ptr::null_mut::<__CFDictionary>();
    let mut matrix: CGAffineTransform = CGAffineTransform {
        a: 0.,
        b: 0.,
        c: 0.,
        d: 0.,
        tx: 0.,
        ty: 0.,
    };
    let mut cascadeList: CFMutableArrayRef = ::core::ptr::null_mut::<__CFArray>();
    let mut lastResort: CTFontDescriptorRef = ::core::ptr::null::<__CTFontDescriptor>();
    let mut tracking: ::core::ffi::c_double = 0.0f64;
    let mut extend: ::core::ffi::c_float = 1.0f32;
    let mut slant: ::core::ffi::c_float = 0.0f32;
    let mut embolden: ::core::ffi::c_float = 0.0f32;
    let mut letterspace: ::core::ffi::c_float = 0.0f32;
    let mut rgbValue: uint32_t = 0;
    ctSize = TeXtoPSPoints(Fix2D(scaled_size as Fixed)) as CGFloat;
    font = CTFontCreateWithFontDescriptor(
        descriptor,
        ctSize,
        ::core::ptr::null::<CGAffineTransform>(),
    );
    if font.is_null() {
        return NULL;
    }
    stringAttributes = CFDictionaryCreateMutable(
        ::core::ptr::null::<__CFAllocator>(),
        0 as CFIndex,
        &raw const kCFTypeDictionaryKeyCallBacks,
        &raw const kCFTypeDictionaryValueCallBacks,
    );
    attributes = CFDictionaryCreateMutable(
        ::core::ptr::null::<__CFAllocator>(),
        0 as CFIndex,
        &raw const kCFTypeDictionaryKeyCallBacks,
        &raw const kCFTypeDictionaryValueCallBacks,
    );
    if !cp1.is_null() {
        let mut features: CFArrayRef = CTFontCopyFeatures(font);
        let mut featureSettings: CFMutableArrayRef = CFArrayCreateMutable(
            ::core::ptr::null::<__CFAllocator>(),
            0 as CFIndex,
            &raw const kCFTypeArrayCallBacks,
        );
        while *cp1 != 0 {
            let mut feature: CFDictionaryRef = ::core::ptr::null::<__CFDictionary>();
            let mut ret: ::core::ffi::c_int = 0;
            let mut cp2: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
            let mut cp3: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
            if *cp1 as ::core::ffi::c_int == ':' as i32 || *cp1 as ::core::ffi::c_int == ';' as i32
            {
                cp1 = cp1.offset(1);
            }
            while *cp1 as ::core::ffi::c_int == ' ' as i32
                || *cp1 as ::core::ffi::c_int == '\t' as i32
            {
                cp1 = cp1.offset(1);
            }
            if *cp1 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                break;
            }
            cp2 = cp1;
            while *cp2 as ::core::ffi::c_int != 0
                && *cp2 as ::core::ffi::c_int != ';' as i32
                && *cp2 as ::core::ffi::c_int != ':' as i32
            {
                cp2 = cp2.offset(1);
            }
            cp3 = cp1;
            while cp3 < cp2 && *cp3 as ::core::ffi::c_int != '=' as i32 {
                cp3 = cp3.offset(1);
            }
            if cp3 == cp2 {
                current_block = 5732607419501982586;
            } else {
                feature = findDictionaryInArray(
                    features,
                    kCTFontFeatureTypeNameKey as *const ::core::ffi::c_void,
                    cp1,
                    cp3.offset_from(cp1) as ::core::ffi::c_long as ::core::ffi::c_int,
                );
                if !feature.is_null() {
                    let mut featLen: ::core::ffi::c_int =
                        cp3.offset_from(cp1) as ::core::ffi::c_long as ::core::ffi::c_int;
                    let mut zeroInteger: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                    let mut zero: CFNumberRef = CFNumberCreate(
                        ::core::ptr::null::<__CFAllocator>(),
                        kCFNumberIntType as ::core::ffi::c_int as CFNumberType,
                        &raw mut zeroInteger as *const ::core::ffi::c_void,
                    );
                    cp3 = cp3.offset(1);
                    while cp3 < cp2 {
                        let mut selector: CFNumberRef = ::core::ptr::null::<__CFNumber>();
                        let mut disable: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                        let mut cp4: *const ::core::ffi::c_char =
                            ::core::ptr::null::<::core::ffi::c_char>();
                        while *cp3 as ::core::ffi::c_int == ' ' as i32
                            || *cp3 as ::core::ffi::c_int == '\t' as i32
                        {
                            cp3 = cp3.offset(1);
                        }
                        if *cp3 as ::core::ffi::c_int == '!' as i32 {
                            disable = 1 as ::core::ffi::c_int;
                            cp3 = cp3.offset(1);
                        }
                        cp4 = cp3;
                        while cp4 < cp2 && *cp4 as ::core::ffi::c_int != ',' as i32 {
                            cp4 = cp4.offset(1);
                        }
                        selector = findSelectorByName(
                            feature,
                            cp3,
                            cp4.offset_from(cp3) as ::core::ffi::c_long as ::core::ffi::c_int,
                        );
                        if !selector.is_null()
                            && CFNumberCompare(selector, zero, NULL) >= 0 as CFComparisonResult
                        {
                            let mut featureType: CFNumberRef = CFDictionaryGetValue(
                                feature,
                                kCTFontFeatureTypeIdentifierKey as *const ::core::ffi::c_void,
                            )
                                as CFNumberRef;
                            let mut featureSetting: CFDictionaryRef =
                                createFeatureSettingDictionary(featureType, selector);
                            CFArrayAppendValue(
                                featureSettings,
                                featureSetting as *const ::core::ffi::c_void,
                            );
                            CFRelease(featureSetting as CFTypeRef);
                        } else {
                            font_feature_warning(
                                cp1 as *const ::core::ffi::c_void,
                                featLen as int32_t,
                                cp3 as *const ::core::ffi::c_void,
                                cp4.offset_from(cp3) as ::core::ffi::c_long as int32_t,
                            );
                        }
                        cp3 = cp4.offset(1 as ::core::ffi::c_int as isize);
                    }
                    CFRelease(zero as CFTypeRef);
                    current_block = 1542033413836778875;
                } else {
                    ret = readCommonFeatures(
                        cp1,
                        cp2,
                        &raw mut extend,
                        &raw mut slant,
                        &raw mut embolden,
                        &raw mut letterspace,
                        &raw mut rgbValue,
                    );
                    if ret == 1 as ::core::ffi::c_int {
                        current_block = 1542033413836778875;
                    } else if ret == -(1 as ::core::ffi::c_int) {
                        current_block = 5732607419501982586;
                    } else {
                        cp3 = strstartswith(
                            cp1,
                            b"tracking\0" as *const u8 as *const ::core::ffi::c_char,
                        );
                        if !cp3.is_null() {
                            let mut trackingNumber: CFNumberRef = ::core::ptr::null::<__CFNumber>();
                            if *cp3 as ::core::ffi::c_int != '=' as i32 {
                                current_block = 5732607419501982586;
                            } else {
                                cp3 = cp3.offset(1);
                                tracking = read_double(&raw mut cp3);
                                trackingNumber = CFNumberCreate(
                                    ::core::ptr::null::<__CFAllocator>(),
                                    kCFNumberDoubleType as ::core::ffi::c_int as CFNumberType,
                                    &raw mut tracking as *const ::core::ffi::c_void,
                                );
                                CFDictionaryAddValue(
                                    stringAttributes,
                                    kCTKernAttributeName as *const ::core::ffi::c_void,
                                    trackingNumber as *const ::core::ffi::c_void,
                                );
                                CFRelease(trackingNumber as CFTypeRef);
                                current_block = 1542033413836778875;
                            }
                        } else {
                            current_block = 5732607419501982586;
                        }
                    }
                }
            }
            match current_block {
                5732607419501982586 => {
                    if !strstartswith(
                        cp1,
                        b"vertical\0" as *const u8 as *const ::core::ffi::c_char,
                    )
                    .is_null()
                    {
                        cp3 = cp2;
                        if *cp3 as ::core::ffi::c_int == ';' as i32
                            || *cp3 as ::core::ffi::c_int == ':' as i32
                        {
                            cp3 = cp3.offset(-1);
                        }
                        while *cp3 as ::core::ffi::c_int == '\0' as i32
                            || *cp3 as ::core::ffi::c_int == ' ' as i32
                            || *cp3 as ::core::ffi::c_int == '\t' as i32
                        {
                            cp3 = cp3.offset(-1);
                        }
                        if *cp3 != 0 {
                            cp3 = cp3.offset(1);
                        }
                        if cp3 == cp1.offset(8 as ::core::ffi::c_int as isize) {
                            let mut orientation: ::core::ffi::c_int =
                                kCTFontOrientationVertical as ::core::ffi::c_int;
                            let mut orientationNumber: CFNumberRef = CFNumberCreate(
                                ::core::ptr::null::<__CFAllocator>(),
                                kCFNumberIntType as ::core::ffi::c_int as CFNumberType,
                                &raw mut orientation as *const ::core::ffi::c_void,
                            );
                            CFDictionaryAddValue(
                                attributes,
                                kCTFontOrientationAttribute as *const ::core::ffi::c_void,
                                orientationNumber as *const ::core::ffi::c_void,
                            );
                            CFRelease(orientationNumber as CFTypeRef);
                            CFDictionaryAddValue(
                                stringAttributes,
                                kCTVerticalFormsAttributeName as *const ::core::ffi::c_void,
                                kCFBooleanTrue as *const ::core::ffi::c_void,
                            );
                            current_block = 1542033413836778875;
                        } else {
                            current_block = 5181772461570869434;
                        }
                    } else {
                        current_block = 5181772461570869434;
                    }
                    match current_block {
                        1542033413836778875 => {}
                        _ => {
                            font_feature_warning(
                                cp1 as *const ::core::ffi::c_void,
                                cp2.offset_from(cp1) as ::core::ffi::c_long as int32_t,
                                ::core::ptr::null::<::core::ffi::c_void>(),
                                0 as int32_t,
                            );
                        }
                    }
                }
                _ => {}
            }
            cp1 = cp2;
        }
        if !features.is_null() {
            CFRelease(features as CFTypeRef);
        }
        if CFArrayGetCount(featureSettings as CFArrayRef) != 0 {
            CFDictionaryAddValue(
                attributes,
                kCTFontFeatureSettingsAttribute as *const ::core::ffi::c_void,
                featureSettings as *const ::core::ffi::c_void,
            );
        }
        CFRelease(featureSettings as CFTypeRef);
    }
    if loaded_font_flags as ::core::ffi::c_int & FONT_FLAGS_COLORED != 0 as ::core::ffi::c_int {
        let mut red: CGFloat =
            ((rgbValue & 0xff000000 as uint32_t) >> 24 as ::core::ffi::c_int) as CGFloat / 255.0f64;
        let mut green: CGFloat =
            ((rgbValue & 0xff0000 as uint32_t) >> 16 as ::core::ffi::c_int) as CGFloat / 255.0f64;
        let mut blue: CGFloat =
            ((rgbValue & 0xff00 as uint32_t) >> 8 as ::core::ffi::c_int) as CGFloat / 255.0f64;
        let mut alpha: CGFloat = (rgbValue & 0xff as uint32_t) as CGFloat / 255.0f64;
        let mut color: CGColorRef = CGColorCreateGenericRGB(red, green, blue, alpha);
        CFDictionaryAddValue(
            stringAttributes,
            kCTForegroundColorAttributeName as *const ::core::ffi::c_void,
            color as *const ::core::ffi::c_void,
        );
        CGColorRelease(color);
    }
    matrix = CGAffineTransformIdentity;
    if extend as ::core::ffi::c_double != 1.0f64 || slant as ::core::ffi::c_double != 0.0f64 {
        matrix = __CGAffineTransformMake(
            extend as CGFloat,
            0 as ::core::ffi::c_int as CGFloat,
            slant as CGFloat,
            1.0f64,
            0 as ::core::ffi::c_int as CGFloat,
            0 as ::core::ffi::c_int as CGFloat,
        );
    }
    if embolden as ::core::ffi::c_double != 0.0f64 {
        let mut emboldenNumber: CFNumberRef = ::core::ptr::null::<__CFNumber>();
        embolden = (embolden as ::core::ffi::c_double * Fix2D(scaled_size as Fixed) / 100.0f64)
            as ::core::ffi::c_float;
        emboldenNumber = CFNumberCreate(
            ::core::ptr::null::<__CFAllocator>(),
            kCFNumberFloatType as ::core::ffi::c_int as CFNumberType,
            &raw mut embolden as *const ::core::ffi::c_void,
        );
        CFDictionaryAddValue(
            stringAttributes,
            kXeTeXEmboldenAttributeName as *const ::core::ffi::c_void,
            emboldenNumber as *const ::core::ffi::c_void,
        );
        CFRelease(emboldenNumber as CFTypeRef);
    }
    if letterspace as ::core::ffi::c_double != 0.0f64 {
        loaded_font_letter_space = (letterspace as ::core::ffi::c_double / 100.0f64
            * scaled_size as ::core::ffi::c_double) as scaled_t;
    }
    cascadeList = CFArrayCreateMutable(
        ::core::ptr::null::<__CFAllocator>(),
        1 as CFIndex,
        &raw const kCFTypeArrayCallBacks,
    );
    lastResort = CTFontDescriptorCreateWithNameAndSize(
        &raw const LAST_RESORT_CFSTRING as CFStringRef,
        0 as ::core::ffi::c_int as CGFloat,
    );
    CFArrayAppendValue(cascadeList, lastResort as *const ::core::ffi::c_void);
    CFRelease(lastResort as CFTypeRef);
    CFDictionaryAddValue(
        attributes,
        kCTFontCascadeListAttribute as *const ::core::ffi::c_void,
        cascadeList as *const ::core::ffi::c_void,
    );
    CFRelease(cascadeList as CFTypeRef);
    descriptor = CTFontDescriptorCreateWithAttributes(attributes as CFDictionaryRef);
    CFRelease(attributes as CFTypeRef);
    actualFont = CTFontCreateCopyWithAttributes(font, ctSize, &raw mut matrix, descriptor);
    CFRelease(font as CFTypeRef);
    CFDictionaryAddValue(
        stringAttributes,
        kCTFontAttributeName as *const ::core::ffi::c_void,
        actualFont as *const ::core::ffi::c_void,
    );
    CFRelease(actualFont as CFTypeRef);
    native_font_type_flag = AAT_FONT_FLAG as int32_t;
    return stringAttributes as *mut ::core::ffi::c_void;
}
pub const false_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
