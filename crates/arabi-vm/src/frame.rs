use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::error::RuntimeError;

pub struct FileHandle(pub std::rc::Rc<std::cell::RefCell<Option<std::fs::File>>>);

impl FileHandle {
    pub fn new(file: std::fs::File) -> Self {
        FileHandle(std::rc::Rc::new(std::cell::RefCell::new(Some(file))))
    }
}

impl Clone for FileHandle {
    fn clone(&self) -> Self {
        FileHandle(std::rc::Rc::clone(&self.0))
    }
}

impl std::fmt::Debug for FileHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<ملف>")
    }
}

#[derive(Clone, Debug)]
pub struct SharedList(pub Rc<RefCell<Vec<Value>>>);

impl SharedList {
    pub fn new(items: Vec<Value>) -> Self {
        SharedList(Rc::new(RefCell::new(items)))
    }
    pub fn borrow(&self) -> std::cell::Ref<'_, Vec<Value>> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, Vec<Value>> {
        self.0.borrow_mut()
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        unsafe { (*self.0.as_ptr()).len() }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub unsafe fn get_unchecked(&self, idx: usize) -> &Value {
        &*(*self.0.as_ptr()).as_ptr().add(idx)
    }

    #[inline(always)]
    pub unsafe fn set_unchecked(&self, idx: usize, val: Value) {
        let ptr = (*self.0.as_ptr()).as_mut_ptr().add(idx);
        let old = std::ptr::read(ptr);
        *ptr = val;
        drop(old);
    }

    #[inline(always)]
    pub unsafe fn swap_unchecked(&self, a: usize, b: usize) {
        let ptr = (*self.0.as_ptr()).as_mut_ptr();
        std::ptr::swap_nonoverlapping(ptr.add(a), ptr.add(b), 1);
    }

    #[inline(always)]
    pub fn push(&self, val: Value) {
        unsafe { (*self.0.as_ptr()).push(val); }
    }
}

#[derive(Clone, Debug)]
pub struct SharedDict {
    pub pairs: Rc<RefCell<Vec<(Value, Value)>>>,
    pub index: Rc<RefCell<HashMap<Value, usize>>>,
}

impl SharedDict {
    pub fn new(items: Vec<(Value, Value)>) -> Self {
        let mut index = HashMap::new();
        for (i, (k, _v)) in items.iter().enumerate() {
            index.insert(k.clone(), i);
        }
        SharedDict {
            pairs: Rc::new(RefCell::new(items)),
            index: Rc::new(RefCell::new(index)),
        }
    }
    pub fn borrow(&self) -> std::cell::Ref<'_, Vec<(Value, Value)>> {
        self.pairs.borrow()
    }
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, Vec<(Value, Value)>> {
        self.pairs.borrow_mut()
    }
    pub fn lookup(&self, key: &Value) -> Option<Value> {
        let index = self.index.borrow();
        if let Some(&idx) = index.get(key) {
            let pairs = self.pairs.borrow();
            pairs.get(idx).map(|(_, v)| v.clone())
        } else {
            None
        }
    }
    pub fn insert(&self, key: Value, val: Value) {
        let mut index = self.index.borrow_mut();
        let mut pairs = self.pairs.borrow_mut();
        if let Some(&idx) = index.get(&key) {
            pairs[idx].1 = val;
        } else {
            let idx = pairs.len();
            index.insert(key.clone(), idx);
            pairs.push((key, val));
        }
    }
    pub fn contains_key(&self, key: &Value) -> bool {
        self.index.borrow().contains_key(key)
    }
}

#[derive(Clone, Debug)]
pub struct SharedSet(pub Rc<RefCell<Vec<Value>>>);

impl SharedSet {
    pub fn new(items: Vec<Value>) -> Self {
        SharedSet(Rc::new(RefCell::new(items)))
    }
    pub fn borrow(&self) -> std::cell::Ref<'_, Vec<Value>> {
        self.0.borrow()
    }
    pub fn borrow_mut(&self) -> std::cell::RefMut<'_, Vec<Value>> {
        self.0.borrow_mut()
    }
}

