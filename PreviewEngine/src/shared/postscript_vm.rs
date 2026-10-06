// Pitex-authored PostScript VM, following Adobe PLRM §§3.3, 3.7.
// Copyright (c) 2026 Pitex contributors. SPDX-License-Identifier: AGPL-3.0-or-later
use super::{Array, Dict, Graphics, Value};
use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{BTreeMap, BTreeSet},
    rc::{Rc, Weak},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Access {
    Unlimited,
    ReadOnly,
    ExecuteOnly,
    None,
}
impl Access {
    pub fn readable(self) -> bool {
        matches!(self, Self::Unlimited | Self::ReadOnly)
    }
    pub fn writable(self) -> bool {
        self == Self::Unlimited
    }
    pub fn executable(self) -> bool {
        self != Self::None
    }
    fn restrict(self, next: Self) -> Result<Self, String> {
        if (next.readable() && !self.readable())
            || (next.writable() && !self.writable())
            || (next.executable() && !self.executable())
        {
            return Err("invalidaccess: cannot increase access".into());
        }
        Ok(next)
    }
}
#[derive(Clone, Debug)]
pub(super) struct Sequence<T> {
    pub data: Rc<RefCell<Vec<T>>>,
    start: usize,
    length: usize,
    pub access: Access,
    pub packed: bool,
}
impl<T> From<Rc<RefCell<Vec<T>>>> for Sequence<T> {
    fn from(data: Rc<RefCell<Vec<T>>>) -> Self {
        let length = data.borrow().len();
        Self {
            data,
            start: 0,
            length,
            access: Access::Unlimited,
            packed: false,
        }
    }
}
impl<T> From<Vec<T>> for Sequence<T> {
    fn from(data: Vec<T>) -> Self {
        Rc::new(RefCell::new(data)).into()
    }
}
impl<T> Sequence<T> {
    pub fn borrow(&self) -> Ref<'_, [T]> {
        Ref::map(self.data.borrow(), |data| {
            &data[self.start..self.start + self.length]
        })
    }
    pub fn borrow_mut(&self) -> RefMut<'_, [T]> {
        RefMut::map(self.data.borrow_mut(), |data| {
            &mut data[self.start..self.start + self.length]
        })
    }
    pub fn interval(&self, start: usize, length: usize) -> Result<Self, String>
    where
        T: Clone,
    {
        if start
            .checked_add(length)
            .is_none_or(|end| end > self.length)
        {
            return Err("rangecheck: getinterval".into());
        }
        let mut result = self.clone();
        result.start += start;
        result.length = length;
        Ok(result)
    }
    pub fn same(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.data, &other.data)
            && self.start == other.start
            && self.length == other.length
    }
    pub fn read(&self) -> Result<(), String> {
        if self.access.readable() {
            Ok(())
        } else {
            Err("invalidaccess: unreadable sequence".into())
        }
    }
    pub fn write(&self) -> Result<(), String> {
        if self.access.writable() && !self.packed {
            Ok(())
        } else {
            Err("invalidaccess: unwritable sequence".into())
        }
    }
    pub fn restrict(&mut self, access: Access) -> Result<(), String> {
        self.access = self.access.restrict(access)?;
        Ok(())
    }
}
// A file value copies its descriptor attributes while retaining the shared
// stream and cursor. Access reduction on one alias does not restrict another.
#[derive(Clone, Debug)]
pub(super) struct File {
    pub data: Rc<RefCell<(Vec<u8>, usize)>>,
    pub access: Access,
}
impl From<Rc<RefCell<(Vec<u8>, usize)>>> for File {
    fn from(data: Rc<RefCell<(Vec<u8>, usize)>>) -> Self {
        Self {
            data,
            access: Access::ReadOnly,
        }
    }
}
impl std::ops::Deref for File {
    type Target = Rc<RefCell<(Vec<u8>, usize)>>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl File {
    fn restrict(&mut self, access: Access) -> Result<(), String> {
        self.access = self.access.restrict(access)?;
        Ok(())
    }
}

type Key = (u8, usize);
#[derive(Clone)]
enum Storage {
    Array(Weak<RefCell<Vec<Value>>>),
    Dict(Weak<RefCell<BTreeMap<String, Value>>>),
    String(Weak<RefCell<Vec<u8>>>),
    File(Weak<RefCell<(Vec<u8>, usize)>>),
}
#[derive(Clone)]
struct Object {
    storage: Storage,
    birth: u64,
    global: bool,
    access: Access,
}
#[derive(Clone)]
enum Contents {
    Array(Rc<RefCell<Vec<Value>>>, Vec<Value>),
    Dict(Dict, BTreeMap<String, Value>),
}
struct Save {
    id: u64,
    epoch: u64,
    contents: Vec<Contents>,
    access: Vec<(Key, Access)>,
    graphics: Graphics,
    saved: Vec<Graphics>,
    global: bool,
    packing: bool,
}
#[derive(Default)]
pub(super) struct Vm {
    objects: BTreeMap<Key, Object>,
    epoch: u64,
    next_save: u64,
    saves: Vec<Save>,
    pub global: bool,
    pub packing: bool,
}
impl Vm {
    fn key(value: &Value) -> Option<Key> {
        Some(match value {
            Value::Array(a) | Value::Proc(a) => (0, Rc::as_ptr(&a.data) as usize),
            Value::Dict(d) => (1, Rc::as_ptr(d) as usize),
            Value::String(s) => (2, Rc::as_ptr(&s.data) as usize),
            Value::File(f) => (3, Rc::as_ptr(&f.data) as usize),
            _ => return None,
        })
    }
    pub fn contains(&self, value: &Value) -> bool {
        Self::key(value).is_some_and(|key| self.objects.contains_key(&key))
    }
    pub fn track(&mut self, value: &Value) {
        self.track_inner(value, &mut BTreeSet::new());
    }
    fn track_inner(&mut self, value: &Value, visited: &mut BTreeSet<Key>) {
        let Some(key) = Self::key(value) else {
            return;
        };
        if !visited.insert(key) {
            return;
        }
        if self.objects.contains_key(&key) {
            return;
        }
        if !self.objects.contains_key(&key) {
            self.epoch += 1;
            let storage = match value {
                Value::Array(a) | Value::Proc(a) => Storage::Array(Rc::downgrade(&a.data)),
                Value::Dict(d) => Storage::Dict(Rc::downgrade(d)),
                Value::String(s) => Storage::String(Rc::downgrade(&s.data)),
                Value::File(f) => Storage::File(Rc::downgrade(&f.data)),
                _ => unreachable!(),
            };
            self.objects.insert(
                key,
                Object {
                    storage,
                    birth: self.epoch,
                    global: self.global,
                    access: if matches!(value, Value::File(_)) {
                        Access::ReadOnly
                    } else {
                        Access::Unlimited
                    },
                },
            );
        }
        match value {
            Value::Array(a) | Value::Proc(a) => {
                let values = a.borrow().to_vec();
                for child in values {
                    self.track_inner(&child, visited);
                }
            }
            Value::Dict(d) => {
                let values = d.borrow().values().cloned().collect::<Vec<_>>();
                for child in values {
                    self.track_inner(&child, visited);
                }
            }
            _ => {}
        }
    }
    pub fn scan(&mut self, value: &Value) {
        self.scan_inner(value, &mut BTreeSet::new());
    }
    fn scan_inner(&mut self, value: &Value, visited: &mut BTreeSet<Key>) {
        let Some(key) = Self::key(value) else {
            return;
        };
        if !visited.insert(key) {
            return;
        }
        self.track(value);
        match value {
            Value::Array(a) | Value::Proc(a) => {
                let children = a.borrow().to_vec();
                for child in children {
                    self.scan_inner(&child, visited);
                }
            }
            Value::Dict(d) => {
                let children = d.borrow().values().cloned().collect::<Vec<_>>();
                for child in children {
                    self.scan_inner(&child, visited);
                }
            }
            _ => {}
        }
    }
    pub fn set_space(&mut self, value: &Value, global: bool) {
        self.track(value);
        if let Some(object) = Self::key(value).and_then(|key| self.objects.get_mut(&key)) {
            object.global = global;
        }
    }
    pub fn global(&self, value: &Value) -> bool {
        Self::key(value)
            .map(|key| self.objects.get(&key).is_some_and(|object| object.global))
            .unwrap_or(!matches!(value, Value::Save(_)))
    }
    fn access(&self, value: &Value) -> Access {
        match value {
            Value::Array(a) | Value::Proc(a) => a.access,
            Value::String(s) => s.access,
            Value::File(f) => f.access,
            _ => Self::key(value)
                .and_then(|key| self.objects.get(&key))
                .map(|object| object.access)
                .unwrap_or(Access::Unlimited),
        }
    }
    pub fn read(&self, value: &Value) -> Result<(), String> {
        if self.access(value).readable() {
            Ok(())
        } else {
            Err("invalidaccess: unreadable object".into())
        }
    }
    pub fn write(&self, value: &Value) -> Result<(), String> {
        if !self.access(value).writable()
            || matches!(value,Value::Array(a)|Value::Proc(a) if a.packed)
        {
            Err("invalidaccess: unwritable object".into())
        } else {
            Ok(())
        }
    }
    pub fn execute(&self, value: &Value) -> Result<(), String> {
        if self.access(value).executable() {
            Ok(())
        } else {
            Err("invalidaccess: executable object".into())
        }
    }
    pub fn rcheck(&self, value: &Value) -> bool {
        self.access(value).readable()
    }
    pub fn wcheck(&self, value: &Value) -> bool {
        self.write(value).is_ok()
    }
    pub fn restrict(&mut self, value: &mut Value, access: Access) -> Result<(), String> {
        self.track(value);
        match value {
            Value::Array(a) | Value::Proc(a) => a.restrict(access),
            Value::String(s) => s.restrict(access),
            Value::File(f) => f.restrict(access),
            Value::Dict(_) => {
                if access == Access::ExecuteOnly && matches!(value, Value::Dict(_)) {
                    return Err("typecheck: executeonly dictionary".into());
                }
                let object = self.objects.get_mut(&Self::key(value).unwrap()).unwrap();
                object.access = object.access.restrict(access)?;
                Ok(())
            }
            _ => Err("typecheck: access attribute".into()),
        }
    }
    pub fn store(&mut self, dest: &Value, value: &Value) -> Result<(), String> {
        self.track(dest);
        self.track(value);
        self.write(dest)?;
        if self.global(dest) && !self.global(value) {
            return Err("invalidaccess: local object in global VM".into());
        }
        Ok(())
    }
    pub fn graphics_floor(&self) -> usize {
        self.saves
            .last()
            .map(|save| save.saved.len() + 1)
            .unwrap_or(0)
    }
    pub fn save(&mut self, graphics: &Graphics, saved: &[Graphics]) -> u64 {
        self.next_save += 1;
        let mut contents = Vec::new();
        let mut access = Vec::new();
        for (key, object) in &self.objects {
            if object.global {
                continue;
            }
            access.push((*key, object.access));
            match &object.storage {
                Storage::Array(weak) => {
                    if let Some(array) = weak.upgrade() {
                        let values = array.borrow().clone();
                        contents.push(Contents::Array(array, values));
                    }
                }
                Storage::Dict(weak) => {
                    if let Some(dict) = weak.upgrade() {
                        let values = dict.borrow().clone();
                        contents.push(Contents::Dict(dict, values));
                    }
                }
                _ => {}
            }
        }
        self.saves.push(Save {
            id: self.next_save,
            epoch: self.epoch,
            contents,
            access,
            graphics: graphics.clone(),
            saved: saved.to_vec(),
            global: self.global,
            packing: self.packing,
        });
        self.next_save
    }
    pub fn restore(
        &mut self,
        id: u64,
        roots: &[Value],
    ) -> Result<(Graphics, Vec<Graphics>), String> {
        let position = self
            .saves
            .iter()
            .position(|save| save.id == id)
            .ok_or("invalidrestore: unknown save")?;
        let epoch = self.saves[position].epoch;
        for value in roots {
            if matches!(value,Value::Save(other) if *other>id) {
                return Err("invalidrestore: newer save object remains on a stack".into());
            }
            if let Some(object) = Self::key(value).and_then(|key| self.objects.get(&key)) {
                if !object.global && object.birth > epoch {
                    return Err("invalidrestore: newer local object remains on a stack".into());
                }
            }
        }
        let save = self.saves.remove(position);
        self.saves.truncate(position);
        for contents in save.contents {
            match contents {
                Contents::Array(data, values) => *data.borrow_mut() = values,
                Contents::Dict(data, values) => *data.borrow_mut() = values,
            }
        }
        for (key, access) in save.access {
            if let Some(object) = self.objects.get_mut(&key) {
                object.access = access;
            }
        }
        self.objects
            .retain(|_, object| object.global || object.birth <= epoch);
        self.global = save.global;
        self.packing = save.packing;
        Ok((save.graphics, save.saved))
    }
}

#[cfg(test)]
mod tests {
    use super::super::Interpreter;
    use super::*;
    fn oracle(source: &str) -> Vec<bool> {
        let program = format!("{} {{ == }} forall", source);
        let output = std::process::Command::new("gs")
            .args(["-q", "-dNODISPLAY", "-dBATCH", "-c", &program])
            .output()
            .expect("Ghostscript regression oracle");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| match line {
                "true" => true,
                "false" => false,
                other => panic!("unexpected oracle {other}"),
            })
            .collect()
    }
    fn compare(source: &str) {
        let expected = oracle(source);
        assert!(!expected.is_empty());
        assert!(expected.iter().all(|v| *v), "oracle fixture is not valid");
        let mut interpreter = Interpreter::new();
        interpreter.budget = 100000;
        interpreter.execute_bytes(source.as_bytes()).unwrap();
        let Value::Array(values) = interpreter.pop().unwrap() else {
            panic!("missing boolean array")
        };
        let observed = values
            .borrow()
            .iter()
            .map(|v| v.boolean().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(observed, expected);
        assert!(interpreter.stack.is_empty());
    }
    #[test]
    fn oracle_local_vm_aliases_and_string_exception() {
        compare(
            r"/a [10 20] def /alias a def /slice a 1 1 getinterval def /d << /n 7 >> def /str (abc) def /ss str 0 1 getinterval def /s save def a 0 99 put slice 0 88 put d /n 8 put ss 0 90 put /temporary 123 def s restore [a 0 get 10 eq alias 0 get 10 eq slice 0 get 20 eq d /n get 7 eq str 0 get 90 eq /temporary where {pop false} {true} ifelse]",
        );
    }
    #[test]
    fn oracle_nested_saves_and_opaque_tokens() {
        compare(
            r"/a [1 2] def /outer save def a 0 3 put /inner save def a 1 4 put inner restore /middle a 0 get 3 eq a 1 get 2 eq and def outer restore [a 0 get 1 eq a 1 get 2 eq save dup type /savetype eq exch restore]",
        );
    }
    #[test]
    fn oracle_global_vm_survives_restore() {
        compare(
            r"true setglobal /g [1] def false setglobal /a [2] def /s save def g 0 9 put a 0 8 put true setglobal s restore [g 0 get 9 eq a 0 get 2 eq g gcheck a gcheck not globaldict gcheck userdict gcheck not currentglobal not]",
        );
    }
    #[test]
    fn oracle_descriptor_and_dictionary_access() {
        compare(
            r"/a [1] def /ro a readonly def /d 2 dict def /alias d def d readonly pop /none (abc) noaccess def /p {1} executeonly def [a wcheck ro wcheck not ro rcheck alias wcheck not d rcheck none rcheck not none wcheck not /p load rcheck not p 1 eq true setglobal (41>) /ASCIIHexDecode filter dup gcheck exch dup wcheck not exch rcheck false setglobal]",
        );
    }
    #[test]
    fn oracle_file_descriptors_share_cursor_and_preserve_alias_access() {
        compare(
            r"/f (4142>) /ASCIIHexDecode filter def /original f def /blocked f noaccess def [original rcheck blocked rcheck not original wcheck not blocked wcheck not original blocked eq original read {65 eq}{false} ifelse f read {66 eq}{false} ifelse original read false eq]",
        );
        compare(
            r"true setglobal /f (41>) /ASCIIHexDecode filter def false setglobal /blocked f noaccess def [f gcheck blocked gcheck f rcheck blocked rcheck not f readonly rcheck]",
        );
    }
    #[test]
    fn oracle_packing_and_procedure_identity() {
        compare(
            r"true setpacking /packed {1} def false setpacking /p {1 2} def /q /p load def /s save def /p load 0 8 put s restore [/q load 0 get 1 eq /p load cvlit /q load eq /packed load type /packedarraytype eq /packed load wcheck not currentpacking not 1 2 2 packedarray type /packedarraytype eq]",
        );
    }
    #[test]
    fn bounded_invalidaccess_and_invalidrestore_errors() {
        for source in [
            "/s save def [1] s restore",
            "/s save def save s restore pop",
            "/a [1] readonly def a 0 2 put",
            "(abc) noaccess (abc) eq",
            "true setglobal /g 1 array def false setglobal g 0 (local) put",
        ] {
            let program = format!("{{ {} }} stopped ==", source);
            let result = std::process::Command::new("gs")
                .args(["-q", "-dNODISPLAY", "-dBATCH", "-c", &program])
                .output()
                .unwrap();
            assert!(result.status.success());
            assert!(String::from_utf8_lossy(&result.stdout).trim() == "true");
            let mut interpreter = Interpreter::new();
            interpreter.budget = 100000;
            let error = interpreter.execute_bytes(source.as_bytes()).unwrap_err();
            assert!(
                error.starts_with("invalidaccess") || error.starts_with("invalidrestore"),
                "{error}"
            );
        }
    }
    #[test]
    fn save_restores_graphics_but_gsave_preserves_vm() {
        compare(
            r"/a [1] def 0 setgray /s save def .5 setgray a 0 2 put s restore gsave a 0 3 put .8 setgray grestore [currentgray 0 eq a 0 get 3 eq /s save def .5 setgray grestore currentgray 0 eq s restore]",
        );
    }
}
