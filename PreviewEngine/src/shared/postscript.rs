// Pitex-authored. Copyright (c) 2026 Pitex contributors.
// Original implementation from Adobe's PostScript Language Reference, third edition.
// Licensed under the repository's GNU AGPL (AGPL-3.0-or-later). No converter source is used.
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
#[path = "postscript_fonts.rs"]
mod fonts;
#[path = "postscript_images.rs"]
mod images;
#[path = "postscript_color.rs"]
mod color;
#[path = "postscript_shading.rs"]
mod shading;
#[path = "postscript_vm.rs"]
mod vm;
use vm::{Access,Sequence};

type Dict = Rc<RefCell<BTreeMap<String, Value>>>;
type Array = Sequence<Value>;
type File = vm::File;
#[derive(Clone, Debug)]
enum Value {
    Number(f64),
    Bool(bool),
    Name(String),
    Word(String),
    Op(String),
    String(Sequence<u8>),
    File(File),
    Array(Array),
    Proc(Array),
    Save(u64),
    Dict(Dict),
    Mark,
    Null,
}
impl Value {
    fn number(&self) -> Result<f64, String> {
        if let Self::Number(n) = self {
            Ok(*n)
        } else {
            Err("typecheck: expected number".into())
        }
    }
    fn name(&self) -> Result<String, String> {
        match self {
            Self::Name(n) | Self::Word(n) | Self::Op(n) => Ok(n.clone()),
            Self::String(s) => {s.read()?;Ok(String::from_utf8_lossy(&s.borrow()).into())},
            _ => Err("typecheck: expected name".into()),
        }
    }
    fn boolean(&self) -> Result<bool, String> {
        if let Self::Bool(b) = self {
            Ok(*b)
        } else {
            Err("typecheck: expected boolean".into())
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    one: bool,
}
impl<'a> Reader<'a> {
    fn space(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'%' => {
                    while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
                        self.pos += 1;
                    }
                }
                b if b.is_ascii_whitespace() => self.pos += 1,
                _ => break,
            }
        }
    }
    fn values(&mut self, end: Option<u8>) -> Result<Vec<Value>, String> {
        let mut out = Vec::new();
        loop {
            self.space();
            if self.pos >= self.bytes.len() {
                if end.is_some() {
                    return Err("syntaxerror: unterminated composite".into());
                }
                break;
            }
            let b = self.bytes[self.pos];
            if Some(b) == end {
                self.pos += 1;
                break;
            }
            self.pos += 1;
            let v = match b {
                b'{' => Value::Proc(self.values(Some(b'}'))?.into()),
                b'[' => Value::Word("[".into()),
                b']' => Value::Word("]".into()),
                b'<' if self.bytes.get(self.pos) == Some(&b'<') => {
                    self.pos += 1;
                    Value::Word("<<".into())
                }
                b'>' if self.bytes.get(self.pos) == Some(&b'>') => {
                    self.pos += 1;
                    Value::Word(">>".into())
                }
                b'(' => {
                    let mut s = Vec::new();
                    let mut depth = 1;
                    while self.pos < self.bytes.len() && depth > 0 {
                        let c = self.bytes[self.pos];
                        self.pos += 1;
                        match c {
                            b'\\' => {
                                let c = *self
                                    .bytes
                                    .get(self.pos)
                                    .ok_or("syntaxerror: string escape")?;
                                self.pos += 1;
                                match c {
                                    b'n' => s.push(b'\n'),
                                    b'r' => s.push(b'\r'),
                                    b't' => s.push(b'\t'),
                                    b'b' => s.push(8),
                                    b'f' => s.push(12),
                                    b'\n' => {}
                                    b'\r' => {
                                        if self.bytes.get(self.pos) == Some(&b'\n') {
                                            self.pos += 1;
                                        }
                                    }
                                    b'0'..=b'7' => {
                                        let mut n = (c - b'0') as u16;
                                        for _ in 0..2 {
                                            if let Some(c @ b'0'..=b'7') = self.bytes.get(self.pos)
                                            {
                                                n = n * 8 + (c - b'0') as u16;
                                                self.pos += 1;
                                            } else {
                                                break;
                                            }
                                        }
                                        s.push(n as u8);
                                    }
                                    _ => s.push(c),
                                }
                            }
                            b'(' => {
                                depth += 1;
                                s.push(c)
                            }
                            b')' => {
                                depth -= 1;
                                if depth > 0 {
                                    s.push(c)
                                }
                            }
                            _ => s.push(c),
                        }
                    }
                    if depth != 0 {
                        return Err("syntaxerror: unterminated string".into());
                    }
                    Value::String(Rc::new(RefCell::new(s)).into())
                }
                b'<' => {
                    let mut hex = String::new();
                    while self.pos < self.bytes.len() && self.bytes[self.pos] != b'>' {
                        let c = self.bytes[self.pos];
                        self.pos += 1;
                        if !c.is_ascii_whitespace() {
                            hex.push(c as char)
                        }
                    }
                    if self.pos == self.bytes.len() {
                        return Err("syntaxerror: hex string".into());
                    }
                    self.pos += 1;
                    if hex.len() % 2 != 0 {
                        hex.push('0')
                    }
                    let s = (0..hex.len())
                        .step_by(2)
                        .map(|i| {
                            u8::from_str_radix(&hex[i..i + 2], 16)
                                .map_err(|_| "syntaxerror: hexadecimal".to_string())
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    Value::String(Rc::new(RefCell::new(s)).into())
                }
                b'}' | b')' | b'>' => return Err("syntaxerror: unexpected delimiter".into()),
                _ => {
                    let literal = b == b'/';
                    let start = if literal { self.pos } else { self.pos - 1 };
                    while self.pos < self.bytes.len()
                        && !self.bytes[self.pos].is_ascii_whitespace()
                        && !b"()<>[]{}/%".contains(&self.bytes[self.pos])
                    {
                        self.pos += 1;
                    }
                    let s = String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned();
                    if literal {
                        Value::Name(s)
                    } else if let Ok(n) = s.parse::<f64>() {
                        Value::Number(n)
                    } else if let Some((base, digits)) = s.split_once('#') {
                        Value::Number(
                            i64::from_str_radix(
                                digits,
                                base.parse().map_err(|_| "syntaxerror: radix")?,
                            )
                            .map_err(|_| "syntaxerror: number")? as f64,
                        )
                    } else {
                        Value::Word(s)
                    }
                }
            };
            out.push(v);
            if self.one && end.is_none() { break; }
            if out.len() > 500_000 {
                return Err("limitcheck: source token count".into());
            }
        }
        Ok(out)
    }
}

#[derive(Clone, Debug)]
enum Segment {
    Move(f64, f64),
    Line(f64, f64),
    Curve([f64; 6]),
    Close,
}
#[derive(Clone)]
struct Graphics {
    matrix: [f64; 6],
    path: Vec<Segment>,
    point: Option<(f64, f64)>,
    start: Option<(f64, f64)>,
    color: String,
    width: f64,
    cap: i32,
    join: i32,
    miter: f64,
    dash: Vec<f64>,
    phase: f64,
    fill_alpha: f64,
    stroke_alpha: f64,
    clips: Vec<(Vec<Segment>, bool)>,
    font: Option<Dict>,
    image_components: usize,
    color_space: color::Space,
    color_definition: Value,
    color_resource: String,
    color_values: Vec<f64>,
}
impl Default for Graphics {
    fn default() -> Self {
        Self {
            matrix: [1., 0., 0., 1., 0., 0.],
            path: vec![],
            point: None,
            start: None,
            color: "0 g 0 G".into(),
            width: 1.,
            cap: 0,
            join: 0,
            miter: 10.,
            dash: vec![],
            phase: 0.,
            fill_alpha: 1.,
            stroke_alpha: 1.,
            clips: vec![],
            font: None,
            image_components: 1,
            color_space: color::Space::Gray,
            color_definition: Value::Array(vec![Value::Name("DeviceGray".into())].into()),
            color_resource: "/DeviceGray".into(),
            color_values: vec![0.],
        }
    }
}
fn transform(m: [f64; 6], x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}
fn multiply(a: [f64; 6], b: [f64; 6]) -> [f64; 6] {
    [
        a[0] * b[0] + a[2] * b[1],
        a[1] * b[0] + a[3] * b[1],
        a[0] * b[2] + a[2] * b[3],
        a[1] * b[2] + a[3] * b[3],
        a[0] * b[4] + a[2] * b[5] + a[4],
        a[1] * b[4] + a[3] * b[5] + a[5],
    ]
}
fn inverse(m: [f64; 6]) -> Result<[f64; 6], String> {
    let d = m[0] * m[3] - m[1] * m[2];
    if d.abs() < 1e-20 {
        return Err("undefinedresult: singular matrix".into());
    }
    Ok([
        m[3] / d,
        -m[1] / d,
        -m[2] / d,
        m[0] / d,
        (m[2] * m[5] - m[3] * m[4]) / d,
        (m[1] * m[4] - m[0] * m[5]) / d,
    ])
}
fn format_path(path: &[Segment]) -> String {
    let mut s = String::new();
    for p in path {
        match p {
            Segment::Move(x, y) => s += &format!("{x:.6} {y:.6} m\n"),
            Segment::Line(x, y) => s += &format!("{x:.6} {y:.6} l\n"),
            Segment::Curve(v) => {
                s += &format!(
                    "{:.6} {:.6} {:.6} {:.6} {:.6} {:.6} c\n",
                    v[0], v[1], v[2], v[3], v[4], v[5]
                )
            }
            Segment::Close => s += "h\n",
        }
    }
    s
}

pub struct Interpreter {
    stack: Vec<Value>,
    dicts: Vec<Dict>,
    system: Dict,
    user: Dict,
    graphics: Graphics,
    saved: Vec<Graphics>,
    output: String,
    budget: usize,
    depth: usize,
    exit: bool,
    x: f64,
    y: f64,
    alphas: Vec<(String, f64, f64)>,
    headers: std::collections::BTreeSet<String>,
    random: u64,
    fonts: BTreeMap<String, Rc<fonts::Font>>,
    dvi_global: bool,
    current_file: Option<File>,
    shadings: Vec<shading::Shading>,
    vm: vm::Vm,
    global: Dict,
    executing: Vec<Value>,
}
impl Interpreter {
    pub fn new() -> Self {
        let system = Rc::new(RefCell::new(BTreeMap::new()));
        let user = Rc::new(RefCell::new(BTreeMap::new()));
        let global = Rc::new(RefCell::new(BTreeMap::new()));
        let mut s = Self {
            stack: vec![],
            dicts: vec![system.clone(), global.clone(), user.clone()],
            system,
            user,
            graphics: Graphics::default(),
            saved: vec![],
            output: String::new(),
            budget: 0,
            depth: 0,
            exit: false,
            x: 0.,
            y: 0.,
            alphas: vec![],
            headers: Default::default(),
            random: 1,
            fonts: BTreeMap::new(),
            dvi_global: false,
            current_file: None,
            shadings: Vec::new(),
            vm: vm::Vm::default(),
            global,
            executing: Vec::new(),
        };
        let ops="add sub mul div idiv mod neg abs sqrt exp ln log sin cos atan floor ceiling round truncate cvi cvr dup exch pop copy index roll clear count mark cleartomark counttomark [ ] << >> dict begin end def store load where known currentdict systemdict userdict globaldict get put getinterval putinterval length array string aload astore forall repeat for loop exit if ifelse exec bind cvx cvlit xcheck type eq ne lt le gt ge and or xor not bitshift true false null matrix currentmatrix defaultmatrix setmatrix concat translate scale rotate transform dtransform itransform idtransform concatmatrix invertmatrix newpath moveto lineto rmoveto rlineto curveto rcurveto closepath currentpoint arc arcn arct arcto gsave grestore grestoreall save restore stroke fill eofill clip eoclip initclip rectfill rectstroke rectclip pathbbox pathforall flattenpath setgray currentgray setrgbcolor currentrgbcolor setcmykcolor currentcmykcolor sethsbcolor setlinewidth currentlinewidth setlinecap currentlinecap setlinejoin currentlinejoin setmiterlimit currentmiterlimit setdash currentdash setflat currentflat setglobal currentglobal rcheck wcheck gcheck packedarray readonly executeonly noaccess setpacking currentpacking stopped stop print flush flushpage showpage copypage erasepage revision languagelevel version product rand srand rrand realtime usertime statusdict setpagedevice currentpagedevice .setopacityalpha .setfillconstantalpha .setstrokeconstantalpha .setshapealpha .setalphaisshape";
        for op in ops.split_whitespace() {
            s.system
                .borrow_mut()
                .insert(op.into(), Value::Op(op.into()));
        }
        for op in "shfill image imagemask colorimage setcolorspace currentcolorspace setcolor currentcolor filter currentfile read readstring readhexstring closefile bytesavailable resetfile fileposition setfileposition".split_whitespace() {
            s.system.borrow_mut().insert(op.into(), Value::Op(op.into()));
        }
        for op in "findfont scalefont makefont setfont currentfont definefont undefinefont stringwidth show ashow widthshow awidthshow charpath".split_whitespace() {
            s.system.borrow_mut().insert(op.into(), Value::Op(op.into()));
        }
        let sd = Rc::new(RefCell::new(BTreeMap::new()));
        sd.borrow_mut()
            .insert("normalscale".into(), Value::Proc(vec![].into()));
        s.user.borrow_mut().insert("SDict".into(), Value::Dict(sd));
        s.user.borrow_mut().insert(
            "TeXDict".into(),
            Value::Dict(Rc::new(RefCell::new(BTreeMap::new()))),
        );
        s.vm.set_space(&Value::Dict(s.system.clone()),true);
        s.vm.set_space(&Value::Dict(s.global.clone()),true);
        s.vm.set_space(&Value::Dict(s.user.clone()),false);
        s.vm.restrict(&mut Value::Dict(s.system.clone()),Access::ReadOnly).unwrap();
        s
    }
    pub fn opacity_resources(&self) -> Vec<(String, f64, f64)> {
        self.alphas.clone()
    }
    pub fn begin_page(&mut self) {
        self.graphics = Graphics::default();
        self.saved.clear();
        self.stack.clear();
        self.dicts.truncate(3);
    }
    pub fn recognizes(s: &str) -> bool {
        s.starts_with("header=")
            || s.starts_with("PST:")
            || s.starts_with("pst:")
            || s.starts_with("ps:")
            || s.starts_with('!')
            || s.starts_with('"')
            || s.starts_with("PSfile=")
            || s.starts_with("psfile=")
    }
    pub fn special(
        &mut self,
        s: &str,
        x: f64,
        y: f64,
        mut load: impl FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<Option<String>, String> {
        let s = s.trim();
        if !Self::recognizes(s) {
            return Ok(None);
        }
        self.output.clear();
        self.budget = 2_000_000;
        self.x = x;
        self.y = y;
        self.exit = false;
        if let Some(file) = s.strip_prefix("header=") {
            let file = file.trim_matches('"');
            if self.headers.contains(file) {
                return Ok(Some(String::new()));
            }
            let data = load(file).ok_or_else(|| format!("PostScript header not found: {file}"))?;
            self.execute_loaded(&data, &mut load)?;
            self.headers.insert(file.into());
            return Ok(Some(std::mem::take(&mut self.output)));
        }
        let prior_graphics = self.graphics.clone();
        let prior_saved = self.saved.len();
        let prior_dicts = self.dicts.len();
        let prior_stack = self.stack.len();
        let isolated = s.starts_with("pst:")
            || s.starts_with('"')
            || s.starts_with("PSfile=")
            || s.starts_with("psfile=");
        self.dvi_global = s.starts_with("PST:");
        if isolated {
            self.graphics = prior_graphics.clone();
            self.graphics.matrix = multiply(prior_graphics.matrix, [1., 0., 0., 1., x, y]);
            self.graphics.path.clear();
            // DVI PostScript literals enter with a moveto at TeX's current
            // point. rectfill preserves it; a later arc connects to it.
            let origin = transform(prior_graphics.matrix, x, y);
            self.graphics.path.push(Segment::Move(origin.0, origin.1));
            self.graphics.point = Some(origin);
            self.graphics.start = Some(origin);
        } else {
            self.graphics.point = Some(transform(self.graphics.matrix, x, y));
        }
        let result = if s.starts_with("PSfile=") || s.starts_with("psfile=") {
            self.eps(s, x, y, &mut load)
        } else {
            let code = if s.starts_with('!') || s.starts_with('"') {
                &s[1..]
            } else {
                s.split_once(':').unwrap().1
            };
            self.execute_loaded(code.as_bytes(), &mut load)
        };
        if isolated {
            self.graphics = prior_graphics;
            self.saved.truncate(prior_saved);
            self.dicts.truncate(prior_dicts);
            self.stack.truncate(prior_stack);
        }
        if let Err(e) = result {
            self.output.clear();
            self.stack.truncate(prior_stack);
            self.dicts.truncate(prior_dicts);
            return Err(format!("embedded PostScript: {e}"));
        }
        Ok(Some(std::mem::take(&mut self.output)))
    }
    fn eps(
        &mut self,
        s: &str,
        x: f64,
        y: f64,
        load: &mut impl FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<(), String> {
        let rest = s.split_once('=').unwrap().1;
        let (file, args) = if let Some(rest) = rest.strip_prefix('"') {
            let i = rest.find('"').ok_or("syntaxerror: EPS filename")?;
            (&rest[..i], &rest[i + 1..])
        } else {
            let i = rest.find(char::is_whitespace).unwrap_or(rest.len());
            (&rest[..i], &rest[i..])
        };
        let bytes = load(file).ok_or_else(|| format!("EPS file not found: {file}"))?;
        let text = String::from_utf8_lossy(&bytes);
        let mut opts = BTreeMap::new();
        for word in args.split_whitespace() {
            if let Some((k, v)) = word.split_once('=') {
                if let Ok(v) = v.parse::<f64>() {
                    opts.insert(k, v);
                }
            }
        }
        let bbox = text
            .lines()
            .filter_map(|l| {
                l.strip_prefix("%%HiResBoundingBox:")
                    .or_else(|| l.strip_prefix("%%BoundingBox:"))
            })
            .filter_map(|l| {
                l.split_whitespace()
                    .map(str::parse::<f64>)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()
            })
            .find(|a| a.len() == 4)
            .unwrap_or(vec![0., 0., 100., 100.]);
        let l = *opts.get("llx").unwrap_or(&bbox[0]);
        let b = *opts.get("lly").unwrap_or(&bbox[1]);
        let r = *opts.get("urx").unwrap_or(&bbox[2]);
        let t = *opts.get("ury").unwrap_or(&bbox[3]);
        if r <= l || t <= b {
            return Err("rangecheck: EPS bounding box".into());
        }
        let sx = opts
            .get("rwi")
            .map(|v| v / 10. / (r - l))
            .or_else(|| opts.get("hscale").map(|v| v / 100.))
            .unwrap_or(1.);
        let sy = opts
            .get("rhi")
            .map(|v| v / 10. / (t - b))
            .or_else(|| opts.get("vscale").map(|v| v / 100.))
            .unwrap_or(sx);
        let angle = opts.get("angle").copied().unwrap_or(0.).to_radians();
        let rot = [
            angle.cos(),
            angle.sin(),
            -angle.sin(),
            angle.cos(),
            x + opts.get("hoffset").copied().unwrap_or(0.),
            y + opts.get("voffset").copied().unwrap_or(0.),
        ];
        self.graphics.matrix = multiply(rot, [sx, 0., 0., sy, -l * sx, -b * sy]);
        if args.split_whitespace().any(|w| w == "clip") {
            let p = vec![
                Segment::Move(
                    transform(self.graphics.matrix, l, b).0,
                    transform(self.graphics.matrix, l, b).1,
                ),
                Segment::Line(
                    transform(self.graphics.matrix, r, b).0,
                    transform(self.graphics.matrix, r, b).1,
                ),
                Segment::Line(
                    transform(self.graphics.matrix, r, t).0,
                    transform(self.graphics.matrix, r, t).1,
                ),
                Segment::Line(
                    transform(self.graphics.matrix, l, t).0,
                    transform(self.graphics.matrix, l, t).1,
                ),
                Segment::Close,
            ];
            self.graphics.clips.push((p, false));
        }
        self.execute_loaded(&bytes, load)
    }
    fn execute_loaded(
        &mut self,
        bytes: &[u8],
        load: &mut impl FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<(), String> {
        // A Type 1 font is a data object, not an arbitrary eexec program here.
        // The installed, replaceable FreeType library reads its encrypted outlines.
        if let Some(pos) = bytes
            .windows(b"currentfile eexec".len())
            .position(|w| w == b"currentfile eexec")
        {
            self.execute_bytes(&bytes[..pos])?;
            let Value::Dict(dict) = self.pop()? else {
                return Err("typecheck: Type 1 font dictionary".into());
            };
            let name = dict
                .borrow()
                .get("FontName")
                .ok_or("invalidfont: missing FontName")?
                .name()?;
            let mut data = b"%!PS-AdobeFont-1.0: PitexRuntimeFont 1.0\n".to_vec();
            data.extend_from_slice(bytes);
            self.fonts
                .insert(name.clone(), Rc::new(fonts::Font::new(data)?));
            self.user
                .borrow_mut()
                .entry("FontDirectory".into())
                .or_insert_with(|| Value::Dict(Rc::new(RefCell::new(BTreeMap::new()))));
            let Value::Dict(directory) = self.user.borrow().get("FontDirectory").cloned().unwrap()
            else {
                return Err("invalidfont: FontDirectory".into());
            };
            directory.borrow_mut().insert(name, Value::Dict(dict));
            return Ok(());
        }
        let file:File = Rc::new(RefCell::new((bytes.to_vec(), 0))).into();
        self.vm.set_space(&Value::File(file.clone()),false);
        let previous = self.current_file.replace(file.clone());
        let result = (|| {
            let mut last = None;
            loop {
                let position = file.borrow().1;
                let mut reader = Reader { bytes, pos: position, one: true };
                let tokens = reader.values(None)?;
                file.borrow_mut().1 = reader.pos;
                if tokens.is_empty() { break; }
                let mut probe = last.iter().cloned().collect::<Vec<_>>();
                probe.extend_from_slice(&tokens);
                self.preload_fonts(&probe, load)?;
                last = tokens.last().cloned();
                self.exec_tokens(&tokens)?;
            }
            Ok(())
        })();
        self.current_file = previous;
        result
    }
    fn preload_fonts(
        &mut self,
        tokens: &[Value],
        load: &mut impl FnMut(&str) -> Option<Vec<u8>>,
    ) -> Result<(), String> {
        for (index, value) in tokens.iter().enumerate() {
            if let Value::Proc(p) = value {
                self.preload_fonts(&p.borrow(), load)?;
            }
            if matches!(value,Value::Word(s) if s=="findfont") && index > 0 {
                if let Ok(name) = tokens[index - 1].name() {
                    if !self.fonts.contains_key(&name) {
                        let aliases = match name.as_str() {
                            "Times-Roman" => vec!["utmr8a.pfb", "NimbusRoman-Regular.otf"],
                            "Times-Bold" => vec!["utmb8a.pfb", "NimbusRoman-Bold.otf"],
                            "Times-Italic" => vec!["utmri8a.pfb", "NimbusRoman-Italic.otf"],
                            "Times-BoldItalic" => vec!["utmbi8a.pfb", "NimbusRoman-BoldItalic.otf"],
                            "Helvetica" => vec!["uhvr8a.pfb", "NimbusSans-Regular.otf"],
                            "Helvetica-Bold" => vec!["uhvb8a.pfb", "NimbusSans-Bold.otf"],
                            "Helvetica-Oblique" => vec!["uhvro8a.pfb", "NimbusSans-Italic.otf"],
                            "Helvetica-BoldOblique" => {
                                vec!["uhvbo8a.pfb", "NimbusSans-BoldItalic.otf"]
                            }
                            "Courier" => vec!["ucrr8a.pfb", "NimbusMonoPS-Regular.otf"],
                            "Courier-Bold" => vec!["ucrb8a.pfb", "NimbusMonoPS-Bold.otf"],
                            "Courier-Oblique" => vec!["ucrro8a.pfb", "NimbusMonoPS-Italic.otf"],
                            "Courier-BoldOblique" => {
                                vec!["ucrbo8a.pfb", "NimbusMonoPS-BoldItalic.otf"]
                            }
                            "Symbol" => vec!["usyr.pfb", "StandardSymbolsPS.otf"],
                            "ZapfDingbats" => vec!["uzdr.pfb", "D050000L.otf"],
                            _ => vec![],
                        };
                        let mut data = load(&name);
                        for alias in aliases {
                            if data.is_none() {
                                data = load(alias)
                            }
                        }
                        for ext in ["pfb", "pfa", "otf", "ttf"] {
                            if data.is_none() {
                                data = load(&format!("{name}.{ext}"))
                            }
                        }
                        if let Some(data) = data {
                            self.fonts.insert(name, Rc::new(fonts::Font::new(data)?));
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn current_font(&self) -> Result<(Rc<fonts::Font>, [f64; 6], Option<Array>), String> {
        let dict = self
            .graphics
            .font
            .as_ref()
            .ok_or("invalidfont: no currentfont")?
            .borrow();
        let name = dict.get("FontName").ok_or("invalidfont: name")?.name()?;
        let font = self
            .fonts
            .get(&name)
            .ok_or_else(|| format!("invalidfont: installed font {name} is unavailable"))?
            .clone();
        let matrix = Self::matrix_value(dict.get("FontMatrix").ok_or("invalidfont: matrix")?)?;
        let encoding = match dict.get("Encoding") {
            Some(Value::Array(a)) => Some(a.clone()),
            _ => None,
        };
        Ok((font, matrix, encoding))
    }
    fn glyph_name(encoding: &Option<Array>, code: u8) -> Option<String> {
        encoding.as_ref()?.borrow().get(code as usize)?.name().ok()
    }
    fn text(
        &mut self,
        bytes: &[u8],
        path_only: bool,
        extra: (f64, f64),
        space: Option<(u8, f64, f64)>,
    ) -> Result<(), String> {
        let (font, matrix, encoding) = self.current_font()?;
        let (mut x, mut y) = self.current()?;
        let old_path = self.graphics.path.clone();
        let old_start = self.graphics.start;
        if !path_only {
            self.graphics.path.clear();
            self.graphics.start = None;
        }
        for code in bytes {
            let (width, path) = font.glyph(*code, Self::glyph_name(&encoding, *code).as_deref())?;
            let user = multiply([1., 0., 0., 1., x, y], matrix);
            let device = multiply(self.graphics.matrix, user);
            for p in path {
                match p {
                    fonts::Command::Move(px, py) => {
                        let p = transform(device, px, py);
                        self.graphics.path.push(Segment::Move(p.0, p.1));
                    }
                    fonts::Command::Line(px, py) => {
                        let p = transform(device, px, py);
                        self.graphics.path.push(Segment::Line(p.0, p.1));
                    }
                    fonts::Command::Curve(a) => {
                        let p = transform(device, a[0], a[1]);
                        let q = transform(device, a[2], a[3]);
                        let r = transform(device, a[4], a[5]);
                        self.graphics
                            .path
                            .push(Segment::Curve([p.0, p.1, q.0, q.1, r.0, r.1]));
                    }
                    fonts::Command::Close => self.graphics.path.push(Segment::Close),
                }
            }
            x += matrix[0] * width + extra.0;
            y += matrix[1] * width + extra.1;
            if let Some((c, dx, dy)) = space {
                if c == *code {
                    x += dx;
                    y += dy;
                }
            }
        }
        if !path_only {
            self.paint("f");
            self.graphics.path = old_path;
            self.graphics.start = old_start;
        }
        self.graphics.point = Some(transform(self.graphics.matrix, x, y));
        Ok(())
    }
    fn execute_bytes(&mut self, b: &[u8]) -> Result<(), String> {
        let tokens = Reader { bytes: b, pos: 0, one: false }.values(None)?;
        self.exec_tokens(&tokens)
    }
    fn exec_tokens(&mut self, tokens: &[Value]) -> Result<(), String> {
        self.depth += 1;
        if self.depth > 256 {
            self.depth -= 1;
            return Err("limitcheck: recursion".into());
        }
        let result = (|| {
            for v in tokens {
                let mut v=v.clone();
                if !self.vm.contains(&v) {if let Value::Proc(a)=&mut v {if self.vm.packing {a.packed=true;a.access=Access::ReadOnly;}}}
                self.vm.track(&v);
                if self.exit {
                    break;
                }
                if matches!(v, Value::Proc(_)) {
                    self.stack.push(v.clone())
                } else {
                    self.execute(v.clone())?;
                }
                if self.stack.len() > 100_000 {
                    return Err("stackoverflow".into());
                }
            }
            Ok(())
        })();
        self.depth -= 1;
        result
    }
    fn execute(&mut self, v: Value) -> Result<(), String> {
        self.vm.track(&v);
        if self.budget == 0 {
            return Err("limitcheck: execution budget".into());
        }
        self.budget -= 1;
        self.depth += 1;
        if self.depth > 256 {
            self.depth -= 1;
            return Err("limitcheck: recursive executable alias".into());
        }
        let result = (|| match v {
            Value::Word(n) => {
                for dict in &self.dicts {self.vm.read(&Value::Dict(dict.clone()))?;}
                let found = self
                    .lookup(&n)
                    .ok_or_else(|| format!("undefined operator {n}"))?;
                self.execute(found)
            }
            Value::Op(n) => self.operator(&n),
            Value::Proc(p) => {
                self.vm.execute(&Value::Proc(p.clone()))?;
                self.executing.push(Value::Proc(p.clone()));
                let tokens=p.borrow().to_vec();let result=self.exec_tokens(&tokens);self.executing.pop();result
            },
            _ => {
                self.stack.push(v);
                Ok(())
            }
        })();
        self.depth -= 1;
        result
    }
    fn lookup(&self, n: &str) -> Option<Value> {
        self.dicts
            .iter()
            .rev()
            .find_map(|d| d.borrow().get(n).cloned())
    }
    fn bind_procedure(&self, value: Value) -> Value {
        match value {
            Value::Proc(values) => {
                if values.write().is_ok(){let source=values.borrow().to_vec();let bound=source.into_iter().map(|v|self.bind_procedure(v)).collect::<Vec<_>>();values.borrow_mut().clone_from_slice(&bound);}
                Value::Proc(values)
            }
            Value::Word(name) => match self.lookup(&name) {
                Some(Value::Op(operator)) => Value::Op(operator),
                _ => Value::Word(name),
            },
            other => other,
        }
    }
    fn pop(&mut self) -> Result<Value, String> {
        self.stack.pop().ok_or_else(|| "stackunderflow".into())
    }
    fn num(&mut self) -> Result<f64, String> {
        self.pop()?.number()
    }
    fn nums(&mut self, n: usize) -> Result<Vec<f64>, String> {
        let mut v = (0..n).map(|_| self.num()).collect::<Result<Vec<_>, _>>()?;
        v.reverse();
        Ok(v)
    }
    fn pushnum(&mut self, n: f64) -> Result<(), String> {
        if !n.is_finite() {
            return Err("undefinedresult: non-finite arithmetic".into());
        }
        self.stack.push(Value::Number(n));
        Ok(())
    }
    fn array(&mut self) -> Result<Array, String> {
        if let Value::Array(a) | Value::Proc(a) = self.pop()? {
            a.read()?;Ok(a)
        } else {
            Err("typecheck: array".into())
        }
    }
    fn matrix_value(v: &Value) -> Result<[f64; 6], String> {
        if let Value::Array(a) | Value::Proc(a) = v {
            a.read()?;let a = a.borrow();
            if a.len() == 6 {
                return Ok([
                    a[0].number()?,
                    a[1].number()?,
                    a[2].number()?,
                    a[3].number()?,
                    a[4].number()?,
                    a[5].number()?,
                ]);
            }
        }
        Err("typecheck: matrix".into())
    }
    fn set_array_matrix(a: &Array, m: [f64; 6])->Result<(),String> {
        a.write()?;if a.borrow().len()<6{return Err("rangecheck: matrix destination".into());}
        a.borrow_mut()[..6].clone_from_slice(&m.map(Value::Number));Ok(())
    }
    fn move_to(&mut self, x: f64, y: f64, line: bool) {
        let (x, y) = transform(self.graphics.matrix, x, y);
        if line {
            self.graphics.path.push(Segment::Line(x, y))
        } else {
            self.graphics.path.push(Segment::Move(x, y));
            self.graphics.start = Some((x, y));
        }
        self.graphics.point = Some((x, y));
    }
    fn current(&self) -> Result<(f64, f64), String> {
        let (x, y) = self.graphics.point.ok_or("nocurrentpoint")?;
        Ok(transform(inverse(self.graphics.matrix)?, x, y))
    }
    fn curve(&mut self, a: [f64; 6]) {
        let p = transform(self.graphics.matrix, a[0], a[1]);
        let q = transform(self.graphics.matrix, a[2], a[3]);
        let r = transform(self.graphics.matrix, a[4], a[5]);
        self.graphics
            .path
            .push(Segment::Curve([p.0, p.1, q.0, q.1, r.0, r.1]));
        self.graphics.point = Some(r);
    }
    fn paint(&mut self, op: &str) {
        let scale = (self.graphics.matrix[0] * self.graphics.matrix[3]
            - self.graphics.matrix[1] * self.graphics.matrix[2])
            .abs()
            .sqrt();
        self.output += "q\n";
        for (p, even) in &self.graphics.clips {
            self.output += &format_path(p);
            self.output += if *even { "W* n\n" } else { "W n\n" };
        }
        self.output += &format!(
            "{}\n{:.6} w\n{} J {} j {:.6} M\n[{}] {:.6} d\n",
            self.graphics.color,
            self.graphics.width * scale,
            self.graphics.cap,
            self.graphics.join,
            self.graphics.miter,
            self.graphics
                .dash
                .iter()
                .map(|d| format!("{:.6}", d * scale))
                .collect::<Vec<_>>()
                .join(" "),
            self.graphics.phase * scale
        );
        if self.graphics.fill_alpha != 1. || self.graphics.stroke_alpha != 1. {
            let key = self
                .alphas
                .iter()
                .find(|(_, f, s)| {
                    *f == self.graphics.fill_alpha && *s == self.graphics.stroke_alpha
                })
                .map(|(n, _, _)| n.clone())
                .unwrap_or_else(|| {
                    let n = format!("PSTalpha{}", self.alphas.len());
                    self.alphas.push((
                        n.clone(),
                        self.graphics.fill_alpha,
                        self.graphics.stroke_alpha,
                    ));
                    n
                });
            self.output += &format!("/{key} gs\n");
        }
        self.output += &format_path(&self.graphics.path);
        self.output += op;
        self.output += "\nQ\n";
        self.graphics.path.clear();
        self.graphics.point = None;
        self.graphics.start = None;
    }
    fn arc(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        start: f64,
        end: f64,
        clockwise: bool,
    ) -> Result<(), String> {
        if r < 0. {
            return Err("rangecheck: negative arc radius".into());
        }
        let mut sweep = end - start;
        if clockwise {
            while sweep > 0. {
                sweep -= 360.
            }
            sweep = sweep.max(-360.);
        } else {
            while sweep < 0. {
                sweep += 360.
            }
            sweep = sweep.min(360.);
        }
        let n = (sweep.abs() / 90.).ceil().max(1.) as usize;
        let step = sweep.to_radians() / n as f64;
        let mut a = start.to_radians();
        let (px, py) = (x + r * a.cos(), y + r * a.sin());
        self.move_to(px, py, self.graphics.point.is_some());
        for _ in 0..n {
            let b = a + step;
            let k = 4. / 3. * (step / 4.).tan();
            self.curve([
                x + r * (a.cos() - k * a.sin()),
                y + r * (a.sin() + k * a.cos()),
                x + r * (b.cos() + k * b.sin()),
                y + r * (b.sin() - k * b.cos()),
                x + r * b.cos(),
                y + r * b.sin(),
            ]);
            a = b;
        }
        Ok(())
    }
    fn guard_operator(&self,op:&str)->Result<(),String>{
        let get=|depth:usize|self.stack.get(self.stack.len().checked_sub(depth+1).unwrap_or(usize::MAX)).ok_or_else(||"stackunderflow".to_string());
        match op {
            "get"|"known"=>self.vm.read(get(1)?)?,
            "length"|"aload"|"begin"|"stringwidth"|"show"|"ashow"|"charpath"|"setdash"|"setmatrix"|"concat"|"shfill"=>self.vm.read(get(0)?)?,
            "getinterval"=>self.vm.read(get(2)?)?,
            "putinterval"=>{self.vm.write(get(2)?)?;self.vm.read(get(0)?)?;},
            "forall"=>self.vm.read(get(1)?)?,
            "copy" if !matches!(get(0)?,Value::Number(_))=>{self.vm.read(get(1)?)?;self.vm.write(get(0)?)?;},
            "astore"|"currentmatrix"|"defaultmatrix"|"concatmatrix"|"invertmatrix"=>self.vm.write(get(0)?)?,
            _=>{}
        }
        Ok(())
    }
    fn operator(&mut self, op: &str) -> Result<(), String> {
        self.guard_operator(op)?;
        match op {
            "findfont" => {
                let name = self.pop()?.name()?;
                if let Some(Value::Dict(directory)) = self.lookup("FontDirectory") {
                    if let Some(font) = directory.borrow().get(&name).cloned() {
                        self.stack.push(font);
                        return Ok(());
                    }
                }
                let units = self
                    .fonts
                    .get(&name)
                    .ok_or_else(|| format!("invalidfont: installed font {name} is unavailable"))?
                    .units;
                let mut dict = BTreeMap::new();
                dict.insert("FontName".into(), Value::Name(name));
                dict.insert(
                    "FontMatrix".into(),
                    Value::Array(Rc::new(RefCell::new(
                        [1. / units, 0., 0., 1. / units, 0., 0.]
                            .into_iter()
                            .map(Value::Number)
                            .collect(),
                    )).into()),
                );
                self.stack.push(Value::Dict(Rc::new(RefCell::new(dict))));
            }
            "scalefont" | "makefont" => {
                let scale = if op == "scalefont" {
                    let size = self.num()?;
                    [size, 0., 0., size, 0., 0.]
                } else {
                    Self::matrix_value(&self.pop()?)?
                };
                let Value::Dict(dict) = self.pop()? else {
                    return Err("typecheck: font".into());
                };
                let mut dict = dict.borrow().clone();
                let m = Self::matrix_value(dict.get("FontMatrix").ok_or("invalidfont: matrix")?)?;
                dict.insert(
                    "FontMatrix".into(),
                    Value::Array(Rc::new(RefCell::new(
                        multiply(scale, m).into_iter().map(Value::Number).collect(),
                    )).into()),
                );
                self.stack.push(Value::Dict(Rc::new(RefCell::new(dict))));
            }
            "setfont" => {
                let Value::Dict(font) = self.pop()? else {
                    return Err("typecheck: setfont".into());
                };
                self.graphics.font = Some(font);
            }
            "currentfont" => {
                self.stack.push(Value::Dict(
                    self.graphics
                        .font
                        .as_ref()
                        .ok_or("invalidfont: currentfont")?
                        .clone(),
                ));
            }
            "definefont" => {
                let Value::Dict(font) = self.pop()? else {
                    return Err("typecheck: definefont".into());
                };
                let name = self.pop()?.name()?;
                self.user
                    .borrow_mut()
                    .entry("FontDirectory".into())
                    .or_insert_with(|| Value::Dict(Rc::new(RefCell::new(BTreeMap::new()))));
                let Value::Dict(directory) = self.lookup("FontDirectory").unwrap() else {
                    return Err("invalidfont: FontDirectory".into());
                };
                directory
                    .borrow_mut()
                    .insert(name, Value::Dict(font.clone()));
                self.stack.push(Value::Dict(font));
            }
            "undefinefont" => {
                let name = self.pop()?.name()?;
                if let Some(Value::Dict(directory)) = self.lookup("FontDirectory") {
                    directory.borrow_mut().remove(&name);
                }
            }
            "stringwidth" => {
                let Value::String(bytes) = self.pop()? else {
                    return Err("typecheck: stringwidth".into());
                };
                bytes.read()?;
                let (font, matrix, encoding) = self.current_font()?;
                let mut width = 0.;
                for code in bytes.borrow().iter() {
                    width += font
                        .glyph(*code, Self::glyph_name(&encoding, *code).as_deref())?
                        .0;
                }
                self.pushnum(matrix[0] * width)?;
                self.pushnum(matrix[1] * width)?;
            }
            "show" | "ashow" | "widthshow" | "awidthshow" | "charpath" => {
                let path_only = op == "charpath";
                if path_only {
                    self.pop()?.boolean()?;
                }
                let Value::String(bytes) = self.pop()? else {
                    return Err("typecheck: show".into());
                };
                bytes.read()?;
                let extra = if op == "ashow" || op == "awidthshow" {
                    let a = self.nums(2)?;
                    (a[0], a[1])
                } else {
                    (0., 0.)
                };
                let space = if op == "widthshow" || op == "awidthshow" {
                    let code = self.num()? as u8;
                    let a = self.nums(2)?;
                    Some((code, a[0], a[1]))
                } else {
                    None
                };
                self.text(&bytes.borrow(), path_only, extra, space)?;
            }
            "add" | "sub" | "mul" | "div" | "idiv" | "mod" | "exp" | "atan" => {
                let a = self.nums(2)?;
                let n = match op {
                    "add" => a[0] + a[1],
                    "sub" => a[0] - a[1],
                    "mul" => a[0] * a[1],
                    "div" => a[0] / a[1],
                    "idiv" => (a[0] / a[1]).trunc(),
                    "mod" => a[0] % a[1],
                    "exp" => a[0].powf(a[1]),
                    _ => (a[0].atan2(a[1]).to_degrees() + 360.) % 360.,
                };
                self.pushnum(n)?;
            }
            "neg" | "abs" | "sqrt" | "ln" | "log" | "sin" | "cos" | "floor" | "ceiling"
            | "round" | "truncate" | "cvi" | "cvr" => {
                let x = self.num()?;
                self.pushnum(match op {
                    "neg" => -x,
                    "abs" => x.abs(),
                    "sqrt" => x.sqrt(),
                    "ln" => x.ln(),
                    "log" => x.log10(),
                    "sin" => x.to_radians().sin(),
                    "cos" => x.to_radians().cos(),
                    "floor" => x.floor(),
                    "ceiling" => x.ceil(),
                    "round" => (x + 0.5).floor(),
                    "truncate" | "cvi" => x.trunc(),
                    _ => x,
                })?;
            }
            "dup" => {
                let v = self.stack.last().ok_or("stackunderflow")?.clone();
                self.stack.push(v)
            }
            "exch" => {
                let a = self.pop()?;
                let b = self.pop()?;
                self.stack.push(a);
                self.stack.push(b)
            }
            "pop" => {
                self.pop()?;
            }
            "clear" => self.stack.clear(),
            "count" => self.pushnum(self.stack.len() as f64)?,
            "index" => {
                let n = self.num()? as usize;
                if n >= self.stack.len() {
                    return Err("rangecheck: index".into());
                }
                self.stack
                    .push(self.stack[self.stack.len() - n - 1].clone())
            }
            "roll" => {
                let j = self.num()? as i64;
                let n = self.num()? as usize;
                if n > self.stack.len() {
                    return Err("rangecheck: roll".into());
                }
                if n != 0 {
                    let len = self.stack.len();
                    self.stack[len - n..].rotate_right(j.rem_euclid(n as i64) as usize);
                }
            }
            "copy" => {
                let dest = self.pop()?;
                if let Value::Number(n) = dest {
                    let n = n as usize;
                    if n > self.stack.len() {
                        return Err("rangecheck: copy".into());
                    }
                    self.stack
                        .extend(self.stack[self.stack.len() - n..].to_vec());
                } else {
                    let src = self.pop()?;
                    match &src {Value::Array(a)|Value::Proc(a)=>for item in a.borrow().iter(){self.vm.store(&dest,item)?;},Value::Dict(d)=>for item in d.borrow().values(){self.vm.store(&dest,item)?;},_=>{}}
                    match (&src, &dest) {
                        (Value::Dict(a), Value::Dict(b)) => {
                            let values=a.borrow().clone();b.borrow_mut().extend(values)
                        }
                        (Value::Array(a), Value::Array(b)) => {
                            let a = a.borrow().to_vec();
                            if a.len() > b.borrow().len() {
                                return Err("rangecheck: array copy".into());
                            }
                            b.borrow_mut()[..a.len()].clone_from_slice(&a)
                        }
                        (Value::String(a),Value::String(b))=>{let values=a.borrow().to_vec();if values.len()>b.borrow().len(){return Err("rangecheck: string copy".into());}b.borrow_mut()[..values.len()].copy_from_slice(&values);},
                        _ => return Err("typecheck: copy".into()),
                    }
                    self.stack.push(dest)
                }
            }
            "mark" | "[" | "<<" => self.stack.push(Value::Mark),
            "cleartomark" => while !matches!(self.pop()?, Value::Mark) {},
            "counttomark" => {
                let n = self
                    .stack
                    .iter()
                    .rev()
                    .position(|v| matches!(v, Value::Mark))
                    .ok_or("unmatchedmark")?;
                self.pushnum(n as f64)?
            }
            "]" | ">>" => {
                let mut a = Vec::new();
                loop {
                    let v = self.pop()?;
                    if matches!(v, Value::Mark) {
                        break;
                    }
                    a.push(v)
                }
                a.reverse();
                if op == "]" {
                    let value=Value::Array(a.clone().into());for item in a {self.vm.store(&value,&item)?;}self.stack.push(value)
                } else {
                    if a.len() % 2 != 0 {
                        return Err("rangecheck: dictionary pairs".into());
                    }
                    let mut d = BTreeMap::new();
                    for pair in a.chunks(2) {
                        d.entry(pair[0].name()?).or_insert_with(||pair[1].clone());
                    }
                    let value=Value::Dict(Rc::new(RefCell::new(d)));if let Value::Dict(dict)=&value{for item in dict.borrow().values(){self.vm.store(&value,item)?;}}self.stack.push(value)
                }
            }
            "dict" => {
                self.num()?;
                self.stack
                    .push(Value::Dict(Rc::new(RefCell::new(BTreeMap::new()))))
            }
            "begin" => {
                if let Value::Dict(d) = self.pop()? {
                    self.dicts.push(d)
                } else {
                    return Err("typecheck: begin".into());
                }
            }
            "end" => {
                if self.dicts.len() <= 3 {
                    return Err("dictstackunderflow".into());
                }
                self.dicts.pop();
            }
            "def" | "store" => {
                let v = self.pop()?;
                let key = self.pop()?.name()?;
                let d = if op == "store" {
                    self.dicts
                        .iter()
                        .rev()
                        .find(|d| d.borrow().contains_key(&key))
                        .unwrap_or(self.dicts.last().unwrap())
                } else {
                    self.dicts.last().unwrap()
                };
                self.vm.store(&Value::Dict(d.clone()),&v)?;
                d.borrow_mut().insert(key, v);
            }
            "load" => {
                let k = self.pop()?.name()?;
                let v = self.lookup(&k).ok_or_else(|| format!("undefined: {k}"))?;
                self.stack.push(v)
            }
            "where" => {
                let k = self.pop()?.name()?;
                let d = self
                    .dicts
                    .iter()
                    .rev()
                    .find(|d| d.borrow().contains_key(&k))
                    .cloned();
                if let Some(d) = d {
                    self.stack.push(Value::Dict(d));
                    self.stack.push(Value::Bool(true))
                } else {
                    self.stack.push(Value::Bool(false))
                }
            }
            "known" => {
                let k = self.pop()?.name()?;
                let d = self.pop()?;
                if let Value::Dict(d) = d {
                    let b = d.borrow().contains_key(&k);
                    self.stack.push(Value::Bool(b))
                } else {
                    return Err("typecheck: known".into());
                }
            }
            "currentdict" => self
                .stack
                .push(Value::Dict(self.dicts.last().unwrap().clone())),
            "systemdict" => self.stack.push(Value::Dict(self.system.clone())),
            "userdict" => self.stack.push(Value::Dict(self.user.clone())),
            "globaldict" => self.stack.push(Value::Dict(self.global.clone())),
            "statusdict" => self
                .stack
                .push(Value::Dict(Rc::new(RefCell::new(BTreeMap::new())))),
            "get" => {
                let k = self.pop()?;
                let a = self.pop()?;
                let v = match a {
                    Value::Dict(d) => d
                        .borrow()
                        .get(&k.name()?)
                        .cloned()
                        .ok_or("undefined: dictionary key")?,
                    Value::Array(a) | Value::Proc(a) => a
                        .borrow()
                        .get(k.number()? as usize)
                        .cloned()
                        .ok_or("rangecheck: array get")?,
                    Value::String(s) => Value::Number(
                        *s.borrow()
                            .get(k.number()? as usize)
                            .ok_or("rangecheck: string get")? as f64,
                    ),
                    _ => return Err("typecheck: get".into()),
                };
                self.stack.push(v)
            }
            "put" => {
                let v = self.pop()?;
                let k = self.pop()?;
                let destination=self.pop()?;self.vm.store(&destination,&v)?;
                match destination {
                    Value::Dict(d) => {
                        d.borrow_mut().insert(k.name()?, v);
                    }
                    Value::Array(a) | Value::Proc(a) => {
                        *a.borrow_mut()
                            .get_mut(k.number()? as usize)
                            .ok_or("rangecheck: array put")? = v;
                    }
                    Value::String(s) => {
                        *s.borrow_mut()
                            .get_mut(k.number()? as usize)
                            .ok_or("rangecheck: string put")? = v.number()? as u8;
                    }
                    _ => return Err("typecheck: put".into()),
                }
            }
            "length" => {
                let n = match self.pop()? {
                    Value::Array(a) => a.borrow().len(),
                    Value::Proc(p) => p.borrow().len(),
                    Value::String(s) => s.borrow().len(),
                    Value::Dict(d) => d.borrow().len(),
                    Value::Name(n) => n.len(),
                    _ => return Err("typecheck: length".into()),
                };
                self.pushnum(n as f64)?
            }
            "array" | "string" => {
                let n = self.num()? as usize;
                if n > 1_000_000 {
                    return Err("limitcheck: allocation".into());
                }
                self.stack.push(if op == "array" {
                    Value::Array(Rc::new(RefCell::new(vec![Value::Null; n])).into())
                } else {
                    Value::String(Rc::new(RefCell::new(vec![0; n])).into())
                })
            }
            "aload" => {
                let a = self.array()?;
                self.stack.extend(a.borrow().to_vec());
                self.stack.push(Value::Array(a))
            }
            "astore" => {
                let a = self.array()?;a.write()?;
                let n = a.borrow().len();
                if n > self.stack.len() {
                    return Err("stackunderflow".into());
                }
                let values=self.stack.split_off(self.stack.len()-n);for item in &values{self.vm.store(&Value::Array(a.clone()),item)?;}a.borrow_mut().clone_from_slice(&values);
                self.stack.push(Value::Array(a))
            }
            "getinterval" => {
                let n=self.num()? as usize;let i=self.num()? as usize;
                let value=match self.pop()?{Value::Array(a)=>Value::Array(a.interval(i,n)?),Value::Proc(a)=>Value::Proc(a.interval(i,n)?),Value::String(s)=>Value::String(s.interval(i,n)?),_=>return Err("typecheck: getinterval".into())};
                self.stack.push(value);
            }
            "putinterval" => {
                let v = self.pop()?;
                let i = self.num()? as usize;
                let a = self.pop()?;
                if let Value::Array(values)|Value::Proc(values)=&v {for item in values.borrow().iter(){self.vm.store(&a,item)?;}}
                match (a, v) {
                    (Value::Array(a), Value::Array(b)) => {
                        let b = b.borrow().to_vec();
                        a.borrow_mut()
                            .get_mut(i..i.checked_add(b.len()).ok_or("rangecheck")?)
                            .ok_or("rangecheck: putinterval")?
                            .clone_from_slice(&b)
                    }
                    (Value::String(a), Value::String(b)) => {
                        let b = b.borrow().to_vec();
                        a.borrow_mut()
                            .get_mut(i..i.checked_add(b.len()).ok_or("rangecheck")?)
                            .ok_or("rangecheck: putinterval")?
                            .copy_from_slice(&b)
                    }
                    _ => return Err("typecheck: putinterval".into()),
                }
            }
            "exec" => {
                let p = self.pop()?;self.vm.execute(&p)?;
                match p {
                    Value::String(s) => {
                        let b = s.borrow().to_vec();
                        self.execute_bytes(&b)?
                    }
                    _ => self.execute(p)?,
                }
            }
            "bind" => {
                let procedure = self.pop()?;
                if !matches!(procedure, Value::Proc(_)) {
                    return Err("typecheck: bind".into());
                }
                self.vm.execute(&procedure)?;let bound=self.bind_procedure(procedure);self.stack.push(bound);
            }
            "readonly" | "executeonly" | "noaccess" => {
                let mut value=self.pop()?;let access=match op {"readonly"=>Access::ReadOnly,"executeonly"=>Access::ExecuteOnly,_=>Access::None};
                self.vm.restrict(&mut value,access)?;self.stack.push(value);
            }
            "rcheck" | "wcheck" | "gcheck" => {let value=self.pop()?;if op!="gcheck"&&!matches!(value,Value::Array(_)|Value::Proc(_)|Value::String(_)|Value::Dict(_)|Value::File(_)){return Err("typecheck: access check".into());}let allowed=match op {"rcheck"=>self.vm.rcheck(&value),"wcheck"=>self.vm.wcheck(&value),_=>self.vm.global(&value)};self.stack.push(Value::Bool(allowed));}
            "cvx" => {
                let v = self.pop()?;
                self.stack.push(match v {
                    Value::Name(n) => Value::Word(n),
                    Value::Array(a) => Value::Proc(a),
                    v => v,
                })
            }
            "cvlit" => {
                let v = self.pop()?;
                self.stack.push(match v {
                    Value::Word(n) | Value::Op(n) => Value::Name(n),
                    Value::Proc(p) => Value::Array(p),
                    v => v,
                })
            }
            "xcheck" => {
                let v = self.pop()?;
                self.stack.push(Value::Bool(matches!(
                    v,
                    Value::Word(_) | Value::Proc(_) | Value::Op(_)
                )))
            }
            "type" => {
                let v = self.pop()?;
                self.stack.push(Value::Name(
                    match v {
                        Value::Number(n) if n.fract() == 0. => "integertype",
                        Value::Number(_) => "realtype",
                        Value::Bool(_) => "booleantype",
                        Value::Name(_) | Value::Word(_) => "nametype",
                        Value::Op(_) => "operatortype",
                        Value::Array(a) | Value::Proc(a) => if a.packed {"packedarraytype"} else {"arraytype"},
                        Value::String(_) => "stringtype",
                        Value::File(_) => "filetype",
                        Value::Dict(_) => "dicttype",
                        Value::Save(_) => "savetype",
                        Value::Mark => "marktype",
                        Value::Null => "nulltype",
                    }
                    .into(),
                ))
            }
            "true" | "false" => self.stack.push(Value::Bool(op == "true")),
            "null" => self.stack.push(Value::Null),
            "eq" | "ne" | "lt" | "le" | "gt" | "ge" => {
                let b = self.pop()?;
                let a = self.pop()?;
                if matches!(a,Value::String(_)){self.vm.read(&a)?;}if matches!(b,Value::String(_)){self.vm.read(&b)?;}
                let cmp = match (&a, &b) {
                    (Value::Number(a), Value::Number(b)) => a.partial_cmp(b),
                    (Value::Bool(a), Value::Bool(b)) => a.partial_cmp(b),
                    (Value::Name(a) | Value::Word(a), Value::Name(b) | Value::Word(b)) => {
                        a.partial_cmp(b)
                    }
                    (Value::String(a), Value::String(b)) => a.borrow().partial_cmp(&b.borrow()),
                    (Value::Save(a),Value::Save(b))=>a.partial_cmp(b),
                    (Value::File(a),Value::File(b)) if Rc::ptr_eq(&a.data,&b.data)=>Some(std::cmp::Ordering::Equal),
                    (Value::Dict(a), Value::Dict(b)) if Rc::ptr_eq(a, b) => {
                        Some(std::cmp::Ordering::Equal)
                    }
                    (Value::Array(a)|Value::Proc(a), Value::Array(b)|Value::Proc(b)) if a.same(b) => {
                        Some(std::cmp::Ordering::Equal)
                    }
                    (Value::Null, Value::Null) => Some(std::cmp::Ordering::Equal),
                    _ => None,
                };
                let value = match op {
                    "eq" => cmp == Some(std::cmp::Ordering::Equal),
                    "ne" => cmp != Some(std::cmp::Ordering::Equal),
                    _ => {
                        let c = cmp.ok_or("typecheck: comparison")?;
                        match op {
                            "lt" => c.is_lt(),
                            "le" => !c.is_gt(),
                            "gt" => c.is_gt(),
                            _ => !c.is_lt(),
                        }
                    }
                };
                self.stack.push(Value::Bool(value))
            }
            "and" | "or" | "xor" => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.stack.push(match (a, b) {
                    (Value::Bool(a), Value::Bool(b)) => Value::Bool(match op {
                        "and" => a && b,
                        "or" => a || b,
                        _ => a ^ b,
                    }),
                    (Value::Number(a), Value::Number(b)) => Value::Number(match op {
                        "and" => (a as i64) & (b as i64),
                        "or" => (a as i64) | (b as i64),
                        _ => (a as i64) ^ (b as i64),
                    }
                        as f64),
                    _ => return Err("typecheck: logical".into()),
                })
            }
            "not" => {
                let v = self.pop()?;
                self.stack.push(match v {
                    Value::Bool(b) => Value::Bool(!b),
                    Value::Number(n) => Value::Number(!(n as i64) as f64),
                    _ => return Err("typecheck: not".into()),
                })
            }
            "bitshift" => {
                let n = self.num()? as i32;
                let v = self.num()? as i64;
                self.pushnum(if n >= 0 {
                    v.checked_shl(n as u32).unwrap_or(0)
                } else {
                    ((v as u64).checked_shr((-n) as u32).unwrap_or(0)) as i64
                } as f64)?
            }
            "if" => {
                let p = self.pop()?;
                if self.pop()?.boolean()? {
                    self.execute(p)?
                }
            }
            "ifelse" => {
                let b = self.pop()?;
                let a = self.pop()?;
                let condition = self.pop()?.boolean()?;
                self.execute(if condition { a } else { b })?
            }
            "repeat" => {
                let p = self.pop()?;
                let n = self.num()? as usize;
                if n > 2_000_000 {
                    return Err("limitcheck: repeat".into());
                }
                for _ in 0..n {
                    self.execute(p.clone())?;
                    if self.exit {
                        self.exit = false;
                        break;
                    }
                }
            }
            "for" => {
                let p = self.pop()?;
                let limit = self.num()?;
                let step = self.num()?;
                let mut x = self.num()?;
                if step == 0. {
                    return Err("rangecheck: zero loop increment".into());
                }
                while if step > 0. { x <= limit } else { x >= limit } {
                    self.pushnum(x)?;
                    self.execute(p.clone())?;
                    if self.exit {
                        self.exit = false;
                        break;
                    }
                    x += step;
                }
            }
            "loop" => {
                let p = self.pop()?;
                loop {
                    self.execute(p.clone())?;
                    if self.exit {
                        self.exit = false;
                        break;
                    }
                }
            }
            "exit" => self.exit = true,
            "forall" => {
                let p = self.pop()?;
                let a = self.pop()?;
                let items: Vec<Vec<Value>> = match a {
                    Value::Array(a) => a.borrow().iter().cloned().map(|v| vec![v]).collect(),
                    Value::String(s) => s
                        .borrow()
                        .iter()
                        .map(|b| vec![Value::Number(*b as f64)])
                        .collect(),
                    Value::Dict(d) => d
                        .borrow()
                        .iter()
                        .map(|(k, v)| vec![Value::Name(k.clone()), v.clone()])
                        .collect(),
                    _ => return Err("typecheck: forall".into()),
                };
                for v in items {
                    self.stack.extend(v);
                    self.execute(p.clone())?;
                    if self.exit {
                        self.exit = false;
                        break;
                    }
                }
            }
            "stopped" => {
                let p = self.pop()?;
                let failed = self.execute(p).is_err();
                self.stack.push(Value::Bool(failed))
            }
            "stop" => return Err("stop".into()),
            "matrix" => self.stack.push(Value::Array(Rc::new(RefCell::new(
                [1., 0., 0., 1., 0., 0.]
                    .into_iter()
                    .map(Value::Number)
                    .collect(),
            )).into())),
            "currentmatrix" | "defaultmatrix" => {
                let a = self.array()?;
                Self::set_array_matrix(
                    &a,
                    if op == "currentmatrix" {
                        self.graphics.matrix
                    } else {
                        [1., 0., 0., 1., 0., 0.]
                    },
                )?;
                self.stack.push(Value::Array(a))
            }
            "setmatrix" => {
                let v = self.pop()?;
                self.graphics.matrix = Self::matrix_value(&v)?
            }
            "concat" => {
                let v = self.pop()?;
                self.graphics.matrix = multiply(self.graphics.matrix, Self::matrix_value(&v)?)
            }
            "translate" | "scale" | "rotate" => {
                let array = if matches!(self.stack.last(), Some(Value::Array(_))) {
                    Some(self.array()?)
                } else {
                    None
                };
                let m = if op == "rotate" {
                    let angle = self.num()?;
                    // PST globals use the DVI rotation convention, while
                    // isolated PostScript literals use PostScript's axes.
                    let a = (if self.dvi_global { -angle } else { angle }).to_radians();
                    [a.cos(), a.sin(), -a.sin(), a.cos(), 0., 0.]
                } else {
                    let a = self.nums(2)?;
                    if op == "scale" {
                        [a[0], 0., 0., a[1], 0., 0.]
                    } else {
                        [1., 0., 0., 1., a[0], a[1]]
                    }
                };
                if let Some(a) = array {
                    Self::set_array_matrix(&a, m)?;
                    self.stack.push(Value::Array(a))
                } else {
                    self.graphics.matrix = multiply(self.graphics.matrix, m)
                }
            }
            "transform" | "dtransform" | "itransform" | "idtransform" => {
                let mut m = if matches!(self.stack.last(), Some(Value::Array(_))) {
                    Self::matrix_value(&self.pop()?)?
                } else {
                    self.graphics.matrix
                };
                if op.starts_with('i') {
                    m = inverse(m)?
                }
                if op.contains("dtransform") {
                    m[4] = 0.;
                    m[5] = 0.;
                }
                let a = self.nums(2)?;
                let p = transform(m, a[0], a[1]);
                self.pushnum(p.0)?;
                self.pushnum(p.1)?
            }
            "concatmatrix" => {
                let dest = self.array()?;
                let b = Self::matrix_value(&self.pop()?)?;
                let a = Self::matrix_value(&self.pop()?)?;
                Self::set_array_matrix(&dest, multiply(b, a))?;
                self.stack.push(Value::Array(dest))
            }
            "invertmatrix" => {
                let dest = self.array()?;
                let a = Self::matrix_value(&self.pop()?)?;
                Self::set_array_matrix(&dest, inverse(a)?)?;
                self.stack.push(Value::Array(dest))
            }
            "newpath" => {
                self.graphics.path.clear();
                self.graphics.point = None;
                self.graphics.start = None;
            }
            "moveto" | "lineto" => {
                let a = self.nums(2)?;
                if op == "lineto" && self.graphics.point.is_none() {
                    return Err("nocurrentpoint".into());
                }
                self.move_to(a[0], a[1], op == "lineto")
            }
            "rmoveto" | "rlineto" => {
                let a = self.nums(2)?;
                let p = self.current()?;
                self.move_to(p.0 + a[0], p.1 + a[1], op == "rlineto")
            }
            "curveto" | "rcurveto" => {
                let a = self.nums(6)?;
                let p = if op == "rcurveto" {
                    self.current()?
                } else {
                    (0., 0.)
                };
                self.curve([
                    a[0] + p.0,
                    a[1] + p.1,
                    a[2] + p.0,
                    a[3] + p.1,
                    a[4] + p.0,
                    a[5] + p.1,
                ])
            }
            "closepath" => {
                self.graphics.path.push(Segment::Close);
                self.graphics.point = self.graphics.start
            }
            "currentpoint" => {
                let p = self.current()?;
                self.pushnum(p.0)?;
                self.pushnum(p.1)?
            }
            "arc" | "arcn" => {
                let a = self.nums(5)?;
                self.arc(a[0], a[1], a[2], a[3], a[4], op == "arcn")?
            }
            "arct" | "arcto" => {
                let a = self.nums(5)?;
                let p = self.current()?;
                let v1 = (p.0 - a[0], p.1 - a[1]);
                let v2 = (a[2] - a[0], a[3] - a[1]);
                let n1 = v1.0.hypot(v1.1);
                let n2 = v2.0.hypot(v2.1);
                if n1 == 0. || n2 == 0. {
                    return Err("undefinedresult: arc tangent".into());
                }
                let u = (v1.0 / n1, v1.1 / n1);
                let v = (v2.0 / n2, v2.1 / n2);
                let theta = (u.0 * v.0 + u.1 * v.1).clamp(-1., 1.).acos();
                let dist = a[4] / (theta / 2.).tan();
                let t1 = (a[0] + u.0 * dist, a[1] + u.1 * dist);
                let t2 = (a[0] + v.0 * dist, a[1] + v.1 * dist);
                let bis = (u.0 + v.0, u.1 + v.1);
                let norm = bis.0.hypot(bis.1);
                if norm < 1e-12 || a[4] == 0. {
                    self.move_to(a[0], a[1], true)
                } else {
                    let cdist = a[4] / (theta / 2.).sin();
                    let center = (a[0] + bis.0 / norm * cdist, a[1] + bis.1 / norm * cdist);
                    let s = (t1.1 - center.1).atan2(t1.0 - center.0).to_degrees();
                    let e = (t2.1 - center.1).atan2(t2.0 - center.0).to_degrees();
                    self.arc(center.0, center.1, a[4], s, e, u.0 * v.1 - u.1 * v.0 > 0.)?;
                }
                if op == "arcto" {
                    for n in [t1.0, t1.1, t2.0, t2.1] {
                        self.pushnum(n)?;
                    }
                }
            }
            "gsave" => self.saved.push(self.graphics.clone()),
            "grestore" => {
                let floor=self.vm.graphics_floor();
                if self.saved.len()>floor {self.graphics=self.saved.pop().unwrap();}
                else if let Some(saved)=self.saved.last(){self.graphics=saved.clone();}
                else{self.graphics=Graphics::default();}
            },
            "save" => {
                for value in self.stack.clone(){self.vm.scan(&value);}for dict in self.dicts.clone(){self.vm.scan(&Value::Dict(dict));}
                if let Some(font)=&self.graphics.font {self.vm.scan(&Value::Dict(font.clone()));}
                let id=self.vm.save(&self.graphics,&self.saved);self.saved.push(self.graphics.clone());self.stack.push(Value::Save(id));
            },
            "restore" => {
                let Value::Save(id)=self.pop()?else{return Err("typecheck: restore expects save object".into());};
                let mut roots=self.stack.clone();roots.extend(self.dicts.iter().cloned().map(Value::Dict));roots.extend(self.executing.clone());
                let (graphics,saved)=self.vm.restore(id,&roots)?;self.graphics=graphics;self.saved=saved;
            }
            "grestoreall" => {
                let floor=self.vm.graphics_floor();
                self.graphics=if floor>0 {self.saved.get(floor-1).cloned().unwrap_or_default()}else{Graphics::default()};
                self.saved.truncate(floor);
            }
            "stroke" | "fill" | "eofill" => self.paint(match op {
                "stroke" => "S",
                "eofill" => "f*",
                _ => "f",
            }),
            "clip" | "eoclip" => self
                .graphics
                .clips
                .push((self.graphics.path.clone(), op == "eoclip")),
            "initclip" => self.graphics.clips.clear(),
            "rectfill" | "rectstroke" | "rectclip" => {
                let a = self.nums(4)?;
                let old = self.graphics.path.clone();
                let point = self.graphics.point;
                let start = self.graphics.start;
                self.graphics.path.clear();
                self.graphics.point = None;
                self.move_to(a[0], a[1], false);
                self.move_to(a[0] + a[2], a[1], true);
                self.move_to(a[0] + a[2], a[1] + a[3], true);
                self.move_to(a[0], a[1] + a[3], true);
                self.graphics.path.push(Segment::Close);
                if op == "rectclip" {
                    self.graphics
                        .clips
                        .push((self.graphics.path.clone(), false))
                } else {
                    self.paint(if op == "rectfill" { "f" } else { "S" })
                }
                self.graphics.path = old;
                self.graphics.point = point;
                self.graphics.start = start;
            }
            "pathbbox" => {
                let inv = inverse(self.graphics.matrix)?;
                let mut points = Vec::new();
                for s in &self.graphics.path {
                    match s {
                        Segment::Move(x, y) | Segment::Line(x, y) => {
                            points.push(transform(inv, *x, *y))
                        }
                        Segment::Curve(a) => {
                            for p in a.chunks(2) {
                                points.push(transform(inv, p[0], p[1]))
                            }
                        }
                        _ => {}
                    }
                }
                if points.is_empty() {
                    return Err("nocurrentpoint".into());
                }
                for n in [
                    points.iter().map(|p| p.0).fold(f64::INFINITY, f64::min),
                    points.iter().map(|p| p.1).fold(f64::INFINITY, f64::min),
                    points.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max),
                    points.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max),
                ] {
                    self.pushnum(n)?;
                }
            }
            "pathforall" => {
                let close = self.pop()?;
                let curve = self.pop()?;
                let line = self.pop()?;
                let mv = self.pop()?;
                let inv = inverse(self.graphics.matrix)?;
                for s in self.graphics.path.clone() {
                    let (a, p) = match s {
                        Segment::Move(x, y) => (vec![x, y], mv.clone()),
                        Segment::Line(x, y) => (vec![x, y], line.clone()),
                        Segment::Curve(a) => (a.to_vec(), curve.clone()),
                        Segment::Close => (vec![], close.clone()),
                    };
                    for a in a.chunks(2) {
                        let p = transform(inv, a[0], a[1]);
                        self.pushnum(p.0)?;
                        self.pushnum(p.1)?
                    }
                    self.execute(p)?;
                }
            }
            "flattenpath" => {
                let mut path = Vec::new();
                let mut p = (0., 0.);
                for s in self.graphics.path.clone() {
                    match s {
                        Segment::Move(x, y) => {
                            p = (x, y);
                            path.push(Segment::Move(x, y));
                        }
                        Segment::Line(x, y) => {
                            p = (x, y);
                            path.push(Segment::Line(x, y));
                        }
                        Segment::Curve(a) => {
                            let start = p;
                            for i in 1..=32 {
                                let t = i as f64 / 32.;
                                let u = 1. - t;
                                p = (
                                    u * u * u * start.0
                                        + 3. * u * u * t * a[0]
                                        + 3. * u * t * t * a[2]
                                        + t * t * t * a[4],
                                    u * u * u * start.1
                                        + 3. * u * u * t * a[1]
                                        + 3. * u * t * t * a[3]
                                        + t * t * t * a[5],
                                );
                                path.push(Segment::Line(p.0, p.1));
                            }
                        }
                        Segment::Close => path.push(Segment::Close),
                    }
                }
                self.graphics.path = path;
            }
            "setgray" => {
                self.select_space(Value::Name("DeviceGray".into()))?;
                let a = self.num()?.clamp(0., 1.);self.graphics.color_values=vec![a];
                self.graphics.color = format!("{a:.6} g {a:.6} G")
            }
            "setrgbcolor" | "setcmykcolor" => {
                let n = if op == "setrgbcolor" { 3 } else { 4 };
                self.select_space(Value::Name(if n==3{"DeviceRGB"}else{"DeviceCMYK"}.into()))?;
                let values=self.nums(n)?.into_iter().map(|x|x.clamp(0.,1.)).collect::<Vec<_>>();self.graphics.color_values=values.clone();
                let a = values
                    .into_iter()
                    .map(|x| format!("{:.6}", x.clamp(0., 1.)))
                    .collect::<Vec<_>>()
                    .join(" ");
                self.graphics.color = format!(
                    "{a} {} {a} {}",
                    if n == 3 { "rg" } else { "k" },
                    if n == 3 { "RG" } else { "K" }
                );
            }
            "sethsbcolor" => {
                self.select_space(Value::Name("DeviceRGB".into()))?;
                let a = self.nums(3)?;
                let h = a[0].rem_euclid(1.) * 6.;
                let s = a[1].clamp(0., 1.);
                let v = a[2].clamp(0., 1.);
                let p = v * (1. - s);
                let q = v * (1. - s * (h - h.floor()));
                let t = v * (1. - s * (1. - (h - h.floor())));
                let (r, g, b) = match h as usize {
                    0 => (v, t, p),
                    1 => (q, v, p),
                    2 => (p, v, t),
                    3 => (p, q, v),
                    4 => (t, p, v),
                    _ => (v, p, q),
                };
                self.graphics.color_values=vec![r,g,b];self.graphics.color = format!("{r:.6} {g:.6} {b:.6} rg {r:.6} {g:.6} {b:.6} RG");
            }
            "currentgray" | "currentrgbcolor" | "currentcmykcolor" => {
                let rgb=self.current_color_rgb()?;
                if op == "currentgray" {
                    self.pushnum(0.3 * rgb[0] + 0.59 * rgb[1] + 0.11 * rgb[2])?
                } else if op == "currentrgbcolor" {
                    for n in rgb {
                        self.pushnum(n)?
                    }
                } else {
                    for n in [1. - rgb[0], 1. - rgb[1], 1. - rgb[2], 0.] {
                        self.pushnum(n)?
                    }
                }
            }
            "setlinewidth" => {
                let v = self.num()?;
                if v < 0. {
                    return Err("rangecheck: linewidth".into());
                }
                self.graphics.width = v
            }
            "currentlinewidth" => self.pushnum(self.graphics.width)?,
            "setlinecap" => self.graphics.cap = self.num()? as i32,
            "currentlinecap" => self.pushnum(self.graphics.cap as f64)?,
            "setlinejoin" => self.graphics.join = self.num()? as i32,
            "currentlinejoin" => self.pushnum(self.graphics.join as f64)?,
            "setmiterlimit" => self.graphics.miter = self.num()?,
            "currentmiterlimit" => self.pushnum(self.graphics.miter)?,
            "setdash" => {
                self.graphics.phase = self.num()?;
                let a = self.array()?;
                self.graphics.dash = a
                    .borrow()
                    .iter()
                    .map(Value::number)
                    .collect::<Result<_, _>>()?;
            }
            "currentdash" => {
                self.stack.push(Value::Array(Rc::new(RefCell::new(
                    self.graphics
                        .dash
                        .iter()
                        .copied()
                        .map(Value::Number)
                        .collect(),
                )).into()));
                self.pushnum(self.graphics.phase)?
            }
            "setflat" => {
                self.num()?;
            }
            "currentflat" => self.pushnum(1.)?,
            ".setopacityalpha" | ".setshapealpha" => {
                let a = self.num()?.clamp(0., 1.);
                self.graphics.fill_alpha = a;
                self.graphics.stroke_alpha = a
            }
            ".setfillconstantalpha" => self.graphics.fill_alpha = self.num()?.clamp(0., 1.),
            ".setstrokeconstantalpha" => self.graphics.stroke_alpha = self.num()?.clamp(0., 1.),
            ".setalphaisshape" => {
                self.pop()?.boolean()?;
            }
            "setglobal" => self.vm.global=self.pop()?.boolean()?,
            "setpacking" => self.vm.packing=self.pop()?.boolean()?,
            "currentglobal" => self.stack.push(Value::Bool(self.vm.global)),
            "currentpacking" => self.stack.push(Value::Bool(self.vm.packing)),
            "packedarray" => {let n=self.num()? as usize;if n>self.stack.len(){return Err("stackunderflow".into());}let mut array:Array=self.stack.split_off(self.stack.len()-n).into();array.packed=true;array.access=Access::ReadOnly;self.stack.push(Value::Array(array));},
            "revision" => self.pushnum(1000.)?,
            "languagelevel" => self.pushnum(3.)?,
            "version" | "product" => {
                self.stack
                    .push(Value::String(Rc::new(RefCell::new(if op == "version" {
                        b"1.0".to_vec()
                    } else {
                        b"Pitex embedded PostScript".to_vec()
                    })).into()))
            }
            "rand" => {
                self.random = self
                    .random
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1);
                self.pushnum(((self.random >> 32) & 0x7fffffff) as f64)?
            }
            "srand" => self.random = self.num()? as u64,
            "rrand" => self.pushnum(self.random as f64)?,
            "realtime" | "usertime" => self.pushnum(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u32 as f64,
            )?,
            "print" => {
                self.pop()?;
            }
            "flush" | "flushpage" | "showpage" | "copypage" => {}
            "erasepage" => self.output.clear(),
            "setpagedevice" => {
                self.pop()?;
            }
            "currentpagedevice" => self
                .stack
                .push(Value::Dict(Rc::new(RefCell::new(BTreeMap::new())))),
            "shfill" => self.shade()?,
            _ if images::recognizes(op) => self.image_operator(op)?,
            _ => return Err(format!("unsupported operator {op}")),
        }
        for value in self.stack.clone(){self.vm.track(&value);}
        for dict in self.dicts.clone(){self.vm.track(&Value::Dict(dict));}
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn procedures_are_literal_and_dynamic() {
        let mut i = Interpreter::new();
        i.budget = 10000;
        i.execute_bytes(b"/square {dup mul} def 5 square /x exch def x 25 eq")
            .unwrap();
        assert!(i.pop().unwrap().boolean().unwrap());
    }
    #[test]
    fn native_shapes_and_transparency() {
        let mut i = Interpreter::new();
        let p=i.special("pst: 1 0 0 setrgbcolor 0.5 .setopacityalpha 0 0 40 20 rectfill 20 10 5 0 360 arc stroke",10.,20.,|_|None).unwrap().unwrap();
        assert!(p.contains("10.000000 20.000000 m"));
        assert!(p.contains("PSTalpha0 gs"));
        assert_eq!(i.opacity_resources().len(), 1);
        assert!(p.contains(" c\n"));
    }
    #[test]
    fn dictionary_alias_and_loops() {
        let mut i = Interpreter::new();
        i.budget = 10000;
        i.execute_bytes(b"/d 4 dict def /a d def d /x 4 put a /x get 0 1 4 {add} for")
            .unwrap();
        assert_eq!(i.num().unwrap(), 14.);
    }
    #[test]
    fn unsupported_has_no_partial_output() {
        let mut i = Interpreter::new();
        let result = i.special("pst: 0 0 20 20 rectfill mystery", 0., 0., |_| None);
        assert!(result.unwrap_err().contains("mystery"));
        assert!(i.output.is_empty());
    }
    #[test]
    fn bounded_execution() {
        let mut i = Interpreter::new();
        assert!(i.special("pst: {} loop", 0., 0., |_| None).is_err());
    }
}