#[derive(Debug, Clone)]
pub struct FunctionData {
    pub name: String,
    pub params: Vec<String>,
    pub param_indices: Vec<usize>,
    pub defaults: Vec<Option<Value>>,
    pub body: usize,
    pub closure: Vec<(usize, Value)>,
    pub varargs_param: Option<String>,
    pub kwargs_param: Option<String>,
    pub is_generator: bool,
    pub module_index: Option<usize>,
    pub num_locals: usize,
    pub normal_param_count: usize,
    pub call_count: Cell<u32>,
    pub jit_entry: Cell<Option<*const u8>>,
    pub jit_attempted: Cell<bool>,
}

#[derive(Debug, Clone)]
pub struct ClassData {
    pub name: Rc<str>,
    pub methods: SharedMethods,
    pub fields: HashMap<String, Value>,
    pub parents: Vec<String>,
    pub field_names: Vec<String>,
    pub field_index: Rc<HashMap<String, usize>>,
}

impl Default for ClassData {
    fn default() -> Self {
        Self {
            name: Rc::from(""),
            methods: Rc::new(HashMap::new()),
            fields: HashMap::new(),
            parents: Vec::new(),
            field_names: Vec::new(),
            field_index: Rc::new(HashMap::new()),
        }
    }
}

pub type SharedMethods = Rc<HashMap<String, Value>>;

const INLINE_FIELD_CAP: usize = 4;

#[derive(Debug)]
pub struct InstanceData {
    pub class: Rc<ClassData>,
    pub inline_fields: RefCell<[Value; INLINE_FIELD_CAP]>,
    pub num_inline_fields: u8,
    pub extra_fields: RefCell<Option<Box<HashMap<String, Value>>>>,
}

impl Default for InstanceData {
    fn default() -> Self {
        Self {
            class: Rc::new(ClassData::default()),
            inline_fields: RefCell::new(core::array::from_fn(|_| Value::Null)),
            num_inline_fields: 0,
            extra_fields: RefCell::new(None),
        }
    }
}

impl Clone for InstanceData {
    fn clone(&self) -> Self {
        Self {
            class: self.class.clone(),
            inline_fields: RefCell::new(self.inline_fields.borrow().clone()),
            num_inline_fields: self.num_inline_fields,
            extra_fields: RefCell::new(self.extra_fields.borrow().clone()),
        }
    }
}

impl InstanceData {
    #[inline(always)]
    pub fn get_field(&self, name: &str) -> Option<Value> {
        if self.num_inline_fields > 0 {
            if let Some(&i) = self.class.field_index.get(name) {
                if i < self.num_inline_fields as usize {
                    return Some(unsafe { (*self.inline_fields.as_ptr())[i].clone() });
                }
            }
        }
        self.extra_fields.borrow().as_ref().and_then(|f| f.get(name).cloned())
    }

    #[inline(always)]
    pub fn set_field(&self, name: String, value: Value) {
        if self.num_inline_fields > 0 {
            if let Some(&i) = self.class.field_index.get(&name) {
                if i < self.num_inline_fields as usize {
                    self.inline_fields.borrow_mut()[i] = value;
                    return;
                }
            }
        }
        self.extra_fields.borrow_mut().get_or_insert_with(|| Box::new(HashMap::new())).insert(name, value);
    }
}

#[derive(Debug, Clone)]
pub struct GeneratorData {
    pub name: String,
    pub body: usize,
    pub closure: Vec<(usize, Value)>,
    pub ip: usize,
    pub locals: Vec<Value>,
    pub last_sent: Option<Value>,
}

pub type SharedGenerator = Rc<RefCell<GeneratorData>>;

#[derive(Debug, Clone)]
pub struct ExceptionData {
    pub class_name: String,
    pub message: String,
    pub line: Option<usize>,
    pub call_stack: Vec<(String, Option<usize>)>,
}

#[derive(Debug, Clone)]
pub struct NativeFunctionData {
    pub name: String,
    pub arity: usize,
}

#[derive(Debug, Clone)]
pub struct FastMethodData {
    pub name: String,
    pub field1: String,
    pub field2: String,
    pub field1_idx: usize,
    pub field2_idx: usize,
    pub op: FastMethodOp,
}

