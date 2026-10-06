// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
// Public PDF document primitives implemented by the native renderer.
use crate::backend_definitions::{Parameter, Storage};
pub const PARAMETERS: &[Parameter] = &[
    Parameter {
        name: "pdfobjcompresslevel",
        storage: Storage::Integer,
        initial: 0,
    },
    Parameter {
        name: "pdflinkmargin",
        storage: Storage::Dimension,
        initial: 0,
    },
    Parameter {
        name: "pdfthreadmargin",
        storage: Storage::Dimension,
        initial: 0,
    },
];
pub const PRIMITIVES: &[(&str, &str, i32)] = &[
    ("pdfthread", "extension", 170),
    ("pdfstartthread", "extension", 171),
    ("pdfendthread", "extension", 172),
    ("pdfrunninglinkoff", "extension", 173),
    ("pdfrunninglinkon", "extension", 174),
    ("pdflastannot", "lastitem", 2400),
    ("pdflastlink", "lastitem", 2401),
    ("pdfpageref", "convert", 175),
    ("pdfxformname", "convert", 176),
    ("pdfximagebbox", "convert", 177),
    ("pdflastximagecolordepth", "lastitem", 2402),
];
