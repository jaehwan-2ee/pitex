// Original Pitex PostScript/PDF shading bridge, from the Adobe PLRM and ISO 32000.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
#[derive(Clone)]
pub struct Object {
    pub dictionary: String,
    pub data: Option<Vec<u8>>,
}
#[derive(Clone)]
pub struct Shading {
    pub name: String,
    pub objects: Vec<Object>,
    pub root: usize,
}
fn pdf(value: &Value, vm: &vm::Vm) -> Result<String, String> {
    vm.read(value)?;
    Ok(match value {
        Value::Number(n) if n.is_finite() => { let mut text=format!("{n:.10}");while text.ends_with('0'){text.pop();}if text.ends_with('.'){text.pop();}if text=="-0"{text="0".into();}text },
        Value::Bool(b) => b.to_string(),
        Value::Name(name) | Value::Word(name) => {
            let mut out = String::from("/");
            for byte in name.bytes() {
                if byte <= 32 || byte >= 127 || b"#/%()<>[]{}@".contains(&byte) {
                    out += &format!("#{byte:02X}");
                } else {
                    out.push(byte as char);
                }
            }
            out
        }
        Value::String(data) => format!(
            "<{}>",
            data.borrow()
                .iter()
                .map(|v| format!("{v:02X}"))
                .collect::<String>()
        ),
        Value::Array(array) => format!(
            "[{}]",
            array
                .borrow()
                .iter()
                .map(|v| pdf(v, vm))
                .collect::<Result<Vec<_>, _>>()?
                .join(" ")
        ),
        Value::Dict(dict) => format!(
            "<<{}>>",
            dict.borrow()
                .iter()
                .map(|(k, v)| Ok(format!(
                    "{} {}",
                    pdf(&Value::Name(k.clone()), vm)?,
                    pdf(v, vm)?
                )))
                .collect::<Result<Vec<_>, String>>()?
                .join(" ")
        ),
        Value::Null => "null".into(),
        _ => return Err("typecheck: PDF shading value".into()),
    })
}
fn source(value: &Value, vm: &vm::Vm) -> Result<Vec<u8>, String> {
    vm.read(value)?;
    match value {
        Value::String(data) => Ok(data.borrow().to_vec()),
        Value::File(file) => {
            let mut file = file.borrow_mut();
            let data = file.0[file.1..].to_vec();
            file.1 = file.0.len();
            Ok(data)
        }
        _ => Err("typecheck: shading DataSource".into()),
    }
}
fn add(objects: &mut Vec<Object>, dictionary: String, data: Option<Vec<u8>>) -> String {
    let id = objects.len();
    objects.push(Object { dictionary, data });
    format!("@@{id}@@")
}
fn entries(dict: &BTreeMap<String, Value>, keys: &[&str], vm: &vm::Vm) -> Result<String, String> {
    let mut out = String::new();
    for key in keys {
        if let Some(value) = dict.get(*key) {
            out += &format!("/{key} {} ", pdf(value, vm)?);
        }
    }
    Ok(out)
}
pub(super) fn function(value: &Value, objects: &mut Vec<Object>, vm: &vm::Vm) -> Result<String, String> {
    vm.read(value)?;
    if let Value::Array(array) = value {
        return Ok(format!(
            "[{}]",
            array
                .borrow()
                .iter()
                .map(|v| function(v, objects, vm))
                .collect::<Result<Vec<_>, _>>()?
                .join(" ")
        ));
    }
    let Value::Dict(dict) = value else {
        return Err("typecheck: shading Function".into());
    };
    let dict = dict.borrow();
    let kind = dict
        .get("FunctionType")
        .ok_or("undefined: FunctionType")?
        .number()? as i32;
    if !(0..=4).contains(&kind) || kind == 1 {
        return Err("rangecheck: FunctionType".into());
    }
    let mut body = entries(
        &dict,
        &[
            "FunctionType",
            "Domain",
            "Range",
            "Size",
            "BitsPerSample",
            "Order",
            "Encode",
            "Decode",
            "C0",
            "C1",
            "N",
            "Bounds",
        ],
        vm,
    )?;
    if let Some(children) = dict.get("Functions") {
        body += &format!("/Functions {} ", function(children, objects, vm)?);
    }
    let data = if kind == 0 || kind == 4 {
        Some(source(
            dict.get("DataSource")
                .ok_or("undefined: function DataSource")?,
            vm,
        )?)
    } else {
        None
    };
    Ok(add(objects, body, data))
}
fn colorspace(value:&Value,objects:&mut Vec<Object>,interpreter:&mut Interpreter)->Result<String,String>{
    let space=interpreter.color_space(value)?;interpreter.color_pdf(&space,objects)
}
impl Interpreter {
    pub fn shading_resources(&self) -> Vec<Shading> {
        self.shadings.clone()
    }
    pub(super) fn shade(&mut self) -> Result<(), String> {
        let Value::Dict(dict) = self.pop()? else {
            return Err("typecheck: shfill".into());
        };
        self.vm.read(&Value::Dict(dict.clone()))?;
        let dict = dict.borrow();
        let kind = dict
            .get("ShadingType")
            .ok_or("undefined: ShadingType")?
            .number()? as i32;
        if !(1..=7).contains(&kind) {
            return Err("rangecheck: ShadingType".into());
        }
        let mut objects = Vec::new();
        let mut body = entries(
            &dict,
            &[
                "ShadingType",
                "BBox",
                "Background",
                "AntiAlias",
                "Coords",
                "Domain",
                "Matrix",
                "Extend",
                "BitsPerCoordinate",
                "BitsPerComponent",
                "BitsPerFlag",
                "Decode",
                "VerticesPerRow",
            ],
            &self.vm,
        )?;
        body += &format!(
            "/ColorSpace {} ",
            colorspace(
                dict.get("ColorSpace")
                    .ok_or("undefined: shading ColorSpace")?,
                &mut objects,
                self
            )?
        );
        if let Some(value) = dict.get("Function") {
            let space=self.color_space(dict.get("ColorSpace").unwrap())?;
            let normalized=self.normalized_color_function(value,&space,&mut objects)?;
            let reference=if let Some(reference)=normalized{reference}else{function(value,&mut objects,&self.vm)?};
            body += &format!("/Function {} ",reference);
        } else if kind <= 3 {
            return Err("undefined: shading Function".into());
        }
        let data = if kind >= 4 {
            Some(source(
                dict.get("DataSource").ok_or("undefined: mesh DataSource")?,
                &self.vm,
            )?)
        } else {
            None
        };
        let root = objects.len();
        objects.push(Object {
            dictionary: body,
            data,
        });
        let name = format!("PSTshade{}", self.shadings.len());
        self.shadings.push(Shading {
            name: name.clone(),
            objects,
            root,
        });
        self.output += "q\n";
        if self.graphics.fill_alpha != 1. || self.graphics.stroke_alpha != 1. {
            let key = self
                .alphas
                .iter()
                .find(|(_, f, s)| {
                    *f == self.graphics.fill_alpha && *s == self.graphics.stroke_alpha
                })
                .map(|(n, _, _)| n.clone())
                .unwrap_or_else(|| {
                    let key = format!("PSTalpha{}", self.alphas.len());
                    self.alphas.push((
                        key.clone(),
                        self.graphics.fill_alpha,
                        self.graphics.stroke_alpha,
                    ));
                    key
                });
            self.output += &format!("/{key} gs\n");
        }
        for (path, even) in &self.graphics.clips {
            self.output += &format_path(path);
            self.output += if *even { "W* n\n" } else { "W n\n" };
        }
        self.output += &format!(
            "{} cm\n/{name} sh\nQ\n",
            self.graphics
                .matrix
                .iter()
                .map(|v| format!("{v:.10}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn axial_real_function_objects() {
        let mut i = Interpreter::new();
        let output=i.special("pst: << /ShadingType 2 /ColorSpace /DeviceRGB /Coords [0 0 40 0] /Function << /FunctionType 2 /Domain [0 1] /C0 [1 0 0] /C1 [0 0 1] /N 1 >> /Extend [true true] >> shfill",0.,0.,|_|None).unwrap().unwrap();
        assert!(output.contains("/PSTshade0 sh"));
        let resources = i.shading_resources();
        assert_eq!(resources[0].objects.len(), 2);
        assert!(resources[0].objects[1]
            .dictionary
            .contains("/Function @@0@@"));
    }
}