#[derive(Debug, Clone, Copy)]
pub enum FastMethodOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub struct LazyModuleData {
    pub name: String,
    pub path: std::path::PathBuf,
    pub search_dirs: Vec<std::path::PathBuf>,
    pub loaded: Rc<RefCell<Option<Value>>>,
}

#[derive(Debug, Clone)]
pub struct SliceData {
    pub start: Box<Value>,
    pub end: Box<Value>,
    pub step: Box<Value>,
}

#[derive(Debug, Clone)]
pub struct RangeData {
    pub start: i64,
    pub end: i64,
    pub step: i64,
}

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(Rc<String>),
    Boolean(bool),
    Null,
    List(SharedList),
    Tuple(Rc<Vec<Value>>),
    Dict(SharedDict),
    Set(SharedSet),
    Cell(Rc<RefCell<Value>>),
    Function(Rc<FunctionData>),
    Class(Rc<ClassData>),
    Instance(Rc<InstanceData>),
    NativeFunction(Rc<NativeFunctionData>),
    FastMethod(Rc<FastMethodData>),
    Slice(Box<SliceData>),
    Range(Box<RangeData>),
    Exception(Box<ExceptionData>),
    File(FileHandle),
    Generator(SharedGenerator),
    LazyModule(Box<LazyModuleData>),
}

