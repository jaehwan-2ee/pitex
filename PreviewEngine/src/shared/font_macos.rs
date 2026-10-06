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

// Manual Rust translation of xetex/layout/xetex-XeTeXFontInst_Mac.cpp.
// Original SHA-256: 4e3e0e5b52d70c075769922c8480e449daaf039a56534d67c6128cd1094a7e81
// Platform handles are opaque; CoreFoundation retains/releases match their APIs.
use std::{
    ffi::{c_char, c_void, CStr},
    ptr,
};
type Handle = *mut c_void;
#[repr(C)]
#[derive(Default)]
pub struct FontInit {
    descriptor: Handle,
    font: Handle,
    path: *mut c_char,
    index: u32,
}
extern "C" {
    fn malloc(size: usize) -> Handle;
    fn free(pointer: Handle);
    fn strdup(value: *const c_char) -> *mut c_char;
    fn CTFontCopyName(font: Handle, key: Handle) -> Handle;
    fn CTFontCopyAttribute(font: Handle, key: Handle) -> Handle;
    fn CFStringGetLength(string: Handle) -> isize;
    fn CFStringGetCString(string: Handle, buffer: *mut c_char, length: isize, encoding: u32) -> u8;
    fn CFURLGetFileSystemRepresentation(
        url: Handle,
        resolve: u8,
        buffer: *mut u8,
        length: isize,
    ) -> u8;
    fn CFRelease(object: Handle);
    fn CFArrayCreate(
        allocator: Handle,
        values: *const Handle,
        count: isize,
        callbacks: *const u8,
    ) -> Handle;
    fn CFDictionaryCreate(
        allocator: Handle,
        keys: *const Handle,
        values: *const Handle,
        count: isize,
        key_callbacks: *const u8,
        value_callbacks: *const u8,
    ) -> Handle;
    fn CTFontDescriptorCreateCopyWithAttributes(descriptor: Handle, attributes: Handle) -> Handle;
    fn CTFontCreateWithFontDescriptor(descriptor: Handle, size: f64, matrix: Handle) -> Handle;
    static kCTFontURLAttribute: Handle;
    static kCTFontPostScriptNameKey: Handle;
    static kCTFontCascadeListAttribute: Handle;
    static kCFTypeArrayCallBacks: u8;
    static kCFTypeDictionaryKeyCallBacks: u8;
    static kCFTypeDictionaryValueCallBacks: u8;
}
unsafe fn name(font: Handle, key: Handle) -> *mut c_char {
    let name = CTFontCopyName(font, key);
    if name.is_null() {
        return ptr::null_mut();
    }
    let length = CFStringGetLength(name).saturating_mul(6).saturating_add(1);
    let data = malloc(length as usize).cast::<c_char>();
    if data.is_null() {
        CFRelease(name);
        return data;
    }
    let converted = CFStringGetCString(name, data, length, 0x08000100) != 0;
    CFRelease(name);
    if converted {
        data
    } else {
        free(data.cast());
        ptr::null_mut()
    }
}
#[no_mangle]
pub unsafe extern "C" fn getFileNameFromCTFont(font: Handle, index: *mut u32) -> *mut c_char {
    let url = CTFontCopyAttribute(font, kCTFontURLAttribute);
    if url.is_null() {
        return ptr::null_mut();
    }
    let mut path = [0u8; 1024];
    let represented =
        CFURLGetFileSystemRepresentation(url, 1, path.as_mut_ptr(), path.len() as isize) != 0;
    CFRelease(url);
    if !represented {
        return ptr::null_mut();
    }
    let path = path.as_ptr().cast::<c_char>();
    let postscript = name(font, kCTFontPostScriptNameKey);
    let selected = crate::font_freetype::collection_index(path, postscript);
    free(postscript.cast());
    if let Some(selected) = selected {
        *index = selected;
        strdup(path)
    } else {
        *index = u32::MAX;
        ptr::null_mut()
    }
}
#[no_mangle]
pub unsafe extern "C" fn pitex_mac_font_initialize(
    descriptor: Handle,
    size: f32,
    result: *mut FontInit,
    status: *mut i32,
) {
    *result = FontInit::default();
    if descriptor.is_null() {
        *status = 1;
        return;
    }
    let empty = CFArrayCreate(
        ptr::null_mut(),
        ptr::null(),
        0,
        ptr::addr_of!(kCFTypeArrayCallBacks),
    );
    let keys = [kCTFontCascadeListAttribute];
    let values = [empty];
    let attributes = CFDictionaryCreate(
        ptr::null_mut(),
        keys.as_ptr(),
        values.as_ptr(),
        1,
        ptr::addr_of!(kCFTypeDictionaryKeyCallBacks),
        ptr::addr_of!(kCFTypeDictionaryValueCallBacks),
    );
    CFRelease(empty);
    let copied = CTFontDescriptorCreateCopyWithAttributes(descriptor, attributes);
    CFRelease(attributes);
    let font = CTFontCreateWithFontDescriptor(copied, size as f64 * 72. / 72.27, ptr::null_mut());
    if font.is_null() {
        if !copied.is_null() {
            CFRelease(copied);
        }
        *status = 1;
        return;
    }
    (*result).descriptor = copied;
    (*result).font = font;
    (*result).path = getFileNameFromCTFont(font, &mut (*result).index);
    if (*result).path.is_null() {
        *status = 1;
    }
}
