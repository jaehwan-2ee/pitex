// Original Pitex font and typography declarations, written from public specifications.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use crate::backend_definitions::{Parameter, Storage};
pub const PARAMETERS: &[Parameter] = &[
    Parameter { name: "showstream", storage: Storage::Integer, initial: -1 },
    Parameter {
        name: "pdfadjustinterwordglue",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfappendkern",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfprependkern",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdftracingfonts",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfmovechars",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdfignoreddimen",
        storage: Storage::Dimension,
        initial: -65_536_000,
    },
    Parameter {
        name: "pdfeachlineheight",
        storage: Storage::Dimension,
        initial: -65_536_000,
    },
    Parameter {
        name: "pdfeachlinedepth",
        storage: Storage::Dimension,
        initial: -65_536_000,
    },
    Parameter {
        name: "pdffirstlineheight",
        storage: Storage::Dimension,
        initial: -65_536_000,
    },
    Parameter {
        name: "pdflastlinedepth",
        storage: Storage::Dimension,
        initial: -65_536_000,
    },
];
pub const ALIASES: &[(&str, &str)] = &[];
pub const PRIMITIVES: &[(&str, &str, i32)] = &[
    ("efcode", "fontinteger", 100),
    ("knaccode", "fontinteger", 101),
    ("knbccode", "fontinteger", 102),
    ("knbscode", "fontinteger", 103),
    ("stbscode", "fontinteger", 104),
    ("shbscode", "fontinteger", 105),
    ("tagcode", "fontinteger", 106),
    ("pdffontexpand", "extension", 130),
    ("pdfcopyfont", "extension", 131),
    ("letterspacefont", "extension", 132),
    ("pdfnoligatures", "extension", 133),
    ("pdfincludechars", "extension", 134),
    ("pdffontattr", "extension", 135),
    ("pdffakespace", "extension", 136),
    ("pdfinterwordspaceon", "extension", 137),
    ("pdfinterwordspaceoff", "extension", 138),
    ("pdfspacefont", "extension", 139),
    ("quitvmode", "extension", 140),
    ("pdfnobuiltintounicode", "extension", 141),
    ("pdffontsize", "convert", 130),
    ("pdffontname", "convert", 131),
    ("pdffontobjnum", "convert", 132),
    ("pdfinsertht", "convert", 133),
];
