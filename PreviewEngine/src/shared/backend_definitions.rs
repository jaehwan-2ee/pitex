// Pitex-authored backend parameters. Copyright (c) 2026 Pitex contributors.
// SPDX-License-Identifier: AGPL-3.0-or-later
// Storage is registered by the engine; values are sent to the native renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Storage {
    Integer,
    Dimension,
    Tokens,
}
#[derive(Clone, Copy, Debug)]
pub struct Parameter {
    pub name: &'static str,
    pub storage: Storage,
    pub initial: i32,
}
pub const PARAMETERS: &[Parameter] = &[
    Parameter {
        name: "pdfdecimaldigits",
        storage: Storage::Integer,
        initial: 3,
    },
    Parameter {
        name: "pdfdestmargin",
        storage: Storage::Dimension,
        initial: 0,
    },
    Parameter {
        name: "pdfdraftmode",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfpagesattr",
        storage: Storage::Tokens,
        initial: 0,
    },
    Parameter {
        name: "pdfgamma",
        storage: Storage::Integer,
        initial: 1000,
    },
    Parameter {
        name: "pdfgentounicode",
        storage: Storage::Integer,
        initial: 1,
    },
    Parameter {
        name: "pdfomitcharset",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfomitinfodict",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfomitprocset",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfimageapplygamma",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfimagegamma",
        storage: Storage::Integer,
        initial: 2200,
    },
    Parameter {
        name: "pdfimagehicolor",
        storage: Storage::Integer,
        initial: 1,
    },
    Parameter {
        name: "pdfimageresolution",
        storage: Storage::Integer,
        initial: 72,
    },
    Parameter {
        name: "pdfinclusioncopyfonts",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfoptionpdfinclusionerrorlevel",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfinclusionerrorlevel",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfinfoomitdate",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfoptionalwaysusepdfpagebox",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfforcepagebox",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfpagebox",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfpxdimen",
        storage: Storage::Dimension,
        initial: 65782,
    },
    Parameter {
        name: "pdfpkmode",
        storage: Storage::Tokens,
        initial: 0,
    },
    Parameter {
        name: "pdfpkresolution",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfsuppressptexinfo",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfsuppresswarningdupdest",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfsuppresswarningdupmap",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfsuppresswarningpagegroup",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfuniqueresname",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfuseptexunderscore",
        storage: Storage::Integer,
        initial: 0,
    },
];
pub const ALIASES: &[(&str, &str)] = &[
    ("pdfoptionpdfminorversion", "pdfminorversion"),
];