impl Value {
    #[inline(always)]
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Integer(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Boolean(b) => *b,
            Value::Null => false,
            Value::List(l) => !l.borrow().is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Dict(d) => !d.borrow().is_empty(),
            Value::Set(s) => !s.borrow().is_empty(),
            Value::Cell(c) => c.borrow().is_truthy(),
            Value::Function(_) => true,
            Value::Class(_) => true,
            Value::Instance(..) => true,
            Value::NativeFunction(_) => true,
            Value::Slice { .. } => true,
            Value::Range { .. } => true,
            Value::Exception(_) => true,
            Value::File(_) => true,
            Value::FastMethod(_) => true,
            Value::Generator(_) => true,
            Value::LazyModule(_) => true,
        }
    }

    #[inline(always)]
    pub fn type_name(&self) -> String {
        match self {
            Value::Integer(_) => "صحيح".to_string(),
            Value::Float(_) => "عشري".to_string(),
            Value::String(_) => "نص".to_string(),
            Value::Boolean(_) => "منطق".to_string(),
            Value::Null => "عدم".to_string(),
            Value::List(_) => "مصفوفة".to_string(),
            Value::Tuple(_) => "مترابطة".to_string(),
            Value::Dict(_) => "فهرس".to_string(),
            Value::Set(_) => "مميزة".to_string(),
            Value::Function(_) => "دالة".to_string(),
            Value::Class(_) => "صنف".to_string(),
            Value::Instance(rc) => rc.class.name.to_string(),
            Value::NativeFunction(_) => "دالة نظامية".to_string(),
            Value::Slice { .. } => "شريحة".to_string(),
            Value::Range { .. } => "نطاق".to_string(),
            Value::Exception(d) => d.class_name.clone(),
            Value::File(_) => "ملف".to_string(),
            Value::Generator(_) => "مولّد".to_string(),
            Value::Cell(_) => "خلية".to_string(),
            Value::LazyModule(_) => "وحدة".to_string(),
            Value::FastMethod(_) => "دالة سريعة".to_string(),
        }
    }

    pub fn to_string_value(&self) -> String {
        match self {
            Value::Integer(n) => n.to_string(),
            Value::Float(f) => format!("{}", f),
            Value::String(s) => s.to_string(),
            Value::Boolean(b) => if *b { "صح".to_string() } else { "خطا".to_string() },
            Value::Null => "عدم".to_string(),
            Value::List(items) => {
                let borrow = items.borrow();
                let strs: Vec<String> = borrow.iter().map(|v| v.to_string_value()).collect();
                format!("[{}]", strs.join(", "))
            }
            Value::Tuple(items) => {
                let strs: Vec<String> = items.as_ref().iter().map(|v| v.to_string_value()).collect();
                format!("({})", strs.join(", "))
            }
            Value::Dict(pairs) => {
                let borrow = pairs.borrow();
                let strs: Vec<String> = borrow.iter()
                    .map(|(k, v)| format!("{}: {}", k.to_string_value(), v.to_string_value()))
                    .collect();
                format!("{{{}}}", strs.join(", "))
            }
            Value::Set(items) => {
                let borrow = items.borrow();
                let strs: Vec<String> = borrow.iter().map(|v| v.to_string_value()).collect();
                format!("{{{}}}", strs.join(", "))
            }
            Value::Function(d) => format!("<دالة {}>", d.name),
            Value::Class(d) => format!("<صنف {}>", d.name),
            Value::Instance(rc) => format!("<نموذج {}>", rc.class.name),
            Value::NativeFunction(d) => format!("<دالة نظامية {}>", d.name),
            Value::Slice(d) => format!("{}:{}:{}", d.start.to_string_value(), d.end.to_string_value(), d.step.to_string_value()),
            Value::Range(d) => format!("نطاق({}, {}, {})", d.start, d.end, d.step),
            Value::Exception(d) => {
                let mut s = format!("{}: {}", d.class_name, d.message);
                if let Some(line) = d.line {
                    s.push_str(&format!(" (سطر {})", line));
                }
                s
            },
            Value::File(_) => "<ملف>".to_string(),
            Value::Generator(d) => format!("<مولّد {}>", d.borrow().name),
            Value::Cell(c) => c.borrow().to_string_value(),
            Value::LazyModule(d) => format!("<وحدة {}>", d.name),
            Value::FastMethod(d) => format!("<دالة سريعة {}>", d.name),
        }
    }

    #[inline(always)]
    pub fn get_attribute(&self, name: &str) -> Option<Value> {
        match self {
            Value::Instance(rc) => {
                if let Some(val) = rc.get_field(name) {
                    Some(val)
                } else {
                    rc.class.methods.get(name).cloned()
                }
            }
            Value::Class(d) => {
                if let Some(method) = d.methods.get(name) {
                    Some(method.clone())
                } else {
                    d.fields.get(name).cloned()
                }
            }
            Value::Range(d) => {
                if name == "طول" {
                    let len = if d.step > 0 {
                        if d.end > d.start { (d.end - d.start + d.step - 1) / d.step } else { 0 }
                    } else if d.step < 0 {
                        if d.start > d.end { (d.start - d.end - d.step - 1) / (-d.step) } else { 0 }
                    } else {
                        0
                    };
                    Some(Value::Integer(len))
                } else {
                    None
                }
            }
            Value::File(_) if name == "__ادخل__" => {
                Some(self.clone())
            }
            _ => None,
        }
    }

    #[inline(always)]
    pub fn get_method(&self, name: &str) -> Option<Value> {
        match self {
            Value::Instance(rc) => {
                rc.class.methods.get(name).cloned()
            }
            Value::Class(d) => {
                d.methods.get(name).cloned()
            }
            _ => None,
        }
    }

    pub fn set_attribute(&mut self, name: String, value: Value) -> bool {
        match self {
            Value::Instance(rc) => {
                rc.set_field(name, value);
                true
            }
            _ => false,
        }
    }

    pub fn call(&self, args: &[Value], kwargs: &[(String, Value)], vm: &mut super::vm::VM, module: &mut arabi_compiler::bytecode::BytecodeModule) -> Result<Value, RuntimeError> {
        match self {
            Value::Function(f) => {
                let params = &f.params;
                let param_indices = &f.param_indices;
                let defaults = &f.defaults;
                let body = f.body;
                let varargs_param = &f.varargs_param;
                let kwargs_param = &f.kwargs_param;
                let is_generator = f.is_generator;
                let name = &f.name;
                let closure = &f.closure;
                let module_index = &f.module_index;
                let use_module_num_locals = f.num_locals;
                if is_generator {
                    let mut local_vars: Vec<Value> = vec![Value::Null; use_module_num_locals.max(1)];
                    for (idx, val) in closure {
                        if *idx < local_vars.len() {
                            local_vars[*idx] = val.clone();
                        }
                    }
                    for (i, arg) in args.iter().enumerate() {
                        let local_idx = param_indices.get(i).copied().unwrap_or(i);
                        if local_idx < local_vars.len() {
                            local_vars[local_idx] = arg.clone();
                        }
                    }
                    return Ok(Value::Generator(Rc::new(RefCell::new(GeneratorData {
                        name: name.clone(),
                        body,
                        closure: closure.clone(),
                        ip: body,
                        locals: local_vars,
                        last_sent: None,
                    }))));
                }
                let arena_offset = vm.locals_arena.len();
                let n = use_module_num_locals.max(1);
                vm.locals_arena.resize(arena_offset + n, Value::Null);

                // Copy closure values directly into arena
                for (idx, val) in closure {
                    if *idx < n {
                        vm.locals_arena[arena_offset + *idx] = val.clone();
                    }
                }

                // Build a map of kwargs for quick lookup by param name (skip if empty)
                let kwargs_map: std::collections::HashMap<&str, &Value> = if kwargs.is_empty() {
                    std::collections::HashMap::new()
                } else {
                    kwargs.iter().map(|(k, v)| (k.as_str(), v)).collect()
                };

                // Find the varargs and kwargs param indices
                let varargs_idx = varargs_param.as_ref().and_then(|name| {
                    params.iter().position(|p| p == name)
                });
                let kwargs_idx = kwargs_param.as_ref().and_then(|name| {
                    params.iter().position(|p| p == name)
                });

                // Number of normal params (excluding varargs and kwargs)
                let normal_count = params.iter().enumerate().filter(|(i, _)| {
                    Some(*i) != varargs_idx && Some(*i) != kwargs_idx
                }).count();

                // How many positional args are for normal params
                let positional_for_normal = args.len().min(normal_count);

                // Assign positional args directly to arena
                let mut arg_pos = 0;
                for (i, param) in params.iter().enumerate() {
                    if Some(i) == varargs_idx || Some(i) == kwargs_idx {
                        continue;
                    }
                    let local_idx = param_indices.get(i).copied().unwrap_or(i);
                    if local_idx < n {
                        if arg_pos < positional_for_normal {
                            vm.locals_arena[arena_offset + local_idx] = args[arg_pos].clone();
                            arg_pos += 1;
                        } else if let Some(val) = kwargs_map.get(param.as_str()) {
                            vm.locals_arena[arena_offset + local_idx] = (*val).clone();
                        } else if let Some(default) = defaults.get(i).and_then(|d| d.as_ref()) {
                            vm.locals_arena[arena_offset + local_idx] = default.clone();
                        }
                    }
                }

                // Collect varargs (remaining positional args)
                if let Some(vi) = varargs_idx {
                    let local_idx = param_indices.get(vi).copied().unwrap_or(vi);
                    if local_idx < n {
                        let remaining: Vec<Value> = args[positional_for_normal..].to_vec();
                        vm.locals_arena[arena_offset + local_idx] = Value::List(SharedList::new(remaining));
                    }
                }

                // Collect kwargs (remaining keyword args)
                if let Some(_ki) = kwargs_idx {
                    let local_idx = param_indices.get(_ki).copied().unwrap_or(_ki);
                    if local_idx < n {
                        let used_params: std::collections::HashSet<&str> = params.iter()
                            .enumerate()
                            .filter(|(i, _)| Some(*i) != varargs_idx && Some(*i) != kwargs_idx)
                            .map(|(_, p)| p.as_str())
                            .collect();
                        let remaining_kwargs: Vec<(Value, Value)> = kwargs.iter()
                            .filter(|(k, _)| !used_params.contains(k.as_str()))
                            .map(|(k, v)| (Value::String(Rc::new(k.as_str().to_string())), v.clone()))
                            .collect();
                        vm.locals_arena[arena_offset + local_idx] = Value::Dict(SharedDict::new(remaining_kwargs));
                    }
                }

                vm.push_frame(arena_offset, n, body)?;
                let result = if let Some(idx) = module_index {
                    vm.run_imported_frame(*idx)?
                } else {
                    vm.run_frame(module)?
                };
                vm.pop_frame();
                Ok(result)
            }
            Value::Class(d) => {
                let name = &d.name;
                let methods = &d.methods;
                let _instance_methods = methods.clone();
                let init_method = methods.get("__تهيئة__").cloned();
                let nf = d.field_names.len().min(INLINE_FIELD_CAP) as u8;
                // Constructor inlining: simple __تهيئة__(self, x, y) with matching args
                if !d.field_names.is_empty() && d.field_names.len() == args.len() && args.len() <= INLINE_FIELD_CAP {
                    let mut inline: [Value; INLINE_FIELD_CAP] = core::array::from_fn(|_| Value::Null);
                    for (i, v) in args.iter().enumerate() {
                        inline[i] = v.clone();
                    }
                    return Ok(Value::Instance(Rc::new(InstanceData {
                        class: Rc::clone(&d),
                        inline_fields: RefCell::new(inline),
                        num_inline_fields: nf,
                        extra_fields: RefCell::new(None),
                    })));
                }
                let instance = Value::Instance(Rc::new(InstanceData {
                    class: Rc::clone(&d),
                    inline_fields: RefCell::new(core::array::from_fn(|_| Value::Null)),
                    num_inline_fields: nf,
                    extra_fields: RefCell::new(None),
                }));
                if let Some(Value::Function(f)) = init_method {
                    let param_indices = &f.param_indices;
                    let body = f.body;
                    let num_locals = f.num_locals;
                    let arena_offset = vm.locals_arena.len();
                    let n = num_locals.max(1);
                    vm.locals_arena.resize(arena_offset + n, Value::Null);
                    if let Some(&idx) = param_indices.first() {
                        if idx < n {
                            vm.locals_arena[arena_offset + idx] = instance.clone();
                        }
                    }
                    for (i, arg) in args.iter().enumerate() {
                        let local_idx = param_indices.get(i + 1).copied().unwrap_or(i + 1);
                        if local_idx < n {
                            vm.locals_arena[arena_offset + local_idx] = arg.clone();
                        }
                    }
                    vm.push_frame(arena_offset, n, body)?;
                    vm.current_class_name = Some(name.clone());
                    vm.current_instance = Some(instance.clone());
                    vm.run_frame(module)?;
                    vm.pop_frame();
                    vm.current_class_name = None;
                    vm.current_instance = None;
                }
                Ok(instance)
            }
            Value::NativeFunction(d) => {
                super::builtins::call_native(&d.name, args, kwargs, vm, module)
            }
            Value::Instance(rc) => {
                if let Some(callable) = rc.class.methods.get("__استدعاء__").cloned() {
                    let mut all_args = vec![self.clone()];
                    all_args.extend_from_slice(args);
                    callable.call(&all_args, kwargs, vm, module)
                } else {
                    Err(format!("غير قادر على استدعاء {}", self.type_name()).into())
                }
            }
            Value::Generator(_) => {
                Err("المولّد لا يمكن استدعاؤه مباشرة. استخدم ابعث()".into())
            }
            _ => Err(format!("غير قادر على استدعاء {}", self.type_name()).into()),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_value())
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Integer(a), Value::Float(b)) => *a as f64 == *b,
            (Value::Float(a), Value::Integer(b)) => *a == *b as f64,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::List(a), Value::List(b)) => *a.borrow() == *b.borrow(),
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Integer(a), Value::Integer(b)) => a.partial_cmp(b),
            (Value::Float(a), Value::Float(b)) => a.partial_cmp(b),
            (Value::Integer(a), Value::Float(b)) => (*a as f64).partial_cmp(b),
            (Value::Float(a), Value::Integer(b)) => a.partial_cmp(&(*b as f64)),
            (Value::String(a), Value::String(b)) => a.partial_cmp(b),
            _ => None,
        }
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::Integer(n) => {
                0u8.hash(state);
                n.hash(state);
            }
            Value::Float(n) => {
                1u8.hash(state);
                n.to_bits().hash(state);
            }
            Value::String(s) => {
                2u8.hash(state);
                s.hash(state);
            }
            Value::Boolean(b) => {
                3u8.hash(state);
                b.hash(state);
            }
            Value::Null => {
                4u8.hash(state);
            }
            Value::Tuple(items) => {
                5u8.hash(state);
                items.len().hash(state);
                for item in items.as_ref() {
                    item.hash(state);
                }
            }
            Value::Range(d) => {
                6u8.hash(state);
                d.start.hash(state);
                d.end.hash(state);
                d.step.hash(state);
            }
            _ => {
                std::ptr::hash(self as *const Self, state);
            }
        }
    }
}

const _: () = assert!(std::mem::align_of::<Value>() == 8, "Value must be 8-byte aligned");
