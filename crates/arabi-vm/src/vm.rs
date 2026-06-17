use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use crate::frame::{Value, SharedList, SharedDict, SharedSet, FunctionData, ClassData, InstanceData, ExceptionData, NativeFunctionData, SliceData, FastMethodData, FastMethodOp};
use crate::error::RuntimeError;
use arabi_compiler::bytecode::*;
use arabi_jit::CraneliftJIT;

pub struct VM {
    frames: Vec<Frame>,
    frame_depth_limit: usize,
    stack: Vec<Value>,
    pub globals: HashMap<String, Value>,
    global_values: Vec<Value>,
    global_name_index: HashMap<String, usize>,
    module_cache: HashMap<String, Value>,
    exception_handlers: Vec<usize>,
    pub(crate) current_exception: Option<Value>,
    pub(crate) current_class_name: Option<Rc<str>>,
    pub(crate) current_instance: Option<Value>,
    pub(crate) exception_hierarchy: HashMap<String, Vec<String>>,
    search_dirs: Vec<PathBuf>,
    pub(crate) imported_modules: Vec<std::cell::RefCell<arabi_compiler::bytecode::BytecodeModule>>,
    pub(crate) locals_arena: Vec<Value>,
    pub(crate) jit_compiler: CraneliftJIT,
    pub(crate) modules: Vec<arabi_compiler::bytecode::BytecodeModule>,
    mc_methods_ptr: *const std::collections::HashMap<String, Value>,
    mc_method: u64,
    mc_value: Value,
    pub(crate) memory_used: usize,
    memory_limit: usize,
}

use std::sync::LazyLock;

// Macro to create a builtin module (Class with static methods)
#[allow(unused)]
macro_rules! make_builtin_module {
    ($name:expr, $($method_name:expr => $fn_name:expr),* $(,)?) => {{
        let mut methods = HashMap::new();
        $(methods.insert($method_name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData {
            name: $fn_name.to_string(), arity: 0,
        })));)*
        Value::Class(Rc::new(ClassData {
            name: Rc::from($name),
            methods: Rc::new(methods),
            fields: HashMap::new(),
            parents: Vec::new(),
            field_names: Vec::new(),
            field_index: Rc::new(HashMap::new()),
        }))
    }};
}

// Same as above but with fields (for modules that have constants like math.PI)
#[allow(unused)]
macro_rules! make_builtin_module_with_fields {
    ($name:expr, {$($field_name:expr => $field_val:expr),* $(,)?}, $($method_name:expr => $fn_name:expr),* $(,)?) => {{
        let mut methods = HashMap::new();
        $(methods.insert($method_name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData {
            name: $fn_name.to_string(), arity: 0,
        })));)*
        let mut fields = HashMap::new();
        $(fields.insert($field_name.to_string(), $field_val);)*
        Value::Class(Rc::new(ClassData {
            name: Rc::from($name),
            methods: Rc::new(methods),
            fields,
            parents: Vec::new(),
            field_names: Vec::new(),
            field_index: Rc::new(HashMap::new()),
        }))
    }};
}

static BUILTIN_NAMES: LazyLock<std::collections::HashSet<&'static str>> = LazyLock::new(|| {
    [
        "اطبع", "ادخل", "طول", "مصفوفة", "مترابطة", "صحيح", "عشري", "منطق", "نص",
        "مميزة", "مجموع", "اكبر", "اصغر", "مدى", "مقرون", "هل_نوع", "نوع",
        "معكوس", "تحقق_اي", "تحقق_او", "معدل", "وقت", "الوقت", "عد_تلقائي",
        "جيب", "تجيب", "ظل", "جيب_عكسي", "تجيب_عكسي", "ظل_عكسي",
        "جذر", "مربع", "مكعب", "جذر_مكعب", "قوة", "مطلق", "قيمة_مطلقة",
        "ارضية", "سقف", "تقريب", "لوغ", "لوغ10", "لوغ2",
        "ط", "ه", "ن",
        "غفوة", "عشوائي", "عشوائي_صحيح",
        "استبدل", "اقسم", "يحتوي", "شطب",
        "استثناء", "افتح", "خطا_ن",
        "تتبع", "ضغط", "تصفية",
        "ابعث",
        "مضروب", "قم_اكبر", "حد_اعلى", "حد_ادنى", "راديان", "درجة", "قسمة_ومظم", "مغلق", "لانهاية", "ليس_رقم",
        "أس", "أس2", "أس10", "جذر_ثلاثي", "سلب", "صفر", "صحيح_قيمة", "لاشي",
        "الآن", "مللي", "توقيت", "تاريخ", "عداد", "زمان", "تحويل", "فرق",
        "سنة", "شهر", "يوم", "ساعة", "دقيقة", "ثانية", "يوم_الاسبوع", "هل_سنة_كبيسة", "ايام_الشهر",
        "نمط_طابق", "نمط_ابحث", "نمط_استبدل", "نمط_قسم", "نمط_جميع", "نمط_كل_التطابقات",
        "تشفير_64", "فك_تشفير_64", "تشفير_سداسي", "فك_تشفير_سداسي", "تشفير_رابط",
        "مفتاح_اكبر", "مفتاح_اصغر", "قيمة_اكبر", "قيمة_اصغر",
        "ادمج", "فرغ", "كرر", "موجود",
        "عد", "اقتران", "اختزل",
        "طلب", "طلب_نص", "طلب_كائن", "طلب_ارسال",
        "حاصل_ضرب", "نسبة", "تقريب_ل", "علامة",
        "بذرة", "منتظم",
        "مرتب", "حرف", "رقم", "ست_عشري",
        "اكبر_قاسم", "اصغر_مضاعف", "هل_اولية", "اولية", "فيبوناتشي", "ترتيبي", "تركيبي",
        "انحراف_معياري", "وسيط",
        "بداية_بـ", "نهاية_بـ", "اعلى", "اسفل", "تكرار_اعلى", "ملء", "توسيط", "قطع",
        "عدد", "اوجد", "اوجد_النهاية", "حرف_البداية", "حرف_النهاية", "معكوس_نص", "تكرار",
        "تحقق_من_الحرف", "اقلب", "ملء_صفر", "اول_حرف_كبير", "كل_اول_حرف_كبير", "اربط",
        "تجزئة", "تحويل_لارقام", "تحويل_من_ارقام", "تنسيق", "يحتوي_اي", "تقطيع",
        "مفاتيح", "قيم", "ازواج", "يحتوي_المفتاح", "ادمج_فهرس",
        "اقرا_ملف", "اكتب_ملف", "اضف_ملف", "يوجد", "ملف_الحجم", "التمس", "احذف_ملف",
        "اقرا_اسطر", "اسم_الملف", "امتداد_ملف", "المسار_المجلد",
        "مجموع_مربعات", "متوسط_وزني",
        "اختيار", "عينة", "خلط", "طبيعي", "برنولي", "عشوائي_نطاق", "جميل", "تحقق",
        "انضم", "مسار_مطلق", "نسخ", "نقل", "قائمة_مجلد", "مشي", "متغير_بيئي",
        "احذف_عنصر", "احطظ", "ادخل_في", "احذف_قيمة", "اختزال", "نفذ", "اخرج",
        "مسطح", "ضخم", "ادمج_فهرس_بـ", "تجميع", "عدد_تكرار", "تجزئة_قائمة", "افصل",
    ].iter().copied().collect()
});

struct Frame {
    arena_offset: usize,
    arena_len: usize,
    return_ip: usize,
    saved_handler_len: usize,
    saved_stack_len: usize,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        let mut vm = VM {
            frames: Vec::new(),
            frame_depth_limit: 1000,
            stack: Vec::with_capacity(256),
            globals: HashMap::new(),
            global_values: Vec::new(),
            global_name_index: HashMap::new(),
            module_cache: HashMap::new(),
            exception_handlers: Vec::new(),
            current_exception: None,
            current_class_name: None,
            current_instance: None,
            exception_hierarchy: HashMap::new(),
            search_dirs: Vec::new(),
            imported_modules: Vec::new(),
            locals_arena: Vec::with_capacity(1024),
            mc_methods_ptr: std::ptr::null(),
            mc_method: 0,
            mc_value: Value::Null,
            memory_used: 0,
            memory_limit: 256 * 1024 * 1024,
            jit_compiler: CraneliftJIT::with_symbols(|builder| {
                use crate::jit_runtime::*;
                builder.symbol("arabi_jit_load_int", arabi_jit_load_int as *const u8);
                builder.symbol("arabi_jit_store_int", arabi_jit_store_int as *const u8);
                builder.symbol("arabi_jit_for_range_next", arabi_jit_for_range_next as *const u8);
                builder.symbol("arabi_jit_for_range_init", arabi_jit_for_range_init as *const u8);
                builder.symbol("arabi_jit_build_list", arabi_jit_build_list as *const u8);
                builder.symbol("arabi_jit_list_append", arabi_jit_list_append as *const u8);
                builder.symbol("arabi_jit_subscript_int", arabi_jit_subscript_int as *const u8);
                builder.symbol("arabi_jit_subscript_int_imm", arabi_jit_subscript_int_imm as *const u8);
                builder.symbol("arabi_jit_store_subscript", arabi_jit_store_subscript as *const u8);
                builder.symbol("arabi_jit_store_subscript_int", arabi_jit_store_subscript_int as *const u8);
                builder.symbol("arabi_jit_store_subscript_add_imm", arabi_jit_store_subscript_add_imm as *const u8);
                builder.symbol("arabi_jit_swap_adjacent", arabi_jit_swap_adjacent as *const u8);
                builder.symbol("arabi_jit_subscript_gt", arabi_jit_subscript_gt as *const u8);
                builder.symbol("arabi_jit_pop_jump_if_subscript_gt", arabi_jit_pop_jump_if_subscript_gt as *const u8);
                builder.symbol("arabi_jit_get_time", arabi_jit_get_time as *const u8);
                builder.symbol("arabi_jit_print_int", arabi_jit_print_int as *const u8);
                builder.symbol("arabi_jit_print_float", arabi_jit_print_float as *const u8);
                builder.symbol("arabi_jit_print_str", arabi_jit_print_str as *const u8);
                builder.symbol("arabi_jit_print_value", arabi_jit_print_value as *const u8);
                builder.symbol("arabi_jit_print_sep", arabi_jit_print_sep as *const u8);
                builder.symbol("arabi_jit_print_newline", arabi_jit_print_newline as *const u8);
                builder.symbol("arabi_jit_load_float_as_int", arabi_jit_load_float_as_int as *const u8);
                builder.symbol("arabi_jit_store_float", arabi_jit_store_float as *const u8);
                builder.symbol("arabi_jit_load_global_to_local", arabi_jit_load_global_to_local as *const u8);
                builder.symbol("arabi_jit_call_func", arabi_jit_call_func as *const u8);
            }),
            modules: Vec::new(),
        };
        vm.register_builtins();
        vm.init_exception_hierarchy();
        vm
    }

    pub fn set_search_dirs(&mut self, dirs: Vec<PathBuf>) {
        self.search_dirs = dirs;
    }

    #[inline(always)]
    fn insert_global(&mut self, name: String, value: Value) {
        if let Some(&idx) = self.global_name_index.get(&name) {
            self.globals.insert(name, value.clone());
            self.global_values[idx] = value;
        } else {
            let idx = self.global_values.len();
            self.global_name_index.insert(name.clone(), idx);
            self.global_values.push(value.clone());
            self.globals.insert(name, value);
        }
    }

    fn register_builtins(&mut self) {
        let builtin_names = vec![
            "اطبع", "ادخل", "طول", "مصفوفة", "مترابطة", "صحيح", "عشري", "منطق", "نص",
            "مميزة", "مجموع", "اكبر", "اصغر", "مدى", "مقرون", "هل_نوع", "نوع",
            "معكوس", "تحقق_اي", "تحقق_او", "معدل", "وقت", "الوقت", "عد_تلقائي",
            "جيب", "تجيب", "ظل", "جيب_عكسي", "تجيب_عكسي", "ظل_عكسي",
            "جذر", "مربع", "مكعب", "جذر_مكعب", "قوة", "مطلق", "قيمة_مطلقة",
            "ارضية", "سقف", "تقريب", "لوغ", "لوغ10", "لوغ2",
            "ط", "ه", "ن",
            "غفوة", "عشوائي", "عشوائي_صحيح",
            "استبدل", "اقسم", "يحتوي", "شطب",
            "استثناء", "افتح", "خطا_ن",
            "تتبع", "ضغط", "تصفية",
            "ابعث",
            "مضروب", "قم_اكبر", "حد_اعلى", "حد_ادنى", "راديان", "درجة", "قسمة_ومظم", "مغلق", "لانهاية", "ليس_رقم",
            "أس", "أس2", "أس10", "جذر_ثلاثي", "سلب", "صفر", "صحيح_قيمة", "لاشي",
            "الآن", "مللي", "توقيت", "تاريخ", "عداد", "زمان", "تحويل", "فرق",
        "سنة", "شهر", "يوم", "ساعة", "دقيقة", "ثانية", "يوم_الاسبوع", "هل_سنة_كبيسة", "ايام_الشهر",
        "تشفير_64", "فك_تشفير_64", "تشفير_سداسي", "فك_تشفير_سداسي", "تشفير_رابط",
        "مفتاح_اكبر", "مفتاح_اصغر", "قيمة_اكبر", "قيمة_اصغر",
        "ادمج", "فرغ", "كرر", "موجود",
        "عد", "اقتران", "اختزل",
        "طلب", "طلب_نص", "طلب_كائن", "طلب_ارسال",
        "حاصل_ضرب", "نسبة", "تقريب_ل", "علامة",
        "نمط_طابق", "نمط_ابحث", "نمط_استبدل", "نمط_قسم", "نمط_جميع", "نمط_كل_التطابقات",
            "بذرة", "منتظم",
            "مرتب", "حرف", "رقم", "ست_عشري",
        "اكبر_قاسم", "اصغر_مضاعف", "هل_اولية", "اولية", "فيبوناتشي", "ترتيبي", "تركيبي",
            "انحراف_معياري", "وسيط",
            "بداية_بـ", "نهاية_بـ", "اعلى", "اسفل", "تكرار_اعلى", "ملء", "توسيط", "قطع",
            "عدد", "اوجد", "اوجد_النهاية", "حرف_البداية", "حرف_النهاية", "معكوس_نص", "تكرار",
            "تحقق_من_الحرف", "اقلب", "ملء_صفر", "اول_حرف_كبير", "كل_اول_حرف_كبير", "اربط",
            "تجزئة", "تحويل_لارقام", "تحويل_من_ارقام", "تنسيق", "يحتوي_اي", "تقطيع",
            "مفاتيح", "قيم", "ازواج", "يحتوي_المفتاح", "ادمج_فهرس",
            "اقرا_ملف", "اكتب_ملف", "اضف_ملف", "يوجد", "ملف_الحجم", "التمس", "احذف_ملف",
            "اقرا_اسطر", "اسم_الملف", "امتداد_ملف", "المسار_المجلد",
            "مجموع_مربعات", "متوسط_وزني",
            "اختيار", "عينة", "خلط", "طبيعي", "برنولي", "عشوائي_نطاق", "جميل", "تحقق",
            "انضم", "مسار_مطلق", "نسخ", "نقل", "قائمة_مجلد", "مشي", "متغير_بيئي",
            "احذف_عنصر", "احطظ", "ادخل_في", "احذف_قيمة", "اختزال", "نفذ", "اخرج",
        ];
        // NEW: Added builtin names for functional/dict operations
        let builtin_names_extra = vec![
            "مسطح", "ضخم", "ادمج_فهرس_بـ", "تجميع", "عدد_تكرار", "تجزئة_قائمة", "افصل",
        ];
        for name in builtin_names {
            self.insert_global(
                name.to_string(),
                Value::NativeFunction(Rc::new(NativeFunctionData {
                    name: name.to_string(),
                    arity: 0,
                })),
            );
        }
        for name in builtin_names_extra {
            self.insert_global(
                name.to_string(),
                Value::NativeFunction(Rc::new(NativeFunctionData {
                    name: name.to_string(),
                    arity: 0,
                })),
            );
        }
    }

    fn init_exception_hierarchy(&mut self) {
        self.exception_hierarchy.insert("استثناء".into(), vec![]);
        self.exception_hierarchy.insert("استثناء_خطا".into(), vec!["استثناء".to_string()]);
        self.exception_hierarchy.insert("استثناء_نوع".into(), vec!["استثناء_خطا".to_string()]);
        self.exception_hierarchy.insert("استثناء_قيمة".into(), vec!["استثناء_خطا".to_string()]);
        self.exception_hierarchy.insert("استثناء_اسم".into(), vec!["استثناء_خطا".to_string()]);
        self.exception_hierarchy.insert("استثناء_نطاق".into(), vec!["استثناء_خطا".to_string()]);
        self.exception_hierarchy.insert("استثناء_قسمة".into(), vec!["استثناء_خطا".to_string()]);
        self.exception_hierarchy.insert("استثناء_مفتاح".into(), vec!["استثناء_خطا".to_string()]);
        self.exception_hierarchy.insert("استثناء_ملف".into(), vec!["استثناء_خطا".to_string()]);
        self.exception_hierarchy.insert("استثناء_بنية".into(), vec!["استثناء_خطا".to_string()]);

        // Register exception child types as Value::Class in globals so user classes can inherit from them.
        // Note: استثناء itself remains a NativeFunction (registered in builtins), so we skip it here.
        let hierarchy_entries: Vec<(String, Vec<String>)> = self.exception_hierarchy.iter()
            .filter(|(k, _)| k.as_str() != "استثناء")
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        for (name, parents) in hierarchy_entries {
            self.insert_global(name.clone(), Value::Class(Rc::new(ClassData {
                name: Rc::from(name.as_str()),
                methods: Rc::new(HashMap::new()),
                fields: HashMap::new(),
                parents,
                field_names: Vec::new(),
                field_index: Rc::new(HashMap::new()),
            })));
        }
    }

    pub(crate) fn is_exception_child(&self, child: &str, target: &str) -> bool {
        if child == target { return true; }
        if let Some(parents) = self.exception_hierarchy.get(child) {
            parents.iter().any(|p| self.is_exception_child(p, target))
        } else {
            false
        }
    }

    pub fn push_frame(&mut self, arena_offset: usize, arena_len: usize, return_ip: usize) -> Result<(), RuntimeError> {
        if self.frames.len() >= self.frame_depth_limit {
            return Err(RuntimeError::new(
                &format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit)
            ));
        }
        let saved_handler_len = self.exception_handlers.len();
        let saved_stack_len = self.stack.len();
        self.frames.push(Frame {
            arena_offset,
            arena_len,
            return_ip,
            saved_handler_len,
            saved_stack_len,

        });
        Ok(())
    }

    pub fn pop_frame(&mut self) {
        if let Some(frame) = self.frames.pop() {
            self.exception_handlers.truncate(frame.saved_handler_len);
            self.stack.truncate(frame.saved_stack_len);
            self.locals_arena.truncate(frame.arena_offset);
        }
    }

    pub fn run_frame(&mut self, module: &mut BytecodeModule) -> Result<Value, RuntimeError> {
        let start_ip = self.current_frame().return_ip;
        self.execute_inner(module, start_ip, false)
    }

    pub fn run_imported_frame(&mut self, module_index: usize) -> Result<Value, RuntimeError> {
        let start_ip = self.current_frame().return_ip;
        let mut module_clone = self.imported_modules.get(module_index)
            .ok_or_else(|| RuntimeError::new("الوحدة المستوردة غير موجودة"))?
            .borrow()
            .clone();
        self.execute_inner(&mut module_clone, start_ip, false)
    }

    pub fn execute(&mut self, module: &mut BytecodeModule) -> Result<Value, RuntimeError> {
        self.modules.push(module.clone());
        let num_locals = module.num_locals.max(1);
        self.locals_arena.reserve(self.frame_depth_limit * 64);
        self.locals_arena.resize(num_locals, Value::Null);
        self.frames.push(Frame {
            arena_offset: 0,
            arena_len: num_locals,
            return_ip: 0,
            saved_handler_len: 0,
            saved_stack_len: 0,

        });
        let result = self.execute_inner(module, 0, true);
        // Sync module-scope locals to globals after execution completes.
        if result.is_ok() {
            let local_names = &module.local_names;
            let (base, len) = match self.frames.first() {
                Some(f) => (f.arena_offset, f.arena_len),
                None => return result,
            };
            let sync_data: Vec<(String, Value)> = local_names.iter().enumerate().filter_map(|(i, name)| {
                if i < len {
                    let val = self.locals_arena[base + i].clone();
                    if !matches!(val, Value::Null) {
                        Some((name.clone(), val))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }).collect();
            for (name, val) in sync_data {
                self.insert_global(name, val);
            }
        }
        result
    }

    pub fn send_generator(&mut self, gen: &crate::frame::SharedGenerator, send_val: Value, module: &mut BytecodeModule) -> Result<Value, RuntimeError> {
        let is_first_call;
        {
            let data = gen.borrow();
            is_first_call = data.ip == data.body;
        }
        {
            let mut data = gen.borrow_mut();
            data.last_sent = Some(send_val.clone());
        }
        let body;
        let local_vars;
        {
            let data = gen.borrow();
            body = data.ip;
            local_vars = data.locals.clone();
        }
        let num_locals = local_vars.len();
        let arena_offset = self.locals_arena.len();
        self.locals_arena.resize(arena_offset + num_locals, Value::Null);
        for (i, val) in local_vars.into_iter().enumerate() {
            self.locals_arena[arena_offset + i] = val;
        }
        self.push_frame(arena_offset, num_locals, body)?;
        // For subsequent calls (after a yield), push sent value so StoreFast can read it
        if !is_first_call {
            self.stack.push(send_val);
        }
        let result = match self.run_frame(module) {
            Ok(val) => {
                if let Some(frame) = self.frames.last() {
                    let new_ip = frame.return_ip;
                    let new_locals: Vec<Value> = self.locals_arena[frame.arena_offset..frame.arena_offset + frame.arena_len].to_vec();
                    {
                        let mut data = gen.borrow_mut();
                        data.ip = new_ip;
                        data.locals = new_locals;
                        data.last_sent = None;
                    }
                }
                val
            }
            Err(e) => {
                self.pop_frame();
                return Err(e);
            }
        };
        self.pop_frame();
        Ok(result)
    }

    #[allow(unused_unsafe)]
    fn execute_inner(&mut self, module: &mut BytecodeModule, start_ip: usize, flat: bool) -> Result<Value, RuntimeError> {
        let mut ip = start_ip;
        let mut result = Value::Null;
        let packed_ptr: *const Vec<u64> = &module.packed;
        let constants_ptr: *const Vec<arabi_compiler::compiler::Value> = &module.constants;

        let (packed_data, packed_len) = unsafe {
            let packed = &*packed_ptr;
            (packed.as_ptr(), packed.len())
        };
        let (consts_data, consts_len) = unsafe {
            let consts = &*constants_ptr;
            (consts.as_ptr(), consts.len())
        };

        // Precompute global name resolution: module.names index -> global_values index
        // This avoids HashMap lookups in the hot LOAD_GLOBAL path
        let mut global_resolution: Vec<usize> = Vec::with_capacity(module.names.len());
        for name in &module.names {
            if let Some(&idx) = self.global_name_index.get(name) {
                global_resolution.push(idx);
            } else {
                global_resolution.push(usize::MAX);
            }
        }
        let gr_data: *const usize = global_resolution.as_ptr();
        let gr_len: usize = global_resolution.len();

        let stack_ptr: *mut Vec<Value> = &mut self.stack as *mut Vec<Value>;
        let mut locals_ptr: *mut Value = unsafe {
            let frame = self.frames.last().unwrap();
            self.locals_arena.as_mut_ptr().add(frame.arena_offset)
        };
        let mut locals_len = self.frames.last().unwrap().arena_len;

        macro_rules! hot_push {
            ($v:expr) => { unsafe { (*stack_ptr).push($v) } }
        }
        macro_rules! hot_pop {
            () => { unsafe { (*stack_ptr).pop().unwrap_or(Value::Null) } }
        }
        macro_rules! hot_const {
            () => { unsafe { std::slice::from_raw_parts(consts_data, consts_len) } }
        }
        macro_rules! hot_packed {
            () => { unsafe { std::slice::from_raw_parts(packed_data, packed_len) } }
        }
        // try_or_catch: replaces ? on sub-frame calls. If the sub-frame returns an error,
        // check if the PARENT frame has an exception handler. If so, catch it here.
        // The sub-frame is still on the stack when this runs (pop_frame happens after).
        macro_rules! try_or_catch {
            ($result:expr) => {
                match $result {
                    Ok(val) => val,
                    Err(e) => {
                        let frame_depth = self.frames.len();
                        // The sub-frame is at frame_depth-1, parent is at frame_depth-2.
                        // Use parent frame's saved_handler_len to determine boundary.
                        let saved_len = if frame_depth >= 2 {
                            self.frames[frame_depth - 2].saved_handler_len
                        } else {
                            0
                        };
                        if self.exception_handlers.len() > saved_len {
                            if let Some(handler_ip) = self.exception_handlers.last().copied() {
                                let exc = e.into_value();
                                self.current_exception = Some(exc.clone());
                                self.exception_handlers.pop();
                                hot_push!(exc);
                                ip = handler_ip;
                                continue;
                            }
                        }
                        return Err(e);
                    }
                }
            };
        }
        macro_rules! runtime_error {
            ($msg:expr) => {{
                runtime_error!("خطا", $msg)
            }};
            ($class:expr, $msg:expr) => {{
                // Only catch handlers belonging to the current frame
                let frame_depth = self.frames.len();
                let saved_len = if frame_depth > 0 { self.frames[frame_depth - 1].saved_handler_len } else { 0 };
                if self.exception_handlers.len() > saved_len {
                    if let Some(handler_ip) = self.exception_handlers.last().copied() {
                        let exc = Value::Exception(Box::new(ExceptionData {
                            class_name: $class.to_string(),
                            message: $msg,
                            line: None,
                            call_stack: Vec::new(),
                        }));
                        self.current_exception = Some(exc.clone());
                        self.exception_handlers.pop();
                        hot_push!(exc);
                        ip = handler_ip;
                        continue;
                    }
                }
                let err_line = if ip > 0 { module.lines.get(ip - 1).copied().unwrap_or(0) as usize } else { 0 };
                return Err(RuntimeError::new_typed($class, $msg).with_line(err_line));
            }};
        }

        // Macro for binary arithmetic ops (SUB, MUL): INT_INT, FLOAT_FLOAT + magic fallback
        macro_rules! binary_op {
            ($op:tt, $magic_l:expr, $magic_r:expr, $fallback:ident) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                match (&left, &right) {
                    (Value::Integer(a), Value::Integer(b)) => hot_push!(Value::Integer(a $op b)),
                    (Value::Float(a), Value::Float(b)) => hot_push!(Value::Float(a $op b)),
                    _ => {
                        let mut found = false;
                        if let Value::Instance(rc) = &left {
                            if let Some(method) = rc.class.methods.get($magic_l) {
                                if let Ok(result) = method.call(&[left.clone(), right.clone()], &[], self, module) {
                                    hot_push!(result); found = true;
                                }
                            }
                        }
                        if !found {
                            if let Value::Instance(rc) = &right {
                                if let Some(method) = rc.class.methods.get($magic_r) {
                                    if let Ok(result) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                        hot_push!(result); found = true;
                                    }
                                }
                            }
                        }
                        if !found { match self.$fallback(left, right) { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                    }
                }
            }};
        }

        // Macro for binary division ops with zero-check: 4 type arms + magic fallback
        macro_rules! binary_div_op {
            ($magic_l:expr, $magic_r:expr, $fallback:ident) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                match (&left, &right) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Float(*a as f64 / *b as f64))
                    }
                    (Value::Float(a), Value::Float(b)) => {
                        if *b == 0.0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Float(a / b))
                    }
                    (Value::Integer(a), Value::Float(b)) => {
                        if *b == 0.0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Float(*a as f64 / b))
                    }
                    (Value::Float(a), Value::Integer(b)) => {
                        if *b == 0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Float(a / *b as f64))
                    }
                    _ => {
                        let mut found = false;
                        if let Value::Instance(rc) = &left {
                            if let Some(method) = rc.class.methods.get($magic_l) {
                                if let Ok(result) = method.call(&[left.clone(), right.clone()], &[], self, module) {
                                    hot_push!(result); found = true;
                                }
                            }
                        }
                        if !found {
                            if let Value::Instance(rc) = &right {
                                if let Some(method) = rc.class.methods.get($magic_r) {
                                    if let Ok(result) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                        hot_push!(result); found = true;
                                    }
                                }
                            }
                        }
                        if !found { match self.$fallback(left, right) { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                    }
                }
            }};
        }

        // Macro for binary modulo ops with zero-check: 4 type arms + magic fallback
        macro_rules! binary_mod_op {
            ($magic_l:expr, $magic_r:expr, $fallback:ident) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                match (&left, &right) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Integer(a % b))
                    }
                    (Value::Float(a), Value::Float(b)) => {
                        if *b == 0.0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Float(a % b))
                    }
                    (Value::Integer(a), Value::Float(b)) => {
                        if *b == 0.0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Float(*a as f64 % b))
                    }
                    (Value::Float(a), Value::Integer(b)) => {
                        if *b == 0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Float(a % *b as f64))
                    }
                    _ => {
                        let mut found = false;
                        if let Value::Instance(rc) = &left {
                            if let Some(method) = rc.class.methods.get($magic_l) {
                                if let Ok(result) = method.call(&[left.clone(), right.clone()], &[], self, module) {
                                    hot_push!(result); found = true;
                                }
                            }
                        }
                        if !found {
                            if let Value::Instance(rc) = &right {
                                if let Some(method) = rc.class.methods.get($magic_r) {
                                    if let Ok(result) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                        hot_push!(result); found = true;
                                    }
                                }
                            }
                        }
                        if !found { match self.$fallback(left, right) { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                    }
                }
            }};
        }

        // Macro for binary floor-div ops: INT_INT, FLOAT_FLOAT + magic fallback
        macro_rules! binary_floor_div_op {
            ($magic_l:expr, $magic_r:expr, $fallback:ident) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                match (&left, &right) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b == 0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        hot_push!(Value::Integer(a / b))
                    }
                    (Value::Float(a), Value::Float(b)) => hot_push!(Value::Integer((*a / b).floor() as i64)),
                    _ => {
                        let mut found = false;
                        if let Value::Instance(rc) = &left {
                            if let Some(method) = rc.class.methods.get($magic_l) {
                                if let Ok(result) = method.call(&[left.clone(), right.clone()], &[], self, module) {
                                    hot_push!(result); found = true;
                                }
                            }
                        }
                        if !found {
                            if let Value::Instance(rc) = &right {
                                if let Some(method) = rc.class.methods.get($magic_r) {
                                    if let Ok(result) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                        hot_push!(result); found = true;
                                    }
                                }
                            }
                        }
                        if !found { match self.$fallback(left, right) { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                    }
                }
            }};
        }

        // Macro for binary power ops: INT_INT (with overflow), FLOAT_FLOAT + magic fallback
        macro_rules! binary_power_op {
            ($magic_l:expr, $magic_r:expr, $fallback:ident) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                match (&left, &right) {
                    (Value::Integer(a), Value::Integer(b)) => {
                        if *b < 0 {
                            hot_push!(Value::Float((*a as f64).powf(*b as f64)));
                        } else if *b == 0 {
                            hot_push!(Value::Integer(1));
                        } else if *a == 0 {
                            hot_push!(Value::Integer(0));
                        } else {
                            match a.checked_pow(*b as u32) {
                                Some(result) => hot_push!(Value::Integer(result)),
                                None => hot_push!(Value::Float((*a as f64).powf(*b as f64))),
                            }
                        }
                    }
                    (Value::Float(a), Value::Float(b)) => hot_push!(Value::Float(a.powf(*b))),
                    (Value::Integer(a), Value::Float(b)) => hot_push!(Value::Float((*a as f64).powf(*b))),
                    (Value::Float(a), Value::Integer(b)) => hot_push!(Value::Float(a.powf(*b as f64))),
                    _ => {
                        let mut found = false;
                        if let Value::Instance(rc) = &left {
                            if let Some(method) = rc.class.methods.get($magic_l) {
                                if let Ok(result) = method.call(&[left.clone(), right.clone()], &[], self, module) {
                                    hot_push!(result); found = true;
                                }
                            }
                        }
                        if !found {
                            if let Value::Instance(rc) = &right {
                                if let Some(method) = rc.class.methods.get($magic_r) {
                                    if let Ok(result) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                        hot_push!(result); found = true;
                                    }
                                }
                            }
                        }
                        if !found { match self.$fallback(left, right) { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                    }
                }
            }};
        }

        // Macro for comparison ops: primitive fast path FIRST, then magic method fallback
        macro_rules! compare_op {
            ($magic_l:expr, $magic_r:expr, $match_fn:expr) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                // Fast path: primitive types go directly to match_fn (no Instance check needed)
                match (&left, &right) {
                    (Value::Integer(_), Value::Integer(_))
                    | (Value::Float(_), Value::Float(_))
                    | (Value::String(_), Value::String(_))
                    | (Value::Boolean(_), Value::Boolean(_))
                    | (Value::Null, Value::Null) => {
                        hot_push!(Value::Boolean(($match_fn)(&left, &right)));
                    }
                    _ => {
                        // Slow path: check Instance magic methods
                        let mut found = false;
                        let mut result = false;
                        if let Value::Instance(rc) = &left {
                            if let Some(method) = rc.class.methods.get($magic_l) {
                                if let Ok(r) = method.call(&[left.clone(), right.clone()], &[], self, module) {
                                    result = r.is_truthy(); found = true;
                                }
                            }
                        }
                        if !found {
                            if let Value::Instance(rc) = &right {
                                if let Some(method) = rc.class.methods.get($magic_r) {
                                    if let Ok(r) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                        result = r.is_truthy(); found = true;
                                    }
                                }
                            }
                        }
                        if !found { result = ($match_fn)(&left, &right); }
                        hot_push!(Value::Boolean(result));
                    }
                }
            }};
        }

        // Macro for integer-specialized comparison ops
        macro_rules! int_compare_op {
            ($match_fn:expr) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                let result = ($match_fn)(&left, &right);
                hot_push!(Value::Boolean(result));
            }};
        }

        // Macro for integer-specialized binary ops (used by Tier 3)
        macro_rules! int_int_binop {
            ($op:tt, $fallback:ident) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                match (left, right) {
                    (Value::Integer(a), Value::Integer(b)) => hot_push!(Value::Integer(a $op b)),
                    (a, b) => { let r = self.$fallback(a, b); match r { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                }
            }};
        }

        // Macro for integer-specialized division ops (with zero check)
        macro_rules! int_int_divop {
            ($fallback:ident) => {{
                let right = hot_pop!();
                let left = hot_pop!();
                match (&left, &right) {
                    (Value::Integer(_), Value::Integer(0)) => { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                    (Value::Integer(a), Value::Integer(b)) => hot_push!(Value::Float(*a as f64 / *b as f64)),
                    _ => { let r = self.$fallback(left, right); match r { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                }
            }};
        }

        // Macro for OP_ADD/SUBTRACT_LOCAL_IMM patterns
        macro_rules! local_imm_op {
            ($a:expr, $imm:expr, $locals_len:expr, $locals_ptr:expr, $op:tt, $fallback:ident) => {{
                if $a < $locals_len {
                    let val = unsafe {
                        let slot = &*($locals_ptr).add($a);
                        match slot {
                            Value::Cell(cell) => {
                                let borrowed = cell.borrow();
                                match &*borrowed {
                                    Value::Integer(n) => Value::Integer(n $op $imm),
                                    Value::Float(f) => Value::Float(f $op $imm as f64),
                                    _ => {
                                        drop(borrowed);
                                        let left = cell.borrow().clone();
                                        let right = Value::Integer($imm);
                                        match self.$fallback(left, right) {
                                            Ok(v) => v,
                                            Err(e) => { runtime_error!(e.class_name(), e.to_string()); }
                                        }
                                    }
                                }
                            }
                            Value::Integer(n) => Value::Integer(n $op $imm),
                            Value::Float(f) => Value::Float(f $op $imm as f64),
                            other => {
                                let left = other.clone();
                                let right = Value::Integer($imm);
                                match self.$fallback(left, right) {
                                    Ok(v) => v,
                                    Err(e) => { runtime_error!(e.class_name(), e.to_string()); }
                                }
                            }
                        }
                    };
                    hot_push!(val);
                } else {
                    hot_push!(Value::Null);
                }
            }};
        }

        loop {
            if ip >= hot_packed!().len() {
                break;
            }

            let pi = hot_packed!()[ip];
            ip += 1;
            let op = pi as u8;
            let a = ((pi >> 8) & 0xFF) as usize;
            let b = ((pi >> 16) & 0xFFFF) as usize;
            let c = (pi >> 32) as usize;
            let imm = ((pi >> 16) & 0xFFFF) as i16 as i64;

            match op {
                OP_HALT => break,
                OP_NOP => {}
                OP_DUP_TOP => {
                    let top = hot_pop!();
                    hot_push!(top.clone());
                    hot_push!(top);
                }

                OP_LOAD_CONST => {
                    let value = self.convert_value(&hot_const!()[b]);
                    hot_push!(value);
                }
                OP_LOAD_NONE => hot_push!(Value::Null),
                OP_LOAD_SUPER => {
                    if let Some(ref class_name) = self.current_class_name {
                        if let Some(Value::Class(d)) = self.globals.get(&**class_name) {
                            let parents = &d.parents;
                            if let Some(parent_name) = parents.first() {
                                let parent = self.globals.get(parent_name).cloned().unwrap_or(Value::Null);
                                hot_push!(parent);
                            } else {
                                runtime_error!("استثناء_اسم", "لا يوجد صنف اب".to_string());
                            }
                        } else {
                            runtime_error!("استثناء_اسم", "الصنف غير موجود".to_string());
                        }
                    } else {
                        runtime_error!("استثناء_بنية", "اصل() يجب ان تُستخدم داخل دالة صنف".to_string());
                    }
                }
                OP_LOAD_TRUE => hot_push!(Value::Boolean(true)),
                OP_LOAD_FALSE => hot_push!(Value::Boolean(false)),
                OP_POP_TOP => { hot_pop!(); }

                OP_LOAD_FAST => {
                    if a < locals_len {
                        let val = unsafe {
                            match &*locals_ptr.add(a) {
                                Value::Cell(cell) => cell.borrow().clone(),
                                other => other.clone(),
                            }
                        };
                        hot_push!(val);
                    } else {
                        hot_push!(Value::Null);
                    }
                }
                OP_STORE_FAST => {
                    if self.frames.len() == 1 && a < module.local_names.len() {
                        // Module scope: store to locals only (globals synced after execute_inner)
                        let value = hot_pop!();
                        if a < locals_len {
                            unsafe {
                                let slot = &mut *locals_ptr.add(a);
                                if let Value::Cell(cell) = slot {
                                    *cell.borrow_mut() = value;
                                } else {
                                    *slot = value;
                                }
                            }
                        } else {
                            unsafe {
                                *locals_ptr.add(a) = value;
                            }
                            locals_len = a + 1;
                        }
                        } else {
                            let value = hot_pop!();
                            if a < locals_len {
                                unsafe {
                                    let slot = &mut *locals_ptr.add(a);
                                    if let Value::Cell(cell) = slot {
                                        *cell.borrow_mut() = value;
                                    } else {
                                        *slot = value;
                                    }
                                }
                        } else {
                            unsafe {
                                *locals_ptr.add(a) = value;
                            }
                            locals_len = a + 1;
                        }
                    }
                }
                OP_INCREMENT_INT => {
                    if a < locals_len {
                        unsafe {
                            let slot = &mut *locals_ptr.add(a);
                            match slot {
                                Value::Cell(cell) => {
                                    let mut borrowed = cell.borrow_mut();
                                    if let Value::Integer(n) = &mut *borrowed {
                                        *n += b as i64;
                                    }
                                }
                                Value::Integer(n) => {
                                    *n += b as i64;
                                }
                                _ => {
                                    let old = std::mem::replace(slot, Value::Null);
                                    if let Value::Integer(n) = old {
                                        *slot = Value::Integer(n + b as i64);
                                    }
                                }
                            }
                        }
                    }
                }

                OP_INPLACE_ADD_STR_CONST => {
                    if a < locals_len {
                        let right_val = self.convert_value(&module.constants[b]);
                        if let Value::String(right_str) = &right_val {
                            let str_bytes = right_str.len();
                            if self.memory_used + str_bytes > self.memory_limit {
                                runtime_error!("استثناء_ذاكرة", format!("تجاوزت الحد الاقصى للذاكرة ({} ميجابايت)", self.memory_limit / (1024 * 1024)));
                            }
                            self.memory_used += str_bytes;
                            let right_str = (**right_str).clone();
                            unsafe {
                                let slot = &mut *locals_ptr.add(a);
                                match slot {
                                    Value::Cell(cell) => {
                                        let mut borrowed = cell.borrow_mut();
                                        if let Value::String(s) = &mut *borrowed {
                                            if Rc::strong_count(s) == 1 {
                                                Rc::get_mut(s).unwrap().push_str(&right_str);
                                            } else {
                                                let mut new_str = (**s).clone();
                                                new_str.push_str(&right_str);
                                                *s = Rc::new(new_str);
                                            }
                                        }
                                    }
                                    Value::String(s) => {
                                        if Rc::strong_count(s) == 1 {
                                            Rc::get_mut(s).unwrap().push_str(&right_str);
                                        } else {
                                            let mut new_str = (**s).clone();
                                            new_str.push_str(&right_str);
                                            *slot = Value::String(Rc::new(new_str));
                                        }
                                    }
                                    _ => {
                                        let old = std::mem::replace(slot, Value::Null);
                                        let left_str = old.to_string_value();
                                        *slot = Value::String(Rc::new(format!("{}{}", left_str, right_str)));
                                    }
                                }
                            }
                        }
                    }
                }

                OP_JUMP_WHILE_INCREMENTED_LT => {
                    let packed_val = (a as u64) | ((b as u64) << 16) | (((c & 0xFFFF) as u64) << 32) | (((c >> 16) as u64) << 48);
                    let local_idx = (packed_val & 0xFFFF) as usize;
                    let loop_start = ((packed_val >> 16) & 0xFFFF) as usize;
                    let increment = (((packed_val >> 32) & 0xFFFF) as u16) as i16 as i64;
                    let limit_field = (packed_val >> 48) & 0xFFFF;
                    // High bit (bit 15 of the 16-bit field) indicates local vs constant
                    let is_local_limit = (limit_field & 0x8000) != 0;
                    let limit_idx = (limit_field & 0x7FFF) as usize;

                    if local_idx < locals_len {
                        let (current, limit) = unsafe {
                            let cur = match &*locals_ptr.add(local_idx) {
                                Value::Integer(n) => *n,
                                _ => 0,
                            };
                            let lim = if is_local_limit {
                                // Fast path: read limit from locals slot (no constant pool lookup)
                                if limit_idx < locals_len {
                                    match &*locals_ptr.add(limit_idx) {
                                        Value::Integer(n) => *n,
                                        Value::Float(f) => *f as i64,
                                        _ => 0,
                                    }
                                } else { 0 }
                            } else {
                                // Fallback: read from constant pool
                                match &hot_const!()[limit_idx] {
                                    arabi_compiler::compiler::Value::Integer(n) => *n,
                                    arabi_compiler::compiler::Value::Float(f) => *f as i64,
                                    _ => 0,
                                }
                            };
                            (cur, lim)
                        };
                        if current < limit {
                            // SAFETY: bounds checked above; single mutable access to the local slot
                            unsafe {
                                if let Value::Integer(n) = &mut *locals_ptr.add(local_idx) {
                                    *n += increment;
                                }
                            }
                            ip = loop_start;
                        }
                    }
                }

                OP_FOR_RANGE => {
                    let packed_val = (a as u64) | ((b as u64) << 16) | (((c & 0xFFFF) as u64) << 32) | (((c >> 16) as u64) << 48);
                    let iter_local = (packed_val & 0xFFFF) as usize;
                    let target_local = ((packed_val >> 16) & 0xFFFF) as usize;
                    let idx_local = ((packed_val >> 32) & 0xFFFF) as usize;
                    let loop_end = ((packed_val >> 48) & 0xFFFF) as usize;

                    if iter_local < locals_len && idx_local < locals_len && target_local < locals_len {
                        let (start, end, step, counter) = unsafe {
                            let range = match &*locals_ptr.add(iter_local) {
                                Value::Range(d) => (d.start, d.end, d.step),
                                _ => return Err("لكل تتطلب نطاق".into()),
                            };
                            let idx = match &*locals_ptr.add(idx_local) {
                                Value::Integer(n) => *n,
                                _ => 0,
                            };
                            (range.0, range.1, range.2, idx)
                        };

                        let in_range = if step == 1 {
                            start + counter < end
                        } else if step == -1 {
                            start + counter > end
                        } else if step > 0 {
                            start + counter * step < end
                        } else {
                            start + counter * step > end
                        };

                        if in_range {
                            let current = if step == 1 || step == -1 {
                                start + counter
                            } else {
                                start + counter * step
                            };
                            let value = Value::Integer(current);
                            unsafe { *locals_ptr.add(target_local) = value; }
                            unsafe {
                                if let Value::Integer(n) = &mut *locals_ptr.add(idx_local) {
                                    *n += 1;
                                }
                            }
                        } else {
                            ip = loop_end;
                        }
                    }
                }

                OP_BINARY_ADD => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    match (&left, &right) {
                        (Value::Integer(a_val), Value::Integer(b_val)) => {
                            hot_push!(Value::Integer(a_val + b_val))
                        }
                        (Value::Float(a), Value::Float(b)) => hot_push!(Value::Float(a + b)),
                        (Value::Integer(a), Value::Float(b)) => hot_push!(Value::Float(*a as f64 + b)),
                        (Value::Float(a), Value::Integer(b)) => hot_push!(Value::Float(a + *b as f64)),
                        (Value::String(a), Value::String(b)) => {
                            let mut s = String::with_capacity(a.len() + b.len());
                            s.push_str(a);
                            s.push_str(b);
                            hot_push!(Value::String(Rc::new(s)));
                        },
                        _ => {
                            // Check for __اجمع__ on left operand
                            let mut found_magic = false;
                            if let Value::Instance(rc) = &left {
                                if let Some(method) = rc.class.methods.get("__اجمع__") {
                                    if let Ok(result) = method.call(&[left.clone(), right.clone()], &[], self, module) {
                                        hot_push!(result);
                                        found_magic = true;
                                    }
                                }
                            }
                            if !found_magic {
                                // Check for __اجمع_ع__ on right operand (reflected)
                                if let Value::Instance(rc) = &right {
                                    if let Some(method) = rc.class.methods.get("__اجمع_ع__") {
                                        if let Ok(result) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                            hot_push!(result);
                                            found_magic = true;
                                        }
                                    }
                                }
                            }
                            if !found_magic {
                                match self.add(left, right) {
                                    Ok(result) => hot_push!(result),
                                    Err(e) => {
                                        runtime_error!(e.class_name(), e.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
                OP_BINARY_SUBTRACT => { binary_op!(-, "__طرح__", "__طرح_ع__", sub); }
                OP_BINARY_MULTIPLY => { binary_op!(*, "__ضرب__", "__ضرب_ع__", mul); }
                OP_BINARY_DIVIDE => { binary_div_op!("__قسمة__", "__قسمة_ع__", div); }
                OP_BINARY_FLOOR_DIVIDE => { binary_floor_div_op!("__ضرب_القسمة__", "__ضرب_القسمة_ع__", floor_div); }
                OP_BINARY_POWER => { binary_power_op!("__قوة__", "__قوة_ع__", pow); }
                OP_BINARY_MODULO => { binary_mod_op!("__باقي__", "__باقي_ع__", modulo); }
                OP_UNARY_NEGATIVE => {
                    let value = hot_pop!();
                    if let Value::Integer(a) = &value {
                        hot_push!(Value::Integer(-a));
                    } else if let Value::Float(a) = &value {
                        hot_push!(Value::Float(-a));
                    } else if let Value::Instance(rc) = &value {
                        let method_clone = rc.class.methods.get("__سالب__").cloned();
                        if let Some(method) = method_clone {
                            match method.call(std::slice::from_ref(&value), &[], self, module) {
                                Ok(result) => hot_push!(result),
                                Err(e) => { runtime_error!(e.class_name(), e.to_string()); }
                            }
                        } else {
                            match self.neg(value) {
                                Ok(result) => hot_push!(result),
                                Err(e) => { runtime_error!(e.class_name(), e.to_string()); }
                            }
                        }
                    } else {
                        match self.neg(value) {
                            Ok(result) => hot_push!(result),
                            Err(e) => { runtime_error!(e.class_name(), e.to_string()); }
                        }
                    }
                }
                OP_UNARY_NOT => {
                    let value = hot_pop!();
                    hot_push!(Value::Boolean(!value.is_truthy()));
                }

                OP_BINARY_BIT_AND => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    match (&left, &right) {
                        (Value::Integer(a), Value::Integer(b)) => hot_push!(Value::Integer(a & b)),
                        _ => return Err(RuntimeError::new("عملية AND bitwise تتطلب صحيحاً").into()),
                    }
                }
                OP_BINARY_BIT_OR => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    match (&left, &right) {
                        (Value::Integer(a), Value::Integer(b)) => hot_push!(Value::Integer(a | b)),
                        _ => return Err(RuntimeError::new("عملية OR bitwise تتطلب صحيحاً").into()),
                    }
                }
                OP_BINARY_BIT_XOR => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    match (&left, &right) {
                        (Value::Integer(a), Value::Integer(b)) => hot_push!(Value::Integer(a ^ b)),
                        _ => return Err(RuntimeError::new("عملية XOR bitwise تتطلب صحيحاً").into()),
                    }
                }
                OP_BINARY_SHL => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    match (&left, &right) {
                        (Value::Integer(a), Value::Integer(b)) => {
                            if *b < 0 { return Err(RuntimeError::new("الإزاحة اليسرى لا يمكن ان تكون سالبة").into()); }
                            hot_push!(Value::Integer(a << b))
                        }
                        _ => return Err(RuntimeError::new("عملية إزاحة يسرى تتطلب صحيحاً").into()),
                    }
                }
                OP_BINARY_SHR => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    match (&left, &right) {
                        (Value::Integer(a), Value::Integer(b)) => {
                            if *b < 0 { return Err(RuntimeError::new("الإزاحة اليمين لا يمكن ان تكون سالبة").into()); }
                            hot_push!(Value::Integer(a >> b))
                        }
                        _ => return Err(RuntimeError::new("عملية إزاحة يمين تتطلب صحيحاً").into()),
                    }
                }
                OP_UNARY_BIT_NOT => {
                    let value = hot_pop!();
                    match &value {
                        Value::Integer(a) => hot_push!(Value::Integer(!a)),
                        _ => return Err(RuntimeError::new("عملية NOT bitwise تتطلب صحيحاً").into()),
                    }
                }

                OP_COMPARE_EQ => { compare_op!("__يساوي__", "__يساوي__", |l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a == b, (Value::Float(a), Value::Float(b)) => a == b, (Value::Boolean(a), Value::Boolean(b)) => a == b, _ => self.equals(l, r) }); }
                OP_COMPARE_NOT_EQ => { compare_op!("__لا_يساوي__", "__لا_يساوي__", |l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a != b, (Value::Float(a), Value::Float(b)) => a != b, (Value::Boolean(a), Value::Boolean(b)) => a != b, _ => !self.equals(l, r) }); }
                OP_COMPARE_LT => { compare_op!("__اصغر__", "__اكبر__", |l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a < b, (Value::Float(a), Value::Float(b)) => a < b, _ => self.less_than(l, r) }); }
                OP_COMPARE_GT => { compare_op!("__اكبر__", "__اصغر__", |l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a > b, (Value::Float(a), Value::Float(b)) => a > b, _ => self.greater_than(l, r) }); }
                OP_COMPARE_LT_EQ => { compare_op!("__اصغر_او_يساوي__", "__اكبر_او_يساوي__", |l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a <= b, (Value::Float(a), Value::Float(b)) => a <= b, _ => self.less_than(l, r) || self.equals(l, r) }); }
                OP_COMPARE_GT_EQ => { compare_op!("__اكبر_او_يساوي__", "__اصغر_او_يساوي__", |l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a >= b, (Value::Float(a), Value::Float(b)) => a >= b, _ => self.greater_than(l, r) || self.equals(l, r) }); }

                OP_COMPARE_IN => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    let mut found_magic = false;
                    let mut result = false;
                    if let Value::Instance(rc) = &right {
                        let method_clone = rc.class.methods.get("__يحتوي__").cloned();
                        if let Some(method) = method_clone {
                            if let Ok(r) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                result = r.is_truthy(); found_magic = true;
                            }
                        }
                    }
                    if !found_magic {
                        result = match &right {
                            Value::List(ref list) => {
                                let items = list.borrow();
                                items.iter().any(|item| self.equals(item, &left))
                            },
                            Value::Tuple(ref elements) => {
                                elements.iter().any(|item| self.equals(item, &left))
                            },
                            Value::Set(ref items) => {
                                let items = items.borrow();
                                items.iter().any(|item| self.equals(item, &left))
                            },
                            Value::String(ref s) => {
                                if let Value::String(sub) = &left {
                                    s.contains(&**sub)
                                } else { false }
                            },
                            Value::Dict(ref dict) => {
                                let pairs = dict.borrow();
                                pairs.iter().any(|(k, _)| self.equals(k, &left))
                            },
                            _ => runtime_error!("استثناء_نوع", "عملية 'في' غير مدعومة على هذا النوع".to_string()),
                        };
                    }
                    hot_push!(Value::Boolean(result));
                }
                OP_COMPARE_NOT_IN => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    let mut found_magic = false;
                    let mut result = true;
                    if let Value::Instance(rc) = &right {
                        let method_clone = rc.class.methods.get("__لا_يحتوي__").cloned();
                        if let Some(method) = method_clone {
                            if let Ok(r) = method.call(&[right.clone(), left.clone()], &[], self, module) {
                                result = r.is_truthy(); found_magic = true;
                            }
                        }
                    }
                    if !found_magic {
                        // Fallback: use !contains logic
                        result = match &right {
                            Value::List(ref list) => {
                                let items = list.borrow();
                                !items.iter().any(|item| self.equals(item, &left))
                            },
                            Value::Tuple(ref elements) => {
                                !elements.iter().any(|item| self.equals(item, &left))
                            },
                            Value::Set(ref items) => {
                                let items = items.borrow();
                                !items.iter().any(|item| self.equals(item, &left))
                            },
                            Value::String(ref s) => {
                                if let Value::String(sub) = &left {
                                    !s.contains(&**sub)
                                } else { true }
                            },
                            Value::Dict(ref dict) => {
                                let pairs = dict.borrow();
                                !pairs.iter().any(|(k, _)| self.equals(k, &left))
                            },
                            _ => runtime_error!("استثناء_نوع", "عملية 'ليس في' غير مدعومة على هذا النوع".to_string()),
                        };
                    }
                    hot_push!(Value::Boolean(result));
                }

                OP_COMPARE_IS => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    hot_push!(Value::Boolean(self.is_identical(&left, &right)));
                }

                OP_COMPARE_IS_NOT => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    hot_push!(Value::Boolean(!self.is_identical(&left, &right)));
                }

                // Tier 3 specialized integer opcodes — skip type checks
                OP_BINARY_ADD_INT_INT => { int_int_binop!(+, add); }
                OP_BINARY_SUB_INT_INT => { int_int_binop!(-, sub); }
                OP_BINARY_MUL_INT_INT => { int_int_binop!(*, mul); }
                OP_BINARY_DIV_INT_INT => { int_int_divop!(div); }
                OP_BINARY_FLOOR_DIV_INT_INT => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    match (&left, &right) {
                        (Value::Integer(_), Value::Integer(0)) => { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        (Value::Integer(a), Value::Integer(b)) => hot_push!(Value::Integer(a / b)),
                        _ => { let r = self.floor_div(left, right); match r { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                    }
                }
                OP_BINARY_MOD_INT_INT => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    match (&left, &right) {
                        (Value::Integer(_), Value::Integer(0)) => { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); }
                        (Value::Integer(a), Value::Integer(b)) => hot_push!(Value::Integer(a % b)),
                        _ => { let r = self.modulo(left, right); match r { Ok(v) => hot_push!(v), Err(e) => { runtime_error!(e.class_name(), e.to_string()); } } }
                    }
                }
                OP_COMPARE_LT_INT_INT => { int_compare_op!(|l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a < b, _ => self.less_than(l, r) }); }
                OP_COMPARE_GT_INT_INT => { int_compare_op!(|l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a > b, _ => self.greater_than(l, r) }); }
                OP_COMPARE_EQ_INT_INT => { int_compare_op!(|l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a == b, _ => self.equals(l, r) }); }
                OP_COMPARE_LE_INT_INT => { int_compare_op!(|l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a <= b, _ => self.less_than(l, r) || self.equals(l, r) }); }
                OP_COMPARE_GE_INT_INT => { int_compare_op!(|l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a >= b, _ => self.greater_than(l, r) || self.equals(l, r) }); }
                OP_COMPARE_NOT_EQ_INT_INT => { int_compare_op!(|l: &Value, r: &Value| match (l, r) { (Value::Integer(a), Value::Integer(b)) => a != b, (Value::Float(a), Value::Float(b)) => a != b, _ => !self.equals(l, r) }); }

                OP_SUBTRACT_LOCAL_IMM => { local_imm_op!(a, imm, locals_len, locals_ptr, -, sub); }
                OP_ADD_LOCAL_IMM => { local_imm_op!(a, imm, locals_len, locals_ptr, +, add); }
                OP_POP_JUMP_IF_LE_LOCAL_IMM => {
                    if a < locals_len {
                        let should_jump = unsafe {
                            match &*locals_ptr.add(a) {
                                Value::Integer(n) => !(*n <= imm),
                                Value::Float(f) => !(*f <= imm as f64),
                                _ => true,
                            }
                        };
                        if should_jump {
                            ip = c;
                            continue;
                        }
                    }
                }
                OP_POP_JUMP_IF_LT_LOCAL_IMM => {
                    if a < locals_len {
                        let should_jump = unsafe {
                            match &*locals_ptr.add(a) {
                                Value::Integer(n) => !(*n < imm),
                                Value::Float(f) => !(*f < imm as f64),
                                _ => true,
                            }
                        };
                        if should_jump {
                            ip = c;
                            continue;
                        }
                    }
                }

                OP_LOGICAL_AND => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    if !left.is_truthy() {
                        hot_push!(left);
                    } else {
                        hot_push!(Value::Boolean(right.is_truthy()));
                    }
                }
                OP_LOGICAL_OR => {
                    let right = hot_pop!();
                    let left = hot_pop!();
                    if left.is_truthy() {
                        hot_push!(left);
                    } else {
                        hot_push!(Value::Boolean(right.is_truthy()));
                    }
                }

                OP_STORE_NAME => {
                    let value = hot_pop!();
                    let name_str = &module.names[b];
                    self.insert_global(name_str.clone(), value);
                }
                OP_LOAD_NAME => {
                    let name_str = &module.names[b];
                    let value = if let Some(&idx) = self.global_name_index.get(name_str) {
                        self.global_values[idx].clone()
                    } else if !self.frames.is_empty() {
                        let frame = &self.frames[0];
                        if let Some(&pos) = module.local_name_map.get(name_str) {
                            if pos < frame.arena_len {
                                self.locals_arena[frame.arena_offset + pos].clone()
                            } else {
                                Value::Null
                            }
                        } else {
                            Value::Null
                        }
                    } else {
                        Value::Null
                    };
                    hot_push!(value);
                }
                OP_STORE_GLOBAL => {
                    let value = hot_pop!();
                    let name_str = &module.names[b];
                    self.insert_global(name_str.clone(), value);
                }
                OP_LOAD_GLOBAL => {
                    let value = if b < gr_len {
                        let idx = unsafe { *gr_data.add(b) };
                        if idx < self.global_values.len() {
                            self.global_values[idx].clone()
                        } else if idx == usize::MAX {
                            // Name wasn't resolved at frame start (defined during execution)
                            // Fall back to live lookup
                            let name_str = &module.names[b];
                            if let Some(&live_idx) = self.global_name_index.get(name_str) {
                                if live_idx < self.global_values.len() {
                                    self.global_values[live_idx].clone()
                                } else {
                                    Value::Null
                                }
                            } else {
                                Value::Null
                            }
                        } else {
                            Value::Null
                        }
                    } else {
                        Value::Null
                    };
                    hot_push!(value);
                }
                OP_STORE_ATTR => {
                    let value = hot_pop!();
                    let mut obj = hot_pop!();
                    let name_str = module.names[b].clone();
                    obj.set_attribute(name_str, value);
                }
                OP_LOAD_ATTR => {
                    let obj = hot_pop!();
                    let name_str = &module.names[b];
                    if let Some(attr) = obj.get_attribute(name_str) {
                        hot_push!(attr);
                    } else {
                        runtime_error!("استثناء_اسم", format!("خاصية غير موجودة: {}", name_str));
                    }
                }

                OP_JUMP_FORWARD => {
                    ip = ip - 1 + c;
                }
                OP_JUMP_BACKWARD => {
                    ip = c;
                }
                OP_POP_JUMP_IF_FALSE => {
                    let cond = hot_pop!();
                    if !cond.is_truthy() {
                        ip = ip - 1 + c;
                    }
                }
                OP_POP_JUMP_IF_TRUE => {
                    let cond = hot_pop!();
                    if cond.is_truthy() {
                        ip = ip - 1 + c;
                    }
                }

                OP_CALL_FUNCTION => {
                    let result = if a == 1 {
                        // SPECIALIZED PATH: 1-arg function calls (most common, e.g. fibonacci)
                        let arg0 = hot_pop!();
                        let func = hot_pop!();
                        match func {
                            Value::Function(f) if !f.is_generator && f.varargs_param.is_none() && f.kwargs_param.is_none() => {
                                // JIT fast path
                                let cc = f.call_count.get();
                                f.call_count.set(cc.wrapping_add(1));
                                if let Some(jit_entry) = f.jit_entry.get() {
                                    if self.jit_compiler.is_loop_compiled(f.body) {
                                        // Loop JIT: don't use here — falls through to defaults + loop JIT call
                                    } else if let Value::Integer(n) = arg0 {
                                        let func_ptr: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(jit_entry) };
                                        let result = func_ptr(n);
                                        hot_push!(Value::Integer(result));
                                        continue;
                                    }
                                } else if cc >= 1 && f.normal_param_count == 1 && !f.jit_attempted.get() {
                                    f.jit_attempted.set(true);
                                    if let Some(module_ref) = self.modules.last() {
                                        if let Some(entry) = self.jit_compiler.compile_function(
                                            &f.name, f.body, f.normal_param_count, f.num_locals, module_ref
                                        ) {
                                            f.jit_entry.set(Some(entry));
                                            if let Value::Integer(n) = arg0 {
                                                let func_ptr: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(entry) };
                                                let result = func_ptr(n);
                                                hot_push!(Value::Integer(result));
                                                continue;
                                            }
                                        }
                                    }
                                }
                                let num_locals = f.num_locals.max(1);
                                let body = f.body;
                                let module_index = f.module_index;
                                let arena_offset = self.locals_arena.len();
                                self.locals_arena.resize(arena_offset + num_locals, Value::Null);
                                let local_vars_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                                if !f.closure.is_empty() {
                                    for (idx, val) in f.closure.iter().cloned() {
                                        if idx < num_locals { unsafe { *local_vars_ptr.add(idx) = val; } }
                                    }
                                }
                                // Place the single arg directly (no Vec, no clone beyond move)
                                if !f.param_indices.is_empty() {
                                    let local_idx = f.param_indices[0];
                                    if local_idx < num_locals { unsafe { *local_vars_ptr.add(local_idx) = arg0; } }
                                } else if num_locals > 0 {
                                    unsafe { *local_vars_ptr = arg0; }
                                }
                                // Fill defaults for remaining params (if more than 1 param)
                                if f.normal_param_count > 1 {
                                    for i in 1..f.normal_param_count {
                                        if let Some(Some(default)) = f.defaults.get(i) {
                                            let local_idx = f.param_indices.get(i).copied().unwrap_or(i);
                                            if local_idx < num_locals { unsafe { *local_vars_ptr.add(local_idx) = default.clone(); } }
                                        }
                                    }
                                }
                                if self.frames.len() >= self.frame_depth_limit {
                                    runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                }
                                if !f.jit_attempted.get() && cc >= 5 {
                                    f.jit_attempted.set(true);
                                    if let Some(module_ref) = self.modules.last() {
                                        if let Some(entry) = self.jit_compiler.compile_loop_function(
                                            &f.name, f.body, f.normal_param_count, f.num_locals, module_ref,
                                            &f.param_indices
                                        ) {
                                            f.jit_entry.set(Some(entry));
                                        }
                                    }
                                }
                                if let Some(jit_entry) = f.jit_entry.get() {
                                    if self.jit_compiler.is_loop_compiled(f.body) {
                                        let func_ptr: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(jit_entry) };
                                        let result_i64 = {
                                            unsafe {
                                                crate::jit_runtime::set_jit_context(
                                                    self as *mut VM as *mut std::ffi::c_void,
                                                    self.modules.last().map_or(std::ptr::null_mut(), |m| m as *const _ as *mut std::ffi::c_void),
                                                );
                                            }
                                            let r = func_ptr(local_vars_ptr as i64);
                                            unsafe { crate::jit_runtime::clear_jit_context(); }
                                            r
                                        };
                                        if self.current_exception.is_some() {
                                            let exc = self.current_exception.take().unwrap();
                                            if let Value::Exception(ref e) = exc {
                                                return Err(RuntimeError::new_typed(&e.class_name, &e.message).with_line(e.line.unwrap_or(0)));
                                            }
                                        }
                                        hot_push!(Value::Integer(result_i64));
                                        continue;
                                    }
                                }
                                if let Some(idx) = module_index {
                                    self.push_frame(arena_offset, num_locals, body)?;
                                    let r = self.run_imported_frame(idx)?;
                                    self.pop_frame();
                                    r
                                } else {
                                    let ret_ip = ip;
                                    let saved_handler_len = self.exception_handlers.len();
                                    let saved_stack_len = unsafe { (*stack_ptr).len() };
                                    if self.frames.len() >= self.frame_depth_limit {
                                        runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                    }
                                    self.frames.push(Frame { arena_offset, arena_len: num_locals, return_ip: ret_ip, saved_handler_len, saved_stack_len });
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                    ip = body;
                                    continue;
                                }
                            }
                            Value::NativeFunction(ref nf) => {
                                // FAST PATH: common 1-arg builtins (bypass call_native dispatch)
                                let result = match nf.name.as_str() {
                                    "طول" => {
                                        match &arg0 {
                                            Value::List(items) => Ok(Value::Integer(items.borrow().len() as i64)),
                                            Value::String(s) => Ok(Value::Integer(if s.is_ascii() { s.len() } else { s.chars().count() } as i64)),
                                            Value::Tuple(items) => Ok(Value::Integer(items.len() as i64)),
                                            Value::Dict(pairs) => Ok(Value::Integer(pairs.borrow().len() as i64)),
                                            Value::Set(items) => Ok(Value::Integer(items.borrow().len() as i64)),
                                            _ => crate::builtins::call_native(&nf.name, &[arg0], &[], self, module),
                                        }
                                    }
                                    "اعلى" => {
                                        if let Value::String(s) = &arg0 {
                                            Ok(Value::String(Rc::new(s.to_uppercase())))
                                        } else {
                                            crate::builtins::call_native(&nf.name, &[arg0], &[], self, module)
                                        }
                                    }
                                    "اسفل" => {
                                        if let Value::String(s) = &arg0 {
                                            Ok(Value::String(Rc::new(s.to_lowercase())))
                                        } else {
                                            crate::builtins::call_native(&nf.name, &[arg0], &[], self, module)
                                        }
                                    }
                                    "شطب" => {
                                        if let Value::String(s) = &arg0 {
                                            let trimmed = s.trim();
                                            if trimmed.len() == s.len() {
                                                Ok(arg0.clone())
                                            } else {
                                                Ok(Value::String(Rc::new(trimmed.to_string())))
                                            }
                                        } else {
                                            crate::builtins::call_native(&nf.name, &[arg0], &[], self, module)
                                        }
                                    }
                                    "مربع" => {
                                        if let Value::Integer(n) = &arg0 {
                                            Ok(Value::Integer(n * n))
                                        } else if let Value::Float(f) = &arg0 {
                                            Ok(Value::Float(f * f))
                                        } else {
                                            crate::builtins::call_native(&nf.name, &[arg0], &[], self, module)
                                        }
                                    }
                                    "مطلق" | "قيمة_مطلقة" => {
                                        if let Value::Integer(n) = &arg0 {
                                            Ok(Value::Integer(n.abs()))
                                        } else if let Value::Float(f) = &arg0 {
                                            Ok(Value::Float(f.abs()))
                                        } else {
                                            crate::builtins::call_native(&nf.name, &[arg0], &[], self, module)
                                        }
                                    }
                                    _ => crate::builtins::call_native(&nf.name, &[arg0], &[], self, module),
                                };
                                let result = try_or_catch!(result);
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                            other => {
                                let args = vec![arg0];
                                let result = try_or_catch!(other.call(&args, &[], self, module));
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                        }
                    } else if a == 0 {
                        // SPECIALIZED PATH: 0-arg function calls
                        let func = hot_pop!();
                        match func {
                            Value::Generator(d) => {
                                let result = self.send_generator(&d, Value::Null, module).unwrap_or_else(|e| { self.current_exception = Some(e.into_value()); Value::Null });
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                            Value::Function(f) if !f.is_generator && f.varargs_param.is_none() && f.kwargs_param.is_none() => {
                                let cc = f.call_count.get();
                                f.call_count.set(cc.wrapping_add(1));
                                if !f.jit_attempted.get() && cc >= 5 {
                                    f.jit_attempted.set(true);
                                    if let Some(module_ref) = self.modules.last() {
                                        if let Some(entry) = self.jit_compiler.compile_loop_function(
                                            &f.name, f.body, f.normal_param_count, f.num_locals, module_ref,
                                            &f.param_indices
                                        ) {
                                            f.jit_entry.set(Some(entry));
                                        }
                                    }
                                }
                                let num_locals = f.num_locals.max(1);
                                let body = f.body;
                                let module_index = f.module_index;
                                let arena_offset = self.locals_arena.len();
                                self.locals_arena.resize(arena_offset + num_locals, Value::Null);
                                let local_vars_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                                if !f.closure.is_empty() {
                                    for (idx, val) in f.closure.iter().cloned() {
                                        if idx < num_locals { unsafe { *local_vars_ptr.add(idx) = val; } }
                                    }
                                }
                                for i in 0..f.normal_param_count {
                                    if let Some(Some(default)) = f.defaults.get(i) {
                                        let local_idx = f.param_indices.get(i).copied().unwrap_or(i);
                                        if local_idx < num_locals {
                                            unsafe { *local_vars_ptr.add(local_idx) = default.clone(); }
                                        }
                                    }
                                }
                                if self.frames.len() >= self.frame_depth_limit {
                                    runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                }
                                if let Some(jit_entry) = f.jit_entry.get() {
                                    if self.jit_compiler.is_loop_compiled(f.body) {
                                        let func_ptr: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(jit_entry) };
                                        let result_i64 = {
                                            unsafe {
                                                crate::jit_runtime::set_jit_context(
                                                    self as *mut VM as *mut std::ffi::c_void,
                                                    self.modules.last().map_or(std::ptr::null_mut(), |m| m as *const _ as *mut std::ffi::c_void),
                                                );
                                            }
                                            let r = func_ptr(local_vars_ptr as i64);
                                            unsafe { crate::jit_runtime::clear_jit_context(); }
                                            r
                                        };
                                        if self.current_exception.is_some() {
                                            let exc = self.current_exception.take().unwrap();
                                            if let Value::Exception(ref e) = exc {
                                                return Err(RuntimeError::new_typed(&e.class_name, &e.message).with_line(e.line.unwrap_or(0)));
                                            }
                                        }
                                        hot_push!(Value::Integer(result_i64));
                                        continue;
                                    }
                                }
                                if let Some(idx) = module_index {
                                    self.push_frame(arena_offset, num_locals, body)?;
                                    let r = self.run_imported_frame(idx)?;
                                    self.pop_frame();
                                    r
                                } else {
                                    let ret_ip = ip;
                                    let saved_handler_len = self.exception_handlers.len();
                                    let saved_stack_len = unsafe { (*stack_ptr).len() };
                                    if self.frames.len() >= self.frame_depth_limit {
                                        runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                    }
                                    self.frames.push(Frame { arena_offset, arena_len: num_locals, return_ip: ret_ip, saved_handler_len, saved_stack_len });
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                    ip = body;
                                    continue;
                                }
                            }
                            Value::NativeFunction(ref nf) => {
                                if nf.name == "وقت" {
                                    let now = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default();
                                    let secs = now.as_secs();
                                    let millis = now.subsec_millis() as u64;
                                    let total_millis = secs * 1000 + millis;
                                    let dict = vec![
                                        (Value::String(Rc::new("الثوان".to_string())), Value::Float(secs as f64)),
                                        (Value::String(Rc::new("الملي_ثانية".to_string())), Value::Integer(millis as i64)),
                                        (Value::String(Rc::new("الكل".to_string())), Value::Float(total_millis as f64 / 1000.0)),
                                    ];
                                    hot_push!(Value::Dict(crate::frame::SharedDict::new(dict)));
                                    continue;
                                }
                                let result = try_or_catch!(Value::NativeFunction(Rc::clone(nf)).call(&[], &[], self, module));
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                            other => {
                                let result = try_or_catch!(other.call(&[], &[], self, module));
                                hot_push!(result);
                                continue;
                            }
                        }
                    } else if a == 2 {
                        // SPECIALIZED PATH: 2-arg function calls (tail-recursive factorial)
                        let arg1 = hot_pop!();
                        let arg0 = hot_pop!();
                        let func = hot_pop!();
                        match func {
                            Value::Function(f) if !f.is_generator && f.varargs_param.is_none() && f.kwargs_param.is_none() => {
                                let cc = f.call_count.get();
                                f.call_count.set(cc.wrapping_add(1));
                                let num_locals = f.num_locals.max(1);
                                let body = f.body;
                                let module_index = f.module_index;
                                let arena_offset = self.locals_arena.len();
                                self.locals_arena.resize(arena_offset + num_locals, Value::Null);
                                let local_vars_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                                if !f.closure.is_empty() {
                                    for (idx, val) in f.closure.iter().cloned() {
                                        if idx < num_locals { unsafe { *local_vars_ptr.add(idx) = val; } }
                                    }
                                }
                                // Place 2 args directly (no Vec)
                                let li0 = f.param_indices.get(0).copied().unwrap_or(0);
                                if li0 < num_locals { unsafe { *local_vars_ptr.add(li0) = arg0; } }
                                let li1 = f.param_indices.get(1).copied().unwrap_or(1);
                                if li1 < num_locals { unsafe { *local_vars_ptr.add(li1) = arg1; } }
                                // Fill defaults for remaining params
                                if f.normal_param_count > 2 {
                                    for i in 2..f.normal_param_count {
                                        if let Some(Some(default)) = f.defaults.get(i) {
                                            let local_idx = f.param_indices.get(i).copied().unwrap_or(i);
                                            if local_idx < num_locals { unsafe { *local_vars_ptr.add(local_idx) = default.clone(); } }
                                        }
                                    }
                                }
                                // JIT compile loop function if threshold reached
                                if !f.jit_attempted.get() && cc >= 5 {
                                    f.jit_attempted.set(true);
                                    if let Some(module_ref) = self.modules.last() {
                                        if let Some(entry) = self.jit_compiler.compile_loop_function(
                                            &f.name, f.body, f.normal_param_count, f.num_locals, module_ref,
                                            &f.param_indices
                                        ) {
                                            f.jit_entry.set(Some(entry));
                                        }
                                    }
                                }
                                if self.frames.len() >= self.frame_depth_limit {
                                    runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                }
                                // JIT fast path
                                if let Some(jit_entry) = f.jit_entry.get() {
                                    if self.jit_compiler.is_loop_compiled(f.body) {
                                        let func_ptr: extern "C" fn(i64) -> i64 = unsafe { std::mem::transmute(jit_entry) };
                                        let result_i64 = {
                                            unsafe {
                                                crate::jit_runtime::set_jit_context(
                                                    self as *mut VM as *mut std::ffi::c_void,
                                                    self.modules.last().map_or(std::ptr::null_mut(), |m| m as *const _ as *mut std::ffi::c_void),
                                                );
                                            }
                                            let r = func_ptr(local_vars_ptr as i64);
                                            unsafe { crate::jit_runtime::clear_jit_context(); }
                                            r
                                        };
                                        if self.current_exception.is_some() {
                                            let exc = self.current_exception.take().unwrap();
                                            if let Value::Exception(ref e) = exc {
                                                return Err(RuntimeError::new_typed(&e.class_name, &e.message).with_line(e.line.unwrap_or(0)));
                                            }
                                        }
                                        hot_push!(Value::Integer(result_i64));
                                        continue;
                                    }
                                }
                                if let Some(idx) = module_index {
                                    self.push_frame(arena_offset, num_locals, body)?;
                                    let r = self.run_imported_frame(idx)?;
                                    self.pop_frame();
                                    r
                                } else {
                                    let ret_ip = ip;
                                    let saved_handler_len = self.exception_handlers.len();
                                    let saved_stack_len = unsafe { (*stack_ptr).len() };
                                    if self.frames.len() >= self.frame_depth_limit {
                                        runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                    }
                                    self.frames.push(Frame { arena_offset, arena_len: num_locals, return_ip: ret_ip, saved_handler_len, saved_stack_len });
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                    ip = body;
                                    continue;
                                }
                            }
                            Value::NativeFunction(ref nf) => {
                                // FAST PATH: common 2-arg builtins
                                let result = match nf.name.as_str() {
                                    "اقسم" => {
                                        if let Value::String(s) = &arg0 {
                                            if let Value::String(sep) = &arg1 {
                                                let parts: Vec<Value> = s.split(sep.as_str()).map(|p| Value::String(Rc::new(p.to_string()))).collect();
                                                Ok(Value::List(crate::frame::SharedList::new(parts)))
                                            } else {
                                                crate::builtins::call_native(&nf.name, &[arg0, arg1], &[], self, module)
                                            }
                                        } else {
                                            crate::builtins::call_native(&nf.name, &[arg0, arg1], &[], self, module)
                                        }
                                    }
                                    _ => crate::builtins::call_native(&nf.name, &[arg0, arg1], &[], self, module),
                                };
                                let result = try_or_catch!(result);
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                            other => {
                                let args = vec![arg0, arg1];
                                let result = try_or_catch!(other.call(&args, &[], self, module));
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                        }
                    } else {
                        // GENERAL PATH: 3+ args
                        let mut args = Vec::with_capacity(a);
                        for _ in 0..a {
                            args.push(hot_pop!());
                        }
                        args.reverse();
                        let func = hot_pop!();
                        match func {
                            Value::Function(f) if !f.is_generator && f.varargs_param.is_none() && f.kwargs_param.is_none() => {
                                let num_locals = f.num_locals.max(1);
                                let body = f.body;
                                let module_index = f.module_index;
                                let normal_count = f.normal_param_count;
                                let arena_offset = self.locals_arena.len();
                                self.locals_arena.resize(arena_offset + num_locals, Value::Null);
                                let local_vars_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                                if !f.closure.is_empty() {
                                    for (idx, val) in f.closure.iter().cloned() {
                                        if idx < num_locals { unsafe { *local_vars_ptr.add(idx) = val; } }
                                    }
                                }
                                let mut arg_iter = args.into_iter();
                                for i in 0..normal_count {
                                    let local_idx = f.param_indices.get(i).copied().unwrap_or(i);
                                    if local_idx < num_locals {
                                        if let Some(arg) = arg_iter.next() {
                                            unsafe { *local_vars_ptr.add(local_idx) = arg; }
                                        } else if let Some(Some(default)) = f.defaults.get(i) {
                                            unsafe { *local_vars_ptr.add(local_idx) = default.clone(); }
                                        }
                                    }
                                }
                                if let Some(idx) = module_index {
                                    self.push_frame(arena_offset, num_locals, body)?;
                                    let r = self.run_imported_frame(idx)?;
                                    self.pop_frame();
                                    r
                                } else {
                                    let ret_ip = ip;
                                    let saved_handler_len = self.exception_handlers.len();
                                    let saved_stack_len = unsafe { (*stack_ptr).len() };
                                    if self.frames.len() >= self.frame_depth_limit {
                                        runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                    }
                                    self.frames.push(Frame { arena_offset, arena_len: num_locals, return_ip: ret_ip, saved_handler_len, saved_stack_len });
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                    ip = body;
                                    continue;
                                }
                            }
                            Value::Generator(d) => {
                                let send_val = args.first().cloned().unwrap_or(Value::Null);
                                let result = self.send_generator(&d, send_val, module).unwrap_or_else(|e| { self.current_exception = Some(e.into_value()); Value::Null });
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                            Value::Class(d) => {
                                let nf = d.field_names.len().min(4) as u8;
                                // Constructor inlining: simple __تهيئة__(self, x, y) with matching args
                                if !d.field_names.is_empty() && d.field_names.len() == args.len() && args.len() <= 4 {
                                    let mut inline: [Value; 4] = core::array::from_fn(|_| Value::Null);
                                    for (i, v) in args.into_iter().enumerate() {
                                        inline[i] = v;
                                    }
                                    hot_push!(Value::Instance(Rc::new(InstanceData {
                                        class: Rc::clone(&d),
                                        inline_fields: RefCell::new(inline),
                                        num_inline_fields: nf,
                                        extra_fields: RefCell::new(None),
                                    })));
                                    continue;
                                }
                let instance = Value::Instance(Rc::new(InstanceData {
                                    class: Rc::clone(&d),
                                    inline_fields: RefCell::new(core::array::from_fn(|_| Value::Null)),
                                    num_inline_fields: nf,
                                    extra_fields: RefCell::new(None),
                }));
                                if let Some(Value::Function(f)) = d.methods.get("__تهيئة__") {
                                    let param_indices = &f.param_indices;
                                    let body = f.body;
                                    let num_locals = f.num_locals.max(1);
                                    let arena_offset = self.locals_arena.len();
                                    self.locals_arena.resize(arena_offset + num_locals, Value::Null);
                                    let local_vars_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                                    if let Some(&idx) = param_indices.first() {
                                        unsafe { *local_vars_ptr.add(idx) = instance.clone(); }
                                    }
                                    for (i, arg) in args.into_iter().enumerate() {
                                        let local_idx = param_indices.get(i + 1).copied().unwrap_or(i + 1);
                                        if local_idx < num_locals {
                                            unsafe { *local_vars_ptr.add(local_idx) = arg; }
                                        }
                                    }
                                    self.push_frame(arena_offset, num_locals, body)?;
                                    let _ = self.run_frame(module)?;
                                    self.pop_frame();
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                }
                                hot_push!(instance);
                                continue;
                            }
                            Value::NativeFunction(ref nf) => {
                                // FAST PATH: common 3-arg builtins (bypass call_native dispatch)
                                let result = match nf.name.as_str() {
                                    "استبدل" => {
                                        if args.len() >= 3 {
                                            if let (Value::String(s), Value::String(from), Value::String(to)) = (&args[0], &args[1], &args[2]) {
                                                Ok(Value::String(Rc::new(s.replace(&**from, to))))
                                            } else {
                                                crate::builtins::call_native(&nf.name, &args, &[], self, module)
                                            }
                                        } else {
                                            crate::builtins::call_native(&nf.name, &args, &[], self, module)
                                        }
                                    }
                                    _ => crate::builtins::call_native(&nf.name, &args, &[], self, module),
                                };
                                let result = try_or_catch!(result);
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                            other => {
                                let result = try_or_catch!(other.call(&args, &[], self, module));
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(result);
                                continue;
                            }
                        }
                    };
                    {
                        let frame = self.frames.last().unwrap();
                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                        locals_len = frame.arena_len;
                    }
                    hot_push!(result);
                }
                OP_CALL_FUNCTION_KW => {
                    let kwargs_count = {
                        let top = hot_pop!();
                        match top {
                            Value::Integer(n) => n as usize,
                            _ => 0,
                        }
                    };
                    let mut kwargs = Vec::new();
                    for _ in 0..kwargs_count {
                        let val = hot_pop!();
                        let key = hot_pop!();
                        if let Value::String(k) = key {
                            kwargs.push((k.to_string(), val));
                        }
                    }
                    kwargs.reverse();
                    let mut args = Vec::with_capacity(a);
                    for _ in 0..a {
                        args.push(hot_pop!());
                    }
                    args.reverse();
                    let func = hot_pop!();
                    let result = try_or_catch!(func.call(&args, &kwargs, self, module));
                    {
                        let frame = self.frames.last().unwrap();
                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                        locals_len = frame.arena_len;
                    }
                    hot_push!(result);
                }
                OP_CALL_FUNCTION_UNPACKED => {
                    // Stack: func, normal_args..., unpacked_args_iterables..., unpacked_kwargs_dicts...
                    // Encoding: a=normal_args, b=star_args, c=kw_dicts
                    let mut unpacked_kwargs = Vec::new();
                    for _ in 0..c {
                        let dict_val = hot_pop!();
                        if let Value::Dict(pairs) = dict_val {
                            for (k, v) in pairs.borrow().iter() {
                                if let Value::String(key) = k {
                                    unpacked_kwargs.push((key.to_string(), v.clone()));
                                }
                            }
                        }
                    }
                    unpacked_kwargs.reverse();

                    let mut all_args_lists: Vec<Vec<Value>> = Vec::new();
                    for _ in 0..b {
                        let iterable = hot_pop!();
                        match iterable {
                            Value::List(items) => all_args_lists.push(items.borrow().clone()),
                            Value::Tuple(items) => all_args_lists.push(items.as_ref().clone()),
                            other => all_args_lists.push(vec![other]),
                        }
                    }
                    all_args_lists.reverse();
                    let mut all_args: Vec<Value> = Vec::new();
                    for list in all_args_lists {
                        all_args.extend(list);
                    }

                    let mut normal_args = Vec::with_capacity(a);
                    for _ in 0..a {
                        normal_args.push(hot_pop!());
                    }
                    normal_args.reverse();

                    let mut final_args = normal_args;
                    final_args.extend(all_args);

                    let func = hot_pop!();
                    let result = try_or_catch!(func.call(&final_args, &unpacked_kwargs, self, module));
                    {
                        let frame = self.frames.last().unwrap();
                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                        locals_len = frame.arena_len;
                    }
                    hot_push!(result);
                }
                OP_RETURN_VALUE => {
                    if flat {
                        let ret_val = hot_pop!();
                        let frame = self.frames.pop()
                            .ok_or_else(|| RuntimeError::new("إرجاع بدون إطار استدعاء"))?;
                        if self.exception_handlers.len() > frame.saved_handler_len {
                            self.exception_handlers.truncate(frame.saved_handler_len);
                        }
                        let ret_ip = frame.return_ip;
                        if self.stack.len() > frame.saved_stack_len {
                            self.stack.truncate(frame.saved_stack_len);
                        }
                        self.locals_arena.truncate(frame.arena_offset);
                        if let Some(prev) = self.frames.last() {
                            locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(prev.arena_offset) };
                            locals_len = prev.arena_len;
                            unsafe { (*stack_ptr).push(ret_val); }
                            ip = ret_ip;
                            continue;
                        } else {
                            result = ret_val;
                            break;
                        }
                    } else {
                        result = hot_pop!();
                        break;
                    }
                }
                OP_TAIL_CALL => {
                    let argc = a as usize;
                    // Stack-allocate args (no heap allocation!)
                    let mut tco_arg0 = Value::Null;
                    let mut tco_arg1 = Value::Null;
                    let mut tco_arg2 = Value::Null;
                    let mut tco_arg3 = Value::Null;
                    // Pop args in reverse order
                    match argc {
                        4 => { tco_arg3 = hot_pop!(); tco_arg2 = hot_pop!(); tco_arg1 = hot_pop!(); tco_arg0 = hot_pop!(); }
                        3 => { tco_arg2 = hot_pop!(); tco_arg1 = hot_pop!(); tco_arg0 = hot_pop!(); }
                        2 => { tco_arg1 = hot_pop!(); tco_arg0 = hot_pop!(); }
                        1 => { tco_arg0 = hot_pop!(); }
                        _ => {}
                    }
                    let func_val = hot_pop!();

                    if let Value::Function(ref f) = func_val {
                        if !f.is_generator {
                            let frame = self.frames.last_mut().unwrap();
                            let arena_offset = frame.arena_offset;
                            let num_locals = f.num_locals.max(1);
                            if num_locals > frame.arena_len {
                                self.locals_arena.resize(arena_offset + num_locals, Value::Null);
                                for i in frame.arena_len..num_locals {
                                    self.locals_arena[arena_offset + i] = Value::Null;
                                }
                            }
                            if !f.closure.is_empty() {
                                for (idx, val) in &f.closure {
                                    if *idx < num_locals { self.locals_arena[arena_offset + *idx] = val.clone(); }
                                }
                            }
                            // Write args directly from stack vars to arena (no Vec, no heap!)
                            if argc > 0 { let li = f.param_indices.get(0).copied().unwrap_or(0); if li < num_locals { self.locals_arena[arena_offset + li] = std::mem::replace(&mut tco_arg0, Value::Null); } }
                            if argc > 1 { let li = f.param_indices.get(1).copied().unwrap_or(1); if li < num_locals { self.locals_arena[arena_offset + li] = std::mem::replace(&mut tco_arg1, Value::Null); } }
                            if argc > 2 { let li = f.param_indices.get(2).copied().unwrap_or(2); if li < num_locals { self.locals_arena[arena_offset + li] = std::mem::replace(&mut tco_arg2, Value::Null); } }
                            if argc > 3 { let li = f.param_indices.get(3).copied().unwrap_or(3); if li < num_locals { self.locals_arena[arena_offset + li] = std::mem::replace(&mut tco_arg3, Value::Null); } }
                            if f.normal_param_count > argc {
                                for i in argc..f.normal_param_count {
                                    if let Some(Some(default)) = f.defaults.get(i) {
                                        let li = f.param_indices.get(i).copied().unwrap_or(i);
                                        if li < num_locals { self.locals_arena[arena_offset + li] = default.clone(); }
                                    }
                                }
                            }

                            frame.arena_len = num_locals;
                            locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                            locals_len = num_locals;
                            ip = f.body;
                            continue;
                        }
                    }
                    // Fallback: non-function or generator
                    let mut args = Vec::with_capacity(argc);
                    match argc {
                        4 => { args.push(std::mem::replace(&mut tco_arg0, Value::Null)); args.push(std::mem::replace(&mut tco_arg1, Value::Null)); args.push(std::mem::replace(&mut tco_arg2, Value::Null)); args.push(std::mem::replace(&mut tco_arg3, Value::Null)); }
                        3 => { args.push(std::mem::replace(&mut tco_arg0, Value::Null)); args.push(std::mem::replace(&mut tco_arg1, Value::Null)); args.push(std::mem::replace(&mut tco_arg2, Value::Null)); }
                        2 => { args.push(std::mem::replace(&mut tco_arg0, Value::Null)); args.push(std::mem::replace(&mut tco_arg1, Value::Null)); }
                        1 => { args.push(std::mem::replace(&mut tco_arg0, Value::Null)); }
                        _ => {}
                    }
                    let result = try_or_catch!(func_val.call(&args, &[], self, module));
                    {
                        let frame = self.frames.last().unwrap();
                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                        locals_len = frame.arena_len;
                    }
                    hot_push!(result);
                }
                OP_MAKE_FUNCTION => {
                    let func_name = module.names[a].clone();
                    let (body_offset, param_names, param_indices, defaults, varargs_name, kwargs_name, is_gen, free_vars_info, declared_num_locals) =
                        if let Some(arabi_compiler::compiler::Value::String(info)) = hot_const!().get(c) {
                            let parts: Vec<&str> = info.split(',').collect();
                            let body_start: usize = parts[0].parse().unwrap_or(0);
                            let pnames_idx: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                            let pidx_idx: usize = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
                            let defaults_idx: usize = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0);
                            let varargs_idx: usize = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0);
                            let kwargs_idx: usize = parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0);
                            let is_gen: bool = parts.get(6).map(|s| *s == "1").unwrap_or(false);
                            let fv_idx: usize = parts.get(7).and_then(|s| s.parse().ok()).unwrap_or(0);
                            let decl_nl: usize = parts.get(8).and_then(|s| s.parse().ok()).unwrap_or(0);

                                let pnames = if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(pnames_idx) {
                                if s.is_empty() { Vec::new() } else { s.split(',').map(|s| s.to_string()).collect() }
                            } else { Vec::new() };

                                let pidxs = if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(pidx_idx) {
                                if s.is_empty() { Vec::new() } else { s.split(',').filter_map(|s| s.parse().ok()).collect() }
                            } else { Vec::new() };

                                let defs = if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(defaults_idx) {
                                if s.is_empty() {
                                    Vec::new()
                                } else {
                                    s.split('|').map(|entry| {
                                        if entry == "x" { return None; }
                                        let (typ, val) = entry.split_at(1);
                                        match typ {
                                            "i" => val.parse().ok().map(Value::Integer),
                                            "f" => val.parse().ok().map(Value::Float),
                                            "s" => Some(Value::String(Rc::new(val.to_string()))),
                                            "b" => Some(Value::Boolean(val == "true")),
                                            "n" => Some(Value::Null),
                                            _ => None,
                                        }
                                    }).collect()
                                }
                            } else { Vec::new() };

                            let varargs = if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(varargs_idx) {
                                if s.is_empty() { None } else { Some(s.clone()) }
                            } else { None };

                            let kwargs = if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(kwargs_idx) {
                                if s.is_empty() { None } else { Some(s.clone()) }
                            } else { None };

                            // Parse free variables info: "inner_idx:outer_idx|..."
                            let fv = if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(fv_idx) {
                                if s.is_empty() {
                                    Vec::new()
                                } else {
                                    s.split('|').filter_map(|entry| {
                                        let parts: Vec<&str> = entry.split(':').collect();
                                        let inner_idx: usize = parts.first()?.parse().ok()?;
                                        let outer_idx: usize = parts.get(1)?.parse().ok()?;
                                        Some((inner_idx, outer_idx))
                                    }).collect()
                                }
                            } else { Vec::new() };

                            (body_start, pnames, pidxs, defs, varargs, kwargs, is_gen, fv, decl_nl)
                        } else {
                            (0, Vec::new(), Vec::new(), Vec::new(), None, None, false, Vec::new(), 0)
                        };

                    // Capture free variables from the immediate parent frame (top of stack)
                    let mut closure = Vec::new();
                    let (parent_arena_offset, parent_arena_len) = match self.frames.last() {
                        Some(f) => (f.arena_offset, f.arena_len),
                        None => (0, 0),
                    };
                    for (inner_idx, outer_idx) in &free_vars_info {
                        if *outer_idx < parent_arena_len {
                            let slot_idx = parent_arena_offset + outer_idx;
                            // Check if outer already has a Cell — share it
                            let cell = match &self.locals_arena[slot_idx] {
                                Value::Cell(existing_cell) => existing_cell.clone(),
                                _ => {
                                    // Create a new Cell and replace the outer frame's local
                                    let new_cell = Rc::new(RefCell::new(self.locals_arena[slot_idx].clone()));
                                    self.locals_arena[slot_idx] = Value::Cell(new_cell.clone());
                                    new_cell
                                }
                            };
                            closure.push((*inner_idx, Value::Cell(cell)));
                        }
                    }

                    let func_num_locals = declared_num_locals.max(
                        param_indices.iter().copied().max().map_or(0, |m| m + 1)
                    ).max(
                        free_vars_info.iter().map(|(inner, _)| *inner).max().map_or(0, |m| m + 1)
                    ).max(
                        param_names.len()
                    );

                    let has_varargs = varargs_name.is_some();
                    let has_kwargs = kwargs_name.is_some();
                    let normal_param_count = param_names.len()
                        - if has_varargs { 1 } else { 0 }
                        - if has_kwargs { 1 } else { 0 };

                    let func = Value::Function(Rc::new(FunctionData {
                        name: func_name,
                        params: param_names,
                        param_indices,
                        defaults,
                        body: body_offset,
                        closure,
                        varargs_param: varargs_name,
                        kwargs_param: kwargs_name,
                        is_generator: is_gen,
                        module_index: None,
                        num_locals: func_num_locals,
                        normal_param_count,
                        call_count: Cell::new(0),
                        jit_entry: Cell::new(None),
                        jit_attempted: Cell::new(false),
                    }));
                    hot_push!(func);
                }

                OP_BUILD_LIST => {
                    if a as usize * 24 + self.memory_used > self.memory_limit {
                        runtime_error!("استثناء_ذاكرة", format!("تجاوزت الحد الاقصى للذاكرة ({} ميجابايت)", self.memory_limit / (1024 * 1024)));
                    }
                    self.memory_used += a as usize * 24;
                    let mut items = Vec::new();
                    for _ in 0..a {
                        items.push(hot_pop!());
                    }
                    items.reverse();
                    hot_push!(Value::List(SharedList::new(items)));
                }
                OP_BUILD_TUPLE => {
                    let mut items = Vec::new();
                    for _ in 0..a {
                        items.push(hot_pop!());
                    }
                    items.reverse();
                    hot_push!(Value::Tuple(Rc::new(items)));
                }
                OP_BUILD_DICT => {
                    let mut pairs = Vec::new();
                    for _ in 0..a {
                        let val = hot_pop!();
                        let key = hot_pop!();
                        pairs.push((key, val));
                    }
                    pairs.reverse();
                    hot_push!(Value::Dict(SharedDict::new(pairs)));
                }
                OP_DICT_SET_ITEM => {
                    let value = hot_pop!();
                    let key = hot_pop!();
                    match hot_pop!() {
                        Value::Dict(dict) => {
                            dict.borrow_mut().push((key, value));
                            hot_push!(Value::Dict(dict));
                        },
                        other => {
                            let type_name = match &other {
                                Value::Integer(_) => "عدد",
                                Value::Float(_) => "عشري",
                                Value::String(_) => "نص",
                                Value::Boolean(_) => "منطقي",
                                Value::Null => "عدم",
                                _ => "مجهول",
                            };
                            runtime_error!("استثناء_نوع", format!("لا يمكن ادراج العناصر في {}", type_name));
                        }
                    }
                }
                OP_BUILD_SET => {
                    let mut items = Vec::new();
                    for _ in 0..a {
                        items.push(hot_pop!());
                    }
                    items.reverse();
                    let mut unique = Vec::new();
                    for item in items {
                        if !unique.iter().any(|u| self.equals(u, &item)) {
                            unique.push(item);
                        }
                    }
                    hot_push!(Value::Set(SharedSet::new(unique)));
                }
                OP_SUBSCRIPT => {
                    let index = hot_pop!();
                    let obj = hot_pop!();
                    match (&obj, &index) {
                        (Value::List(items), Value::Integer(i)) => {
                            let borrow = items.borrow();
                            let idx = if *i < 0 { borrow.len() as i64 + i } else { *i } as usize;
                            if idx < borrow.len() {
                                hot_push!(borrow[idx].clone());
                            } else {
                                runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                            }
                        }
                        (Value::Tuple(items), Value::Integer(i)) => {
                            let len = items.len();
                            let idx = if *i < 0 { len as i64 + i } else { *i } as usize;
                            if idx < len {
                                hot_push!(items[idx].clone());
                            } else {
                                runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                            }
                        }
                        (Value::Range(d), Value::Integer(i)) => {
                            if d.step == 0 {
                                runtime_error!("استثناء_نطاق", "خطوة النطاق لا يمكن ان تكون صفر".to_string());
                            }
                            let actual_value = d.start + *i * d.step;
                            let in_range = if d.step > 0 {
                                actual_value < d.end
                            } else {
                                actual_value > d.end
                            };
                            if in_range {
                                hot_push!(Value::Integer(actual_value));
                            } else {
                                runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                            }
                        }
                        (Value::List(items), Value::Slice(d)) => {
                            let borrow = items.borrow();
                            let len = borrow.len() as i64;
                            let step_val = match d.step.as_ref() {
                                Value::Integer(i) => if *i == 0 { 1 } else { *i },
                                _ => 1,
                            };
                            let s = match d.start.as_ref() {
                                Value::Integer(i) => if *i < 0 { (len + i).max(0) } else { (*i).min(len - 1) },
                                Value::Null => if step_val > 0 { 0 } else { len - 1 },
                                _ => if step_val > 0 { 0 } else { len - 1 },
                            };
                            let e = match d.end.as_ref() {
                                Value::Integer(i) => if *i < 0 { (len + i).max(0) } else { (*i).min(len) },
                                Value::Null => if step_val > 0 { len } else { -1 },
                                _ => if step_val > 0 { len } else { -1 },
                            };
                            if step_val > 0 {
                                let mut result = Vec::new();
                                let mut i = s;
                                while i < e && i < len {
                                    result.push(borrow[i as usize].clone());
                                    i += step_val;
                                }
                                hot_push!(Value::List(SharedList::new(result)));
                            } else if step_val < 0 {
                                let mut result = Vec::new();
                                let mut i = s;
                                while i > e && i >= 0 {
                                    result.push(borrow[i as usize].clone());
                                    i += step_val;
                                }
                                hot_push!(Value::List(SharedList::new(result)));
                            } else {
                                hot_push!(Value::List(SharedList::new(Vec::new())));
                            }
                        }
                        (Value::String(s), Value::Integer(i)) => {
                            let chars: Vec<char> = s.chars().collect();
                            let len = chars.len() as i64;
                            let idx = if *i < 0 { len + i } else { *i } as usize;
                            if idx < chars.len() {
                                hot_push!(Value::String(Rc::new(chars[idx].to_string())));
                            } else {
                                runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                            }
                        }
                        (Value::String(s), Value::Slice(d)) => {
                            let chars: Vec<char> = s.chars().collect();
                            let len = chars.len() as i64;
                            let step_val = match d.step.as_ref() {
                                Value::Integer(i) => if *i == 0 { 1 } else { *i },
                                _ => 1,
                            };
                            let st = match d.start.as_ref() {
                                Value::Integer(i) => if *i < 0 { (len + i).max(0) } else { (*i).min(len - 1) },
                                Value::Null => if step_val > 0 { 0 } else { len - 1 },
                                _ => if step_val > 0 { 0 } else { len - 1 },
                            };
                            let en = match d.end.as_ref() {
                                Value::Integer(i) => if *i < 0 { (len + i).max(0) } else { (*i).min(len) },
                                Value::Null => if step_val > 0 { len } else { -1 },
                                _ => if step_val > 0 { len } else { -1 },
                            };
                            if step_val > 0 {
                                let mut result = String::new();
                                let mut i = st;
                                while i < en && i < len {
                                    result.push(chars[i as usize]);
                                    i += step_val;
                                }
                                hot_push!(Value::String(result.into()));
                            } else if step_val < 0 {
                                let mut result = String::new();
                                let mut i = st;
                                while i > en && i >= 0 {
                                    result.push(chars[i as usize]);
                                    i += step_val;
                                }
                                hot_push!(Value::String(result.into()));
                            } else {
                                hot_push!(Value::String(Rc::new(String::new())));
                            }
                        }
                        (Value::Dict(pairs), key) => {
                            if let Some(v) = pairs.lookup(&key) {
                                hot_push!(v);
                            } else {
                                runtime_error!("استثناء_مفتاح", "مفتاح غير موجود".to_string());
                            }
                        }
                        _ => runtime_error!("استثناء_نوع", "غير قادر على الوصول بالفهرس".to_string()),
                    }
                }
                OP_STORE_SUBSCRIPT => {
                    let value = hot_pop!();
                    let index = hot_pop!();
                    let mut obj = hot_pop!();
                    match (&mut obj, &index) {
                        (Value::List(items), Value::Integer(i)) => {
                            let mut borrow = items.borrow_mut();
                            let idx = if *i < 0 { borrow.len() as i64 + i } else { *i } as usize;
                            if idx < borrow.len() {
                                borrow[idx] = value;
                            } else {
                                runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                            }
                        }
                        (Value::Dict(pairs), _key) => {
                            pairs.insert(index, value);
                        }
                        _ => runtime_error!("استثناء_نوع", "غير قادر على التخزين بالفهرس".to_string()),
                    }
                    hot_push!(obj);
                }
                OP_BUILD_SLICE => {
                    let step = hot_pop!();
                    let end = hot_pop!();
                    let start = hot_pop!();
                    hot_push!(Value::Slice(Box::new(SliceData {
                        start: Box::new(start),
                        end: Box::new(end),
                        step: Box::new(step),
                    })));
                }
                OP_LIST_APPEND => {
                    let item = hot_pop!();
                    for _ in 1..b {
                        let existing = hot_pop!();
                        if let Value::List(ref items) = existing {
                            items.borrow_mut().push(item.clone());
                        }
                        hot_push!(existing);
                    }
                }
                OP_GET_ATTRIBUTE => {
                    let name_val = hot_pop!();
                    let obj = hot_pop!();
                    if let Value::String(name) = name_val {
                        match (&obj, name.as_str()) {
                            (Value::List(items), "طول") => {
                                hot_push!(Value::Integer(items.borrow().len() as i64));
                            }
                            (Value::Tuple(items), "طول") => {
                                hot_push!(Value::Integer(items.len() as i64));
                            }
                            (Value::Dict(pairs), "طول") => {
                                hot_push!(Value::Integer(pairs.borrow().len() as i64));
                            }
                            (Value::String(s), "طول") => {
                                hot_push!(Value::Integer(s.chars().count() as i64));
                            }
                            (Value::Range(d), "طول") => {
                                let len = if d.step > 0 {
                                    if d.end > d.start { (d.end - d.start + d.step - 1) / d.step } else { 0 }
                                } else if d.step < 0 {
                                    if d.start > d.end { (d.start - d.end + (-d.step) - 1) / (-d.step) } else { 0 }
                                } else {
                                    0
                                };
                                hot_push!(Value::Integer(len));
                            }
                            (Value::List(items), "عكس") => {
                                items.borrow_mut().reverse();
                                hot_push!(Value::List(items.clone()));
                            }
                            (Value::List(items), "رتب") => {
                                items.borrow_mut().sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                                hot_push!(Value::List(items.clone()));
                            }
                            (Value::List(_), "امسح") => {
                                hot_push!(Value::List(SharedList::new(Vec::new())));
                            }
                            (Value::Dict(pairs), "مفاتيح") => {
                                let keys: Vec<Value> = pairs.borrow().iter().map(|(k, _)| k.clone()).collect();
                                hot_push!(Value::List(SharedList::new(keys)));
                            }
                            (Value::Dict(pairs), "قيم") => {
                                let vals: Vec<Value> = pairs.borrow().iter().map(|(_, v)| v.clone()).collect();
                                hot_push!(Value::List(SharedList::new(vals)));
                            }
                            (Value::Dict(pairs), "احضر") => {
                                let key = hot_pop!();
                                if let Some(v) = pairs.lookup(&key) {
                                    hot_push!(v);
                                } else {
                                    hot_push!(Value::Null);
                                }
                            }
                            (Value::String(s), "استبدل") => {
                                let old = hot_pop!();
                                let new = hot_pop!();
                                let old_str = old.to_string_value();
                                let new_str = new.to_string_value();
                                hot_push!(Value::String(Rc::new(s.replace(&old_str, &new_str))));
                            }
                            (Value::String(s), "افصل") => {
                                let sep = hot_pop!();
                                let sep_str = sep.to_string_value();
                                let parts: Vec<Value> = s.split(&sep_str)
                                    .map(|p| Value::String(Rc::new(p.to_string())))
                                    .collect();
                                hot_push!(Value::List(SharedList::new(parts)));
                            }
                            (Value::String(s), "اوجد") => {
                                let sub = hot_pop!();
                                let sub_str = sub.to_string_value();
                                match s.find(&sub_str) {
                                    Some(pos) => hot_push!(Value::Integer(pos as i64)),
                                    None => hot_push!(Value::Integer(-1)),
                                }
                            }
                            (Value::String(s), "يمتلك") => {
                                let sub = hot_pop!();
                                let sub_str = sub.to_string_value();
                                hot_push!(Value::Boolean(s.contains(&sub_str)));
                            }
                            (Value::String(s), "يبدا") => {
                                let prefix = hot_pop!();
                                let prefix_str = prefix.to_string_value();
                                hot_push!(Value::Boolean(s.starts_with(&prefix_str)));
                            }
                            (Value::String(s), "ينتهي") => {
                                let suffix = hot_pop!();
                                let suffix_str = suffix.to_string_value();
                                hot_push!(Value::Boolean(s.ends_with(&suffix_str)));
                            }
                            (Value::String(s), "اكبر") => {
                                hot_push!(Value::String(Rc::new(s.to_uppercase())));
                            }
                            (Value::String(s), "اصغر") => {
                                hot_push!(Value::String(Rc::new(s.to_lowercase())));
                            }
                            (Value::String(_), "مسح") => {
                                hot_push!(Value::String(Rc::new(String::new())));
                            }
                            (Value::File(handle), "اكتب") => {
                                let content = hot_pop!().to_string_value();
                                let mut borrow = handle.0.borrow_mut();
                                if let Some(ref mut file) = *borrow {
                                    use std::io::Write;
                                    file.write_all(content.as_bytes()).map_err(|e| format!("خطا في الكتابة: {}", e))?;
                                }
                                hot_push!(Value::Null);
                            }
                            (Value::File(handle), "اقرا") => {
                                let mut borrow = handle.0.borrow_mut();
                                if let Some(ref mut file) = *borrow {
                                    use std::io::Read;
                                    let mut content = String::new();
                                    file.read_to_string(&mut content).map_err(|e| format!("خطا في القراءة: {}", e))?;
                                    hot_push!(Value::String(content.into()));
                                } else {
                                    hot_push!(Value::Null);
                                }
                            }
                            (Value::File(handle), "اقرا_سطر") => {
                                let mut borrow = handle.0.borrow_mut();
                                if let Some(ref mut file) = *borrow {
                                    use std::io::BufRead;
                                    let mut buf = String::new();
                                    let mut reader = std::io::BufReader::new(file);
                                    let n = reader.read_line(&mut buf).map_err(|e| format!("خطا في القراءة: {}", e))?;
                                    if n == 0 {
                                        hot_push!(Value::Null);
                                    } else {
                                        hot_push!(Value::String(buf.into()));
                                    }
                                } else {
                                    hot_push!(Value::Null);
                                }
                            }
                        (Value::File(handle), "اغلق") | (Value::File(handle), "__اترك__") => {
                                let file_opt = handle.0.borrow_mut().take();
                                drop(file_opt);
                                hot_push!(Value::Null);
                            }
                            (Value::Instance(_), _) => {
                                if let Some(attr) = obj.get_attribute(&name) {
                                    hot_push!(attr);
                                } else {
                                    return Err(format!("سمة غير موجودة: {}", name).into());
                                }
                            }
                            (Value::Class(d), _) => {
                                if let Some(attr) = obj.get_attribute(&name) {
                                    hot_push!(attr);
                                } else {
                                    return Err(format!("سمة غير موجودة: {} (في {})", name, d.name).into());
                                }
                            }
                            (Value::LazyModule(data), _) => {
                                let loaded = self.load_lazy_module(data)?;
                                let attr = loaded.get_attribute(&name)
                                    .ok_or_else(|| RuntimeError::new_typed("استثناء_اسم", format!("سمة غير موجودة: {} في الوحدة {}", name, data.name)))?;
                                hot_push!(attr);
                            }
                            _ => {
                                return Err(format!("سمة غير موجودة: {}", name).into());
                            }
                        }
                    } else {
                        return Err("اسم السمة يجب ان يكون نصاً".into());
                    }
                }
                // Inline Cache specialized attribute opcodes
                OP_GET_ATTR_IC_INSTANCE => {
                    let name_val = hot_pop!();
                    let obj = hot_pop!();
                    if let (Value::Instance(rc), Value::String(name)) = (&obj, &name_val) {
                        let fields_val = rc.get_field(&**name);
                        let methods_val = if fields_val.is_none() { rc.class.methods.get(&**name).cloned() } else { None };
                        if let Some(val) = fields_val {
                            hot_push!(val);
                        } else if let Some(val) = methods_val {
                            hot_push!(val);
                        } else {
                            runtime_error!("استثناء_اسم", format!("سمة غير موجودة: {}", name));
                        }
                    } else if let (Value::LazyModule(data), Value::String(name)) = (&obj, &name_val) {
                        let loaded = self.load_lazy_module(data)?;
                        let attr = loaded.get_attribute(name)
                            .ok_or_else(|| RuntimeError::new_typed("استثناء_اسم", format!("سمة غير موجودة: {} في الوحدة {}", name, data.name)))?;
                        hot_push!(attr);
                    } else if let Value::String(name) = &name_val {
                        // Type changed — deoptimize to full lookup
                        if let Some(attr) = obj.get_attribute(name) {
                            hot_push!(attr);
                        } else {
                            runtime_error!("استثناء_اسم", format!("سمة غير موجودة: {}", name));
                        }
                    } else {
                        runtime_error!("استثناء_اسم", "اسم السمة يجب ان يكون نصاً".to_string());
                    }
                }
                OP_GET_ATTR_IC_CLASS => {
                    let name_val = hot_pop!();
                    let obj = hot_pop!();
                    if let (Value::Class(d), Value::String(name)) = (&obj, &name_val) {
                        if let Some(val) = d.methods.get(&**name).cloned()
                            .or_else(|| d.fields.get(&**name).cloned())
                        {
                            hot_push!(val);
                        } else {
                            runtime_error!("استثناء_اسم", format!("سمة غير موجودة: {} (في {})", name, d.name));
                        }
                    } else if let (Value::LazyModule(data), Value::String(name)) = (&obj, &name_val) {
                        let loaded = self.load_lazy_module(data)?;
                        let attr = loaded.get_attribute(name)
                            .ok_or_else(|| RuntimeError::new_typed("استثناء_اسم", format!("سمة غير موجودة: {} في الوحدة {}", name, data.name)))?;
                        hot_push!(attr);
                    } else if let Value::String(name) = &name_val {
                        if let Some(attr) = obj.get_attribute(name) {
                            hot_push!(attr);
                        } else {
                            runtime_error!("استثناء_اسم", format!("سمة غير موجودة: {}", name));
                        }
                    } else {
                        runtime_error!("استثناء_اسم", "اسم السمة يجب ان يكون نصاً".to_string());
                    }
                }
                OP_MOD_JUMP_IF_NOT_ZERO => {
                    let local_a = ((pi >> 8) & 0xFF) as usize;
                    let local_b = ((pi >> 16) & 0xFFFF) as usize;
                    let target = (pi >> 32) as usize;
                    unsafe {
                        let a_val = &*locals_ptr.add(local_a);
                        let b_val = &*locals_ptr.add(local_b);
                        match (a_val, b_val) {
                            (Value::Integer(va), Value::Integer(vb)) => {
                                if *vb != 0 && *va % *vb != 0 { ip = target; }
                            }
                            (Value::Float(va), Value::Float(vb)) => {
                                if *vb != 0.0 {
                                    let r = va % vb;
                                    if r != 0.0 { ip = target; }
                                }
                            }
                            (Value::Integer(va), Value::Float(vb)) => {
                                if *vb != 0.0 {
                                    let r = (*va as f64) % vb;
                                    if r != 0.0 { ip = target; }
                                }
                            }
                            (Value::Float(va), Value::Integer(vb)) => {
                                if *vb != 0 {
                                    let r = va % (*vb as f64);
                                    if r != 0.0 { ip = target; }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                OP_MOD_ADD_IF_ZERO => {
                    let src_local = a;
                    let mod_idx = b as usize;
                    let target_local = c;
                    if src_local < locals_len && target_local < locals_len {
                        unsafe {
                            let src_val = &*locals_ptr.add(src_local);
                            let mod_val_raw = &hot_const!()[mod_idx];
                            let mod_val = self.convert_value(mod_val_raw);
                            match (src_val, &mod_val) {
                                (Value::Integer(s), Value::Integer(m)) => {
                                    if *m != 0 && *s % *m == 0 {
                                        let slot = &mut *locals_ptr.add(target_local);
                                        match slot {
                                            Value::Integer(n) => *n += 1,
                                            _ => {}
                                        }
                                    }
                                }
                                (Value::Integer(s), Value::Float(m)) => {
                                    if *m != 0.0 {
                                        let r = *s as f64 % *m;
                                        if r == 0.0 {
                                            let slot = &mut *locals_ptr.add(target_local);
                                            match slot {
                                                Value::Integer(n) => *n += 1,
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                (Value::Float(s), Value::Integer(m)) => {
                                    if *m != 0 {
                                        let r = *s % (*m as f64);
                                        if r == 0.0 {
                                            let slot = &mut *locals_ptr.add(target_local);
                                            match slot {
                                                Value::Integer(n) => *n += 1,
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                (Value::Float(s), Value::Float(m)) => {
                                    if *m != 0.0 {
                                        let r = *s % *m;
                                        if r == 0.0 {
                                            let slot = &mut *locals_ptr.add(target_local);
                                            match slot {
                                                Value::Integer(n) => *n += 1,
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                OP_SUBSCRIPT_LOCAL => {
                    let list_local = a;
                    let idx_local = b as usize;
                    unsafe {
                        if list_local < locals_len && idx_local < locals_len {
                            let list = &*locals_ptr.add(list_local);
                            let idx_val = &*locals_ptr.add(idx_local);
                            match (list, idx_val) {
                                (Value::List(items), Value::Integer(i)) => {
                                    let len = items.len();
                                    let idx = if *i < 0 { len as i64 + i } else { *i } as usize;
                                    if idx < len {
                                        hot_push!(items.get_unchecked(idx).clone());
                                    } else {
                                        runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                    }
                                }
                                (Value::Tuple(items), Value::Integer(i)) => {
                                    let len = items.len();
                                    let idx = if *i < 0 { len as i64 + i } else { *i } as usize;
                                    if idx < len {
                                        hot_push!(items[idx].clone());
                                    } else {
                                        runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                    }
                                }
                                (Value::Range(d), Value::Integer(i)) => {
                                    if d.step == 0 {
                                        runtime_error!("استثناء_نطاق", "خطوة النطاق لا يمكن ان تكون صفر".to_string());
                                    }
                                    let actual_value = d.start + *i * d.step;
                                    let in_range = if d.step > 0 { actual_value < d.end } else { actual_value > d.end };
                                    if in_range {
                                        hot_push!(Value::Integer(actual_value));
                                    } else {
                                        runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                    }
                                }
                                (Value::Dict(pairs), key) => {
                                    if let Some(v) = pairs.lookup(key) {
                                        hot_push!(v);
                                    } else {
                                        runtime_error!("استثناء_مفتاح", "مفتاح غير موجود".to_string());
                                    }
                                }
                                (Value::String(s), Value::Integer(i)) => {
                                    let chars: Vec<char> = s.chars().collect();
                                    let len = chars.len() as i64;
                                    let idx = if *i < 0 { len + i } else { *i } as usize;
                                    if idx < chars.len() {
                                        hot_push!(Value::String(Rc::new(chars[idx].to_string())));
                                    } else {
                                        runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                    }
                                }
                                _ => runtime_error!("استثناء_نوع", "غير قادر على الوصول بالفهرس".to_string()),
                            }
                        }
                    }
                }
                OP_SUBSCRIPT_LOCAL_2D => {
                    let list_local = a;
                    let idx1_local = b as usize;
                    let idx2_local = c;
                    unsafe {
                        if list_local < locals_len && idx1_local < locals_len && idx2_local < locals_len {
                            let list = &*locals_ptr.add(list_local);
                            let idx1_val = &*locals_ptr.add(idx1_local);
                            let idx2_val = &*locals_ptr.add(idx2_local);
                            if let (Value::List(items), Value::Integer(i1), Value::Integer(i2)) = (list, idx1_val, idx2_val) {
                                let len1 = items.len();
                                let idx1 = if *i1 < 0 { len1 as i64 + i1 } else { *i1 } as usize;
                                if idx1 < len1 {
                                    if let Value::List(inner) = items.get_unchecked(idx1) {
                                        let len2 = inner.len();
                                        let idx2 = if *i2 < 0 { len2 as i64 + i2 } else { *i2 } as usize;
                                        if idx2 < len2 {
                                            hot_push!(inner.get_unchecked(idx2).clone());
                                        } else {
                                            runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                        }
                                    } else {
                                        runtime_error!("استثناء_نوع", "العنصر ليس مصفوفة".to_string());
                                    }
                                } else {
                                    runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                }
                            } else {
                                runtime_error!("استثناء_نوع", "غير قادر على الوصول بالفهرس".to_string());
                            }
                        }
                    }
                }
                OP_STORE_SUBSCRIPT_LOCAL => {
                    let list_local = a;
                    let idx_local = b as usize;
                    let value = hot_pop!();
                    unsafe {
                        if list_local < locals_len && idx_local < locals_len {
                            let list = &mut *locals_ptr.add(list_local);
                            let idx_val = &*locals_ptr.add(idx_local);
                            if let (Value::List(items), Value::Integer(i)) = (list, idx_val) {
                                let len = items.len();
                                let idx = if *i < 0 { len as i64 + i } else { *i } as usize;
                                if idx < len {
                                    items.set_unchecked(idx, value);
                                } else {
                                    runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                }
                            } else {
                                runtime_error!("استثناء_نوع", "غير قادر على التخزين بالفهرس".to_string());
                            }
                        }
                    }
                }
                OP_STORE_SUBSCRIPT_LOCAL_2D => {
                    let list_local = a;
                    let idx1_local = b as usize;
                    let idx2_local = c;
                    let value = hot_pop!();
                    unsafe {
                        if list_local < locals_len && idx1_local < locals_len && idx2_local < locals_len {
                            let list = &*locals_ptr.add(list_local);
                            let idx1_val = &*locals_ptr.add(idx1_local);
                            let idx2_val = &*locals_ptr.add(idx2_local);
                            if let (Value::List(items), Value::Integer(i1), Value::Integer(i2)) = (list, idx1_val, idx2_val) {
                                let len1 = items.len();
                                let idx1 = if *i1 < 0 { len1 as i64 + i1 } else { *i1 } as usize;
                                if idx1 < len1 {
                                    if let Value::List(inner) = items.get_unchecked(idx1) {
                                        let len2 = inner.len();
                                        let idx2 = if *i2 < 0 { len2 as i64 + i2 } else { *i2 } as usize;
                                        if idx2 < len2 {
                                            inner.set_unchecked(idx2, value);
                                        } else {
                                            runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                        }
                                    } else {
                                        runtime_error!("استثناء_نوع", "العنصر ليس مصفوفة".to_string());
                                    }
                                } else {
                                    runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                }
                            } else {
                                runtime_error!("استثناء_نوع", "غير قادر على التخزين بالفهرس".to_string());
                            }
                        }
                    }
                }
                OP_SUBSCRIPT_ADD_IMM => {
                    let list_local = a;
                    let idx_local = c as usize;
                    let imm = ((pi >> 16) & 0xFFFF) as i16 as i64;
                    unsafe {
                        if list_local < locals_len && idx_local < locals_len {
                            let list = &*locals_ptr.add(list_local);
                            let idx_val = &*locals_ptr.add(idx_local);
                            if let (Value::List(items), Value::Integer(i)) = (list, idx_val) {
                                let len = items.len();
                                let idx = *i + imm;
                                let idx_u = if idx < 0 { len as i64 + idx } else { idx } as usize;
                                if idx_u < len {
                                    hot_push!(items.get_unchecked(idx_u).clone());
                                } else {
                                    runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                }
                            } else {
                                runtime_error!("استثناء_نوع", "غير قادر على الوصول بالفهرس".to_string());
                            }
                        }
                    }
                }
                OP_STORE_SUBSCRIPT_ADD_IMM => {
                    let list_local = a;
                    let idx_local = c as usize;
                    let imm = ((pi >> 16) & 0xFFFF) as i16 as i64;
                    let value = hot_pop!();
                    unsafe {
                        if list_local < locals_len && idx_local < locals_len {
                            let list = &*locals_ptr.add(list_local);
                            let idx_val = &*locals_ptr.add(idx_local);
                            if let (Value::List(items), Value::Integer(i)) = (list, idx_val) {
                                let len = items.len();
                                let idx = *i + imm;
                                let idx_u = if idx < 0 { len as i64 + idx } else { idx } as usize;
                                if idx_u < len {
                                    items.set_unchecked(idx_u, value);
                                } else {
                                    runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                }
                            } else {
                                runtime_error!("استثناء_نوع", "غير قادر على التخزين بالفهرس".to_string());
                            }
                        }
                    }
                }
                OP_ADD_TO_SUBSCRIPT_2D => {
                    let list_local = a;
                    let idx1_local = b as usize;
                    let idx2_local = (c >> 16) as usize;
                    let val_local = c & 0xFFFF;
                    unsafe {
                        if list_local < locals_len && idx1_local < locals_len && idx2_local < locals_len && val_local < locals_len {
                            let list = &*locals_ptr.add(list_local);
                            let idx1_val = &*locals_ptr.add(idx1_local);
                            let idx2_val = &*locals_ptr.add(idx2_local);
                            let val = &*locals_ptr.add(val_local);
                            if let (Value::List(items), Value::Integer(i1), Value::Integer(i2), addend) = (list, idx1_val, idx2_val, val) {
                                let len1 = items.len();
                                let idx1_u = if *i1 < 0 { len1 as i64 + i1 } else { *i1 } as usize;
                                if idx1_u < len1 {
                                    if let Value::List(inner) = items.get_unchecked(idx1_u) {
                                        let len2 = inner.len();
                                        let idx2_u = if *i2 < 0 { len2 as i64 + i2 } else { *i2 } as usize;
                                        if idx2_u < len2 {
                                            let old_val = inner.get_unchecked(idx2_u).clone();
                                            let new_val = match (&old_val, addend) {
                                                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
                                                (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                                                (Value::Integer(a), Value::Float(b)) => Value::Float(*a as f64 + b),
                                                (Value::Float(a), Value::Integer(b)) => Value::Float(a + *b as f64),
                                                _ => {
                                                    hot_push!(old_val);
                                                    hot_push!(addend.clone());
                                                    let r = hot_pop!();
                                                    let l = hot_pop!();
                                                    match (&l, &r) {
                                                        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
                                                        (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                                                        _ => Value::Null,
                                                    }
                                                }
                    };
                                            inner.set_unchecked(idx2_u, new_val);
                                        } else {
                                            runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                        }
                                    } else {
                                        runtime_error!("استثناء_نوع", "العنصر ليس مصفوفة".to_string());
                                    }
                                } else {
                                    runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                }
                            } else {
                                runtime_error!("استثناء_نوع", "غير قادر على الوصول بالفهرس".to_string());
                            }
                        }
                    }
                }
                OP_GET_ITER => {}
                OP_SWAP_ADJACENT => {
                    let list_local = a;
                    let idx_local = b as usize;
                    unsafe {
                        if list_local < locals_len && idx_local < locals_len {
                            let list = &*locals_ptr.add(list_local);
                            let idx_val = &*locals_ptr.add(idx_local);
                            if let (Value::List(items), Value::Integer(i)) = (list, idx_val) {
                                let len = items.len();
                                let idx = if *i < 0 { len as i64 + i } else { *i } as usize;
                                if idx + 1 < len {
                                    items.swap_unchecked(idx, idx + 1);
                                } else {
                                    runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                }
                            } else {
                                runtime_error!("استثناء_نوع", "غير قادر على التبديل".to_string());
                            }
                        }
                    }
                }
                OP_POP_JUMP_IF_SUBSCRIPT_GT => {
                    let list_local = a;
                    let idx_local = (b & 0xFF) as usize;
                    let imm = ((b >> 8) & 0xFF) as i16 as i64;
                    let target = c;
                    unsafe {
                        if list_local < locals_len && idx_local < locals_len {
                            let list = &*locals_ptr.add(list_local);
                            let idx_val = &*locals_ptr.add(idx_local);
                            if let (Value::List(items), Value::Integer(i)) = (list, idx_val) {
                                let len = items.len();
                                let idx1 = if *i < 0 { len as i64 + i } else { *i } as usize;
                                let idx2_val = *i + imm;
                                let idx2 = if idx2_val < 0 { len as i64 + idx2_val } else { idx2_val } as usize;
                                if idx1 < len && idx2 < len {
                                    let val1 = items.get_unchecked(idx1);
                                    let val2 = items.get_unchecked(idx2);
                                    let should_jump = match (val1, val2) {
                                        (Value::Integer(a), Value::Integer(b)) => a <= b,
                                        (Value::Float(a), Value::Float(b)) => a <= b,
                                        _ => !self.greater_than(val1, val2),
                                    };
                                    if should_jump { ip = target; }
                                } else {
                                    runtime_error!("استثناء_نطاق", "فهرس خارج النطاق".to_string());
                                }
                            } else {
                                runtime_error!("استثناء_نوع", "غير قادر على الوصول بالفهرس".to_string());
                            }
                        }
                    }
                }
                OP_FLOAT_ADD_MUL_LOCAL => {
                    let dst = a;
                    let src1 = b as usize;
                    let src2 = c as usize;
                    if dst < locals_len && src1 < locals_len && src2 < locals_len {
                        unsafe {
                            let v1 = &*locals_ptr.add(src1);
                            let v2 = &*locals_ptr.add(src2);
                            let slot = &mut *locals_ptr.add(dst);
                            match (v1, v2, slot) {
                                (Value::Float(a), Value::Float(b), Value::Float(s)) => {
                                    *s += *a * *b;
                                }
                                (Value::Integer(a), Value::Integer(b), Value::Float(s)) => {
                                    *s += *a as f64 * *b as f64;
                                }
                                (Value::Float(a), Value::Integer(b), Value::Float(s)) => {
                                    *s += *a * *b as f64;
                                }
                                (Value::Integer(a), Value::Float(b), Value::Float(s)) => {
                                    *s += *a as f64 * *b;
                                }
                                (Value::Integer(a), Value::Integer(b), Value::Integer(s)) => {
                                    *s += *a * *b;
                                }
                                (Value::Float(a), Value::Float(b), Value::Integer(s)) => {
                                    *s += (*a * *b) as i64;
                                }
                                (Value::Integer(a), Value::Float(b), Value::Integer(s)) => {
                                    *s += (*a as f64 * *b) as i64;
                                }
                                (Value::Float(a), Value::Integer(b), Value::Integer(s)) => {
                                    *s += (*a * *b as f64) as i64;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                OP_FLOAT_ADD_MUL_IMM => {
                    let dst = a;
                    let src = b as usize;
                    let const_idx = c as usize;
                    if dst < locals_len && src < locals_len {
                        unsafe {
                            let v = &*locals_ptr.add(src);
                            let c_raw = &hot_const!()[const_idx];
                            let c_val = self.convert_value(c_raw);
                            let slot = &mut *locals_ptr.add(dst);
                            match (v, &c_val, slot) {
                                (Value::Float(a), Value::Float(c), Value::Float(s)) => {
                                    *s += *a * *c;
                                }
                                (Value::Integer(a), Value::Float(c), Value::Float(s)) => {
                                    *s += *a as f64 * *c;
                                }
                                (Value::Float(a), Value::Integer(c), Value::Float(s)) => {
                                    *s += *a * *c as f64;
                                }
                                (Value::Integer(a), Value::Integer(c), Value::Float(s)) => {
                                    *s += *a as f64 * *c as f64;
                                }
                                (Value::Integer(a), Value::Integer(c), Value::Integer(s)) => {
                                    *s += *a * *c;
                                }
                                (Value::Float(a), Value::Float(c), Value::Integer(s)) => {
                                    *s += (*a * *c) as i64;
                                }
                                (Value::Integer(a), Value::Float(c), Value::Integer(s)) => {
                                    *s += (*a as f64 * *c) as i64;
                                }
                                (Value::Float(a), Value::Integer(c), Value::Integer(s)) => {
                                    *s += (*a * *c as f64) as i64;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                OP_NEG_FLOOR_DIV_SQR_ADD_IMM => {
                    let dst = a;
                    let src = b as usize;
                    let const_idx = c as usize;
                    if dst < locals_len && src < locals_len {
                        unsafe {
                            let v = &*locals_ptr.add(src);
                            let c_raw = &hot_const!()[const_idx];
                            let c_val = self.convert_value(c_raw);
                            let slot = &mut *locals_ptr.add(dst);
                            match (v, &c_val) {
                                (Value::Float(x), Value::Float(eps)) => {
                                    let denom = *x * *x + *eps;
                                    if denom != 0.0 {
                                        let result = (-1.0_f64 / denom).floor() as i64;
                                        *slot = Value::Integer(result);
                                    }
                                }
                                (Value::Integer(x), Value::Float(eps)) => {
                                    let xf = *x as f64;
                                    let denom = xf * xf + *eps;
                                    if denom != 0.0 {
                                        let result = (-1.0_f64 / denom).floor() as i64;
                                        *slot = Value::Integer(result);
                                    }
                                }
                                (Value::Float(x), Value::Integer(eps)) => {
                                    let denom = *x * *x + *eps as f64;
                                    if denom != 0.0 {
                                        let result = (-1.0_f64 / denom).floor() as i64;
                                        *slot = Value::Integer(result);
                                    }
                                }
                                (Value::Integer(x), Value::Integer(eps)) => {
                                    let xf = *x as f64;
                                    let denom = xf * xf + *eps as f64;
                                    if denom != 0.0 {
                                        let result = (-1.0_f64 / denom).floor() as i64;
                                        *slot = Value::Integer(result);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                OP_POP_JUMP_IF_LT_LOCAL => {
                    let local_a = a;
                    let local_b = b as usize;
                    let target = c;
                    if local_a < locals_len && local_b < locals_len {
                        unsafe {
                            let va = &*locals_ptr.add(local_a);
                            let vb = &*locals_ptr.add(local_b);
                            let should_jump = match (va, vb) {
                                (Value::Integer(a), Value::Integer(b)) => !(*a < *b),
                                (Value::Float(a), Value::Float(b)) => !(*a < *b),
                                (Value::Integer(a), Value::Float(b)) => !((*a as f64) < *b),
                                (Value::Float(a), Value::Integer(b)) => !(*a < (*b as f64)),
                                _ => !self.less_than(va, vb),
                            };
                            if should_jump { ip = target; }
                        }
                    }
                }
                OP_FLOAT_SQR_SUB_ADD_IMM => {
                    let dst = a;
                    let b_local = b as usize;
                    let cd = c;
                    let c_local = (cd >> 16) as usize;
                    let d_local = cd & 0xFFFF;
                    if dst < locals_len && b_local < locals_len && c_local < locals_len && d_local < locals_len {
                        unsafe {
                            let vb = &*locals_ptr.add(b_local);
                            let vc = &*locals_ptr.add(c_local);
                            let vd = &*locals_ptr.add(d_local);
                            let slot = &mut *locals_ptr.add(dst);
                            let bf = match vb { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 0.0 };
                            let cf = match vc { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 0.0 };
                            let df = match vd { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 0.0 };
                            *slot = Value::Float(bf * bf - cf * cf + df);
                        }
                    }
                }
                OP_FLOAT_MUL_MUL_ADD_IMM => {
                    let dst = a;
                    let b_local = b as usize;
                    let cd = c;
                    let c_local = (cd >> 16) as usize;
                    let d_local = cd & 0xFFFF;
                    if dst < locals_len && b_local < locals_len && c_local < locals_len && d_local < locals_len {
                        unsafe {
                            let vb = &*locals_ptr.add(b_local);
                            let vc = &*locals_ptr.add(c_local);
                            let vd = &*locals_ptr.add(d_local);
                            let slot = &mut *locals_ptr.add(dst);
                            let bf = match vb { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 0.0 };
                            let cf = match vc { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 0.0 };
                            let df = match vd { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 0.0 };
                            *slot = Value::Float(2.0 * bf * cf + df);
                        }
                    }
                }
                OP_POP_JUMP_IF_NOT_SQR_ADD_SQR_GT_IMM => {
                    let local_a = a;
                    let local_b = b as usize;
                    let const_idx = (c >> 16) as usize;
                    let target = c & 0xFFFF;
                    if local_a < locals_len && local_b < locals_len {
                        unsafe {
                            let va = &*locals_ptr.add(local_a);
                            let vb = &*locals_ptr.add(local_b);
                            let af = match va { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 0.0 };
                            let bf = match vb { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 0.0 };
                            let c_raw = &hot_const!()[const_idx];
                            let c_val = self.convert_value(c_raw);
                            let threshold = match &c_val { Value::Float(x) => *x, Value::Integer(x) => *x as f64, _ => 4.0 };
                            if af * af + bf * bf <= threshold {
                                ip = target;
                            }
                        }
                    }
                }
                OP_LIST_APPEND_LOCAL => {
                    if self.memory_used + 24 > self.memory_limit {
                        runtime_error!("استثناء_ذاكرة", format!("تجاوزت الحد الاقصى للذاكرة ({} ميجابايت)", self.memory_limit / (1024 * 1024)));
                    }
                    self.memory_used += 24;
                    let list_local = a;
                    let val = hot_pop!();
                    if list_local < locals_len {
                        unsafe {
                            let list = &mut *locals_ptr.add(list_local);
                            if let Value::List(items) = list {
                                if Rc::strong_count(&items.0) == 1 {
                                    if let Some(inner) = Rc::get_mut(&mut items.0) {
                                        inner.get_mut().push(val);
                                    }
                                } else {
                                    items.borrow_mut().push(val);
                                }
                            }
                        }
                    }
                }
                OP_ADD_LOCAL_FROM_STACK => {
                    let dest_local = a;
                    let val = hot_pop!();
                    if dest_local < locals_len {
                        unsafe {
                            let slot = &mut *locals_ptr.add(dest_local);
                            let new_val = match (&*slot, &val) {
                                (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
                                (Value::Float(a), Value::Float(b)) => Value::Float(a + b),
                                (Value::Integer(a), Value::Float(b)) => Value::Float(*a as f64 + b),
                                (Value::Float(a), Value::Integer(b)) => Value::Float(a + *b as f64),
                                _ => {
                                    let old = std::mem::replace(&mut *slot, Value::Null);
                                    self.add(old, val).unwrap_or(Value::Integer(0))
                                }
                            };
                            *slot = new_val;
                        }
                    }
                }
                OP_GET_INSTANCE_FIELD => {
                    let name_idx = c;
                    let obj = hot_pop!();
                    if let Value::Instance(rc) = &obj {
                        match &module.constants[name_idx] {
                            arabi_compiler::compiler::Value::Integer(offset) => {
                                let idx = *offset as usize;
                                if idx < rc.num_inline_fields as usize {
                                    hot_push!(unsafe { (*rc.inline_fields.as_ptr())[idx].clone() });
                                    continue;
                                }
                                if let Some(name) = rc.class.field_names.get(idx) {
                                    if let Some(val) = rc.extra_fields.borrow().as_ref().and_then(|f| f.get(name.as_str()).cloned()) {
                                        hot_push!(val);
                                        continue;
                                    }
                                }
                                runtime_error!("استثناء_اسم", format!("حقل غير موجود: {}", offset));
                            }
                            arabi_compiler::compiler::Value::String(name_str) => {
                                if let Some(val) = rc.get_field(name_str.as_str()) {
                                    hot_push!(val);
                                    continue;
                                }
                                if let Some(val) = rc.class.methods.get(name_str.as_str()).cloned() {
                                    hot_push!(val);
                                    continue;
                                } else {
                                    runtime_error!("استثناء_اسم", format!("سمة غير موجودة: {}", name_str));
                                }
                            }
                            _ => {}
                        }
                    }
                    runtime_error!("استثناء_نوع", "الوصول الميداني يتطلب مثياً".to_string());
                }
                OP_SET_INSTANCE_FIELD => {
                    let name_idx = c;
                    let val = hot_pop!();
                    let obj = hot_pop!();
                    if let Value::Instance(rc) = &obj {
                        match &module.constants[name_idx] {
                            arabi_compiler::compiler::Value::Integer(offset) => {
                                let idx = *offset as usize;
                                if idx < rc.num_inline_fields as usize {
                                    rc.inline_fields.borrow_mut()[idx] = val;
                                } else if let Some(name) = rc.class.field_names.get(idx) {
                                    rc.extra_fields.borrow_mut().get_or_insert_with(|| Box::new(HashMap::new())).insert(name.clone(), val);
                                }
                            }
                            arabi_compiler::compiler::Value::String(name_str) => {
                                rc.set_field(name_str.clone(), val);
                            }
                            _ => {}
                        }
                    } else {
                        runtime_error!("استثناء_نوع", "تعيين الميدان يتطلب مثياً".to_string());
                    }
                }
                OP_FOR_ITER => {
                    let obj = hot_pop!();
                    let items: Vec<Value> = match obj {
                        Value::List(l) => l.borrow().clone(),
                        Value::Tuple(l) => l.as_ref().clone(),
                        Value::String(s) => s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect(),
                        Value::Generator(d) => {
                            let gen = d.clone();
                            let body;
                            let gen_locals;
                            {
                                let data = gen.borrow();
                                body = data.body;
                                gen_locals = data.locals.clone();
                            }
                            // Execute generator body and collect yielded values
                            let mut gen_items = Vec::new();
                            let num_gen_locals = gen_locals.len();
                            let gen_arena_offset = self.locals_arena.len();
                            self.locals_arena.resize(gen_arena_offset + num_gen_locals, Value::Null);
                            for (i, val) in gen_locals.into_iter().enumerate() {
                                self.locals_arena[gen_arena_offset + i] = val;
                            }
                            self.push_frame(gen_arena_offset, num_gen_locals, body)?;
                            loop {
                                let frame_ip_before = self.current_frame().return_ip;
                                match self.run_frame(module) {
                                    Ok(val) => {
                                        let frame_ip_after = self.current_frame().return_ip;
                                        if frame_ip_after == frame_ip_before && matches!(val, Value::Null) {
                                            break;
                                        }
                                        gen_items.push(val);
                                    }
                                    Err(_) => break,
                                }
                            }
                            self.pop_frame();
                            gen_items
                        }
                        Value::Dict(d) => {
                            let borrow = d.borrow();
                            borrow.iter().map(|(k, v)| Value::Tuple(Rc::new(vec![k.clone(), v.clone()]))).collect()
                        }
                        Value::Range(d) => {
                            let mut items = Vec::new();
                            if d.step > 0 {
                                let mut i = d.start;
                                while i < d.end {
                                    items.push(Value::Integer(i));
                                    i += d.step;
                                }
                            } else if d.step < 0 {
                                let mut i = d.start;
                                while i > d.end {
                                    items.push(Value::Integer(i));
                                    i += d.step;
                                }
                            }
                            items
                        }
                        Value::Instance(rc) => {
                            let begin_method_clone = rc.class.methods.get("__بادئ__").cloned();
                            if let Some(begin_method) = begin_method_clone {
                                let iterator = match begin_method.call(std::slice::from_ref(&Value::Instance(rc.clone())), &[], self, module) {
                                    Ok(v) => v,
                                    Err(e) => return Err(e.to_string().into()),
                                };
                                let mut items = Vec::new();
                                if let Value::Instance(rc) = &iterator {
                                    loop {
let next_method_clone = rc.class.methods.get("__التالي__").cloned();
                                        match next_method_clone {
                                            Some(next_method) => {
                                                match next_method.call(std::slice::from_ref(&iterator), &[], self, module) {
                                                    Ok(val) => {
                                                        if matches!(val, Value::Null) { break; }
                                                        items.push(val);
                                                    }
                                                    Err(_) => break,
                                                }
                                            }
                                            None => break,
                                        }
                                    }
                                }
                                items
                            } else {
                                let next_method_clone = rc.class.methods.get("__التالي__").cloned();
                                if let Some(next_method) = next_method_clone {
                                    // If no __بادئ__, treat the instance itself as iterator
                                    let mut items = Vec::new();
                                    let obj_clone = Value::Instance(rc.clone());
                                    while let Ok(val) = next_method.call(std::slice::from_ref(&obj_clone), &[], self, module) {
                                        if matches!(val, Value::Null) { break; }
                                        items.push(val);
                                    }
                                    items
                                } else {
                                    return Err("غير قادر على التكرار".into());
                                }
                            }
                        }
                        _ => return Err("غير قادر على التكرار".into()),
                    };
                    // Re-acquire locals pointers after Generator/Instance ForIter
                    {
                        let frame = self.frames.last().unwrap();
                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                        locals_len = frame.arena_len;
                    }
                    let len = items.len();
                    hot_push!(Value::List(SharedList::new(items)));
                    hot_push!(Value::Integer(0));
                    hot_push!(Value::Integer(len as i64));
                }

                OP_STRING_COERCE => {
                    let val = hot_pop!();
                    let mut coerced = false;
if let Value::Instance(rc) = &val {
                        let method_clone = rc.class.methods.get("__نص__").cloned();
                        if let Some(method) = method_clone {
                            if let Ok(result) = method.call(std::slice::from_ref(&val), &[], self, module) {
                                {
                                    let frame = self.frames.last().unwrap();
                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                    locals_len = frame.arena_len;
                                }
                                hot_push!(Value::String(result.to_string_value().into()));
                                coerced = true;
                            }
                        }
                        if !coerced {
                            let method_clone = rc.class.methods.get("__عرض__").cloned();
                            if let Some(method) = method_clone {
                                if let Ok(result) = method.call(std::slice::from_ref(&val), &[], self, module) {
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                    hot_push!(Value::String(result.to_string_value().into()));
                                    coerced = true;
                                }
                            }
                        }
                    }
                    if !coerced {
                        hot_push!(Value::String(val.to_string_value().into()));
                    }
                }
                OP_STRING_FORMAT => {
                    let val = hot_pop!();
                    let fmt = hot_pop!();
                    let fmt_str = fmt.to_string_value();
                    let result = apply_format(&val, &fmt_str);
                    hot_push!(Value::String(result.into()));
                }
                OP_YIELD_VALUE => {
                    let value = hot_pop!();
                    // Save current IP into the frame so next run_frame resumes here
                    if let Some(frame) = self.frames.last_mut() {
                        frame.return_ip = ip;
                    }
                    return Ok(value);
                }

                OP_CREATE_CLASS => {
                    let class_name_str = &module.names[a];
                    let methods_str = if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(b) {
                        s.clone()
                    } else {
                        String::new()
                    };
                    let method_names: Vec<&str> = methods_str.split(',').filter(|s| !s.is_empty()).collect();
                    let method_count_val = c & 0xFFFF;
                    let parent_count_val = (c >> 16) & 0xFFFF;

                    let mut method_values: Vec<Value> = Vec::new();
                    for _ in 0..method_count_val {
                        method_values.push(hot_pop!());
                    }
                    method_values.reverse();

                    let mut parent_methods: HashMap<String, Value> = HashMap::new();
                    let mut parent_names = Vec::new();
                    for _ in 0..parent_count_val {
                        if let Value::Class(d) = hot_pop!() {
                            parent_methods.extend(d.methods.iter().map(|(k, v)| (k.clone(), v.clone())));
                            parent_names.push(d.name.to_string());
                        }
                    }

                    let mut methods = parent_methods;
                    for (i, mval) in method_values.into_iter().enumerate() {
                        let mname = method_names.get(i).unwrap_or(&"").to_string();
                        methods.insert(mname, mval);
                    }

                    // Analyze __تهيئة__ to detect simple constructor pattern for inlining
                    let field_names = if let Some(Value::Function(f)) = methods.get("__تهيئة__") {
                        let body = f.body;
                        let packed = &module.packed;
                        let mut detected_fields = Vec::new();
                        let mut ok = true;
                        let mut ip = body;
                        loop {
                            if ip >= packed.len() { ok = false; break; }
                            let pi = packed[ip];
                            let op = pi as u8;
                            match op {
                                OP_LOAD_FAST => {
                                    let local_a = ((pi >> 8) & 0xFF) as usize;
                                    if local_a == 0 {
                                        ip += 1;
                                        if ip >= packed.len() { ok = false; break; }
                                        let pi2 = packed[ip];
                                        let op2 = pi2 as u8;
                                        let local_a2 = ((pi2 >> 8) & 0xFF) as usize;
                                        if op2 == OP_LOAD_FAST && local_a2 > 0 {
                                            ip += 1;
                                            if ip >= packed.len() { ok = false; break; }
                                            let pi3 = packed[ip];
                                            let op3 = pi3 as u8;
                                            if op3 == OP_STORE_ATTR {
                                                let name_idx = ((pi3 >> 16) & 0xFFFF) as usize;
                                                let field_name = module.names[name_idx].clone();
                                                detected_fields.push(field_name);
                                                ip += 1;
                                                continue;
                                            }
                                        }
                                    }
                                    ok = false;
                                    break;
                                }
                                OP_LOAD_NONE | OP_RETURN_VALUE => {
                                    break;
                                }
                                _ => {
                                    ok = false;
                                    break;
                                }
                            }
                        }
                        if ok && !detected_fields.is_empty() && detected_fields.len() == f.params.len().saturating_sub(1) {
                            detected_fields
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    };

                    // Analyze methods for fast inlining: return this.field1 + this.field2
                    let mut fast_methods_to_insert = Vec::new();
                    for (mname, mval) in methods.iter() {
                        if mname == "__تهيئة__" { continue; }
                        if let Value::Function(f) = mval {
                            let body = f.body;
                            let packed = &module.packed;
                            let mut mip = body;
                            let mut ok = true;
                            let mut fields: Vec<String> = Vec::new();
                            let mut has_op = false;
                            let mut op_code: u8 = 0;
                            loop {
                                if mip >= packed.len() { ok = false; break; }
                                let pi = packed[mip];
                                let op = pi as u8;
                                match op {
                                    OP_LOAD_FAST => {
                                        let la = ((pi >> 8) & 0xFF) as usize;
                                        if la == 0 {
                                            mip += 1;
                                            if mip >= packed.len() { ok = false; break; }
                                            let pi2 = packed[mip];
                                            let op2 = pi2 as u8;
                                            if op2 == OP_GET_INSTANCE_FIELD {
                                                let c2 = (pi2 >> 32) as usize;
                                                if let arabi_compiler::compiler::Value::String(fname) = &module.constants[c2] {
                                                    fields.push(fname.clone());
                                                    mip += 1;
                                                    continue;
                                                }
                                            } else if op2 == OP_GET_ATTRIBUTE {
                                                mip += 1;
                                                if mip >= packed.len() { ok = false; break; }
                                                let pi3 = packed[mip];
                                                let op3 = pi3 as u8;
                                                if op3 == OP_GET_ATTRIBUTE {
                                                    mip += 1;
                                                    continue;
                                                }
                                            }
                                        }
                                        ok = false;
                                        break;
                                    }
                                    OP_BINARY_ADD | OP_BINARY_SUBTRACT | OP_BINARY_MULTIPLY | OP_BINARY_DIVIDE => {
                                        op_code = op;
                                        has_op = true;
                                        mip += 1;
                                        continue;
                                    }
                                    OP_LOAD_NONE | OP_RETURN_VALUE => { break; }
                                    _ => { ok = false; break; }
                                }
                            }
                            if ok && has_op && fields.len() == 2 {
                                let fast_op = match op_code {
                                    OP_BINARY_ADD => FastMethodOp::Add,
                                    OP_BINARY_SUBTRACT => FastMethodOp::Sub,
                                    OP_BINARY_MULTIPLY => FastMethodOp::Mul,
                                    OP_BINARY_DIVIDE => FastMethodOp::Div,
                                    _ => FastMethodOp::Add,
                                };
                                let fast = FastMethodData {
                                    name: mname.clone(),
                                    field1: fields[0].clone(),
                                    field2: fields[1].clone(),
                                    field1_idx: field_names.iter().position(|n| n == &fields[0]).unwrap_or(0),
                                    field2_idx: field_names.iter().position(|n| n == &fields[1]).unwrap_or(0),
                                    op: fast_op,
                                };
                                fast_methods_to_insert.push((mname.clone(), Value::FastMethod(Rc::new(fast))));
                            }
                        }
                    }
                    for (mname, fval) in fast_methods_to_insert {
                        methods.insert(mname, fval);
                    }

                    let mut fi = HashMap::new();
                    for (i, n) in field_names.iter().enumerate() {
                        fi.insert(n.clone(), i);
                    }

                    hot_push!(Value::Class(Rc::new(ClassData {
                        name: Rc::from(class_name_str.as_str()),
                        methods: Rc::new(methods),
                        fields: HashMap::new(),
                        parents: parent_names,
                        field_names,
                        field_index: Rc::new(fi),
                    })));
                }
                OP_CREATE_INSTANCE => {
                    let class_name_str = &module.names[a];
                    let class_val = self.globals.get(class_name_str).cloned();
                    match class_val {
                        Some(Value::Class(d)) => {
                            let nf = d.field_names.len().min(4) as u8;
                let instance = Value::Instance(Rc::new(InstanceData {
                                class: Rc::clone(&d),
                                inline_fields: RefCell::new(core::array::from_fn(|_| Value::Null)),
                                num_inline_fields: nf,
                                extra_fields: RefCell::new(None),
                }));
                            hot_push!(instance);
                        }
                        _ => runtime_error!("استثناء_اسم", format!("الصنف {} غير موجود", class_name_str)),
                    }
                }
                OP_CALL_METHOD => {
                    let method_name_str = &module.names[a];
                    // Stack-based args: avoid heap allocation for method calls
                    let args_len = b.min(4);
                    let mut args_buf: [Value; 4] = [Value::Null, Value::Null, Value::Null, Value::Null];
                    for i in 0..args_len {
                        args_buf[i] = hot_pop!();
                    }
                    args_buf[..args_len].reverse();
                    let obj = hot_pop!();
                    let args = &args_buf[..args_len];
                    // FAST PATH: String methods — skip Instance check + 50+ match arms
                    if let Value::String(s) = &obj {
                        match method_name_str.as_str() {
                            "اكبر" => { hot_push!(Value::String(Rc::new(s.to_uppercase()))); continue; }
                            "اصغر" => { hot_push!(Value::String(Rc::new(s.to_lowercase()))); continue; }
                            "شطب" => { hot_push!(Value::String(s.trim().to_string().into())); continue; }
                            "طول" => { hot_push!(Value::Integer(s.chars().count() as i64)); continue; }
                            "يحتوي" => {
                                let pat = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                                hot_push!(Value::Boolean(s.contains(&pat))); continue;
                            }
                            "تبدا_بـ" => {
                                let prefix = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                                hot_push!(Value::Boolean(s.starts_with(&prefix))); continue;
                            }
                            "تنتهي_بـ" => {
                                let suffix = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                                hot_push!(Value::Boolean(s.ends_with(&suffix))); continue;
                            }
                            "استبدل" => {
                                if args.len() >= 2 {
                                    let from = args[0].to_string_value();
                                    let to = args[1].to_string_value();
                                    let result = s.replace(&from, &to);
                                    hot_push!(Value::String(result.into()));
                                } else {
                                    hot_push!(Value::String(s.clone()));
                                }
                                continue;
                            }
                            "اقسم" => {
                                let sep = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                                let parts: Vec<Value> = s.split(&*sep).map(|p| Value::String(Rc::new(p.to_string()))).collect();
                                hot_push!(Value::List(SharedList::new(parts)));
                                continue;
                            }
                            "تحقق_من_الحرف" => {
                                hot_push!(Value::Boolean(!s.is_empty() && s.chars().all(|c| c.is_alphabetic())));
                                continue;
                            }
                            "تحقق_من_الرقم" => {
                                hot_push!(Value::Boolean(!s.is_empty() && s.chars().all(|c| c.is_ascii_digit())));
                                continue;
                            }
                            _ => {} // fall through to normal dispatch
                        }
                    }
                    // FAST PATH: Instance custom method — skip all 40+ builtin checks
                    if let Value::Instance(rc) = &obj {
                        let name_hash = {
                            let mut h: u64 = 0xcbf29ce484222325;
                            for b in method_name_str.as_bytes() {
                                h ^= *b as u64;
                                h = h.wrapping_mul(0x100000001b3);
                            }
                            h
                        };
                        // DIRECT ACCESS: no outer RefCell borrow needed
                        let methods_ptr = Rc::as_ptr(&rc.class.methods);
                        let cache_hit = methods_ptr == self.mc_methods_ptr && name_hash == self.mc_method;
                        if cache_hit {
                            match &self.mc_value {
                                Value::FastMethod(fm) => {
                                    let (v1, v2) = (rc.get_field(fm.field1.as_str()).unwrap_or(Value::Null),
                                                     rc.get_field(fm.field2.as_str()).unwrap_or(Value::Null));
                                    let result = match (&v1, &v2, fm.op) {
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Add) => Value::Integer(a + b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Add) => Value::Float(a + b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Add) => Value::Float(*a as f64 + b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Add) => Value::Float(a + *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Sub) => Value::Integer(a - b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Sub) => Value::Float(a - b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Sub) => Value::Float(*a as f64 - b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Sub) => Value::Float(a - *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Mul) => Value::Integer(a * b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Mul) => Value::Float(a * b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Mul) => Value::Float(*a as f64 * b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Mul) => Value::Float(a * *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Div) => Value::Integer(a / b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Div) => Value::Float(a / b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Div) => Value::Float(*a as f64 / b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Div) => Value::Float(a / *b as f64),
                                        _ => Value::Null,
                                    };
                                    hot_push!(result);
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                    continue;
                                }
                                other => {
                                    let method_val = other.clone();
                                    match method_val {
                                        Value::Function(f) => {
                                            let param_indices = f.param_indices.clone();
                                            let body = f.body;
                                            let n_locals = f.num_locals.max(1);
                                             let class_name_str_rc = rc.class.name.clone();
                                             let arena_offset = self.locals_arena.len();
                                            self.locals_arena.resize(arena_offset + n_locals, Value::Null);
                                            let local_vars_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                                            let this_val = obj.clone();
                                            if let Some(&idx) = param_indices.first() {
                                                unsafe { *local_vars_ptr.add(idx) = this_val; }
                                            }
                                            for (i, arg) in args.iter().enumerate() {
                                                let local_idx = param_indices.get(i + 1).copied().unwrap_or(i + 1);
                                                if local_idx < n_locals {
                                                    unsafe { *local_vars_ptr.add(local_idx) = arg.clone(); }
                                                }
                                            }
                                            let prev_class = self.current_class_name.clone();
                                            self.current_class_name = Some(class_name_str_rc.clone());
                                            {
                                    let ret_ip = ip;
                                    let saved_handler_len = self.exception_handlers.len();
                                    let saved_stack_len = unsafe { (*stack_ptr).len() };
                                    if self.frames.len() >= self.frame_depth_limit {
                                        runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                    }
                                    self.frames.push(Frame { arena_offset, arena_len: n_locals, return_ip: ret_ip, saved_handler_len, saved_stack_len });
                                    {
                                        let frame = self.frames.last().unwrap();
                                                    locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                                    locals_len = frame.arena_len;
                                                }
                                                self.current_class_name = prev_class;
                                                ip = body;
                                                continue;
                                            }
                                        }
                                        Value::NativeFunction(_) => {
                                            let mut all_args = Vec::with_capacity(args.len() + 1);
                                            all_args.push(obj.clone());
                                            all_args.extend(args.iter().cloned());
                                            let result = try_or_catch!(method_val.call(&all_args, &[], self, module));
                                            {
                                                let frame = self.frames.last().unwrap();
                                                locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                                locals_len = frame.arena_len;
                                            }
                                            hot_push!(result);
                                            continue;
                                        }
                                        _ => {
                                            runtime_error!("استثناء_نوع", format!("ال_method {} ليست function", method_name_str));
                                        }
                                    }
                                }
                            }
                        } else {
                            // Cache miss: lookup + update + handle
                            let val = rc.class.methods.get(method_name_str).cloned();
                            if let Some(ref v) = val {
                                self.mc_methods_ptr = methods_ptr;
                                self.mc_method = name_hash;
                                self.mc_value = v.clone();
                            }
                            match val {
                                Some(Value::FastMethod(fm)) => {
                                    let (v1, v2) = (rc.get_field(fm.field1.as_str()).unwrap_or(Value::Null),
                                                     rc.get_field(fm.field2.as_str()).unwrap_or(Value::Null));
                                    let result = match (&v1, &v2, fm.op) {
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Add) => Value::Integer(a + b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Add) => Value::Float(a + b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Add) => Value::Float(*a as f64 + b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Add) => Value::Float(a + *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Sub) => Value::Integer(a - b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Sub) => Value::Float(a - b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Sub) => Value::Float(*a as f64 - b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Sub) => Value::Float(a - *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Mul) => Value::Integer(a * b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Mul) => Value::Float(a * b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Mul) => Value::Float(*a as f64 * b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Mul) => Value::Float(a * *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Div) => Value::Integer(a / b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Div) => Value::Float(a / b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Div) => Value::Float(*a as f64 / b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Div) => Value::Float(a / *b as f64),
                                        _ => Value::Null,
                                    };
                                    hot_push!(result);
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                    continue;
                                }
                                Some(Value::Function(f)) => {
                                    let param_indices = f.param_indices.clone();
                                    let body = f.body;
                                    let n_locals = f.num_locals.max(1);
                                    let class_name_str_rc = rc.class.name.clone();
                                    let arena_offset = self.locals_arena.len();
                                    self.locals_arena.resize(arena_offset + n_locals, Value::Null);
                                    let local_vars_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                                    let this_val = obj.clone();
                                    if let Some(&idx) = param_indices.first() {
                                        unsafe { *local_vars_ptr.add(idx) = this_val; }
                                    }
                                    for (i, arg) in args.iter().enumerate() {
                                        let local_idx = param_indices.get(i + 1).copied().unwrap_or(i + 1);
                                        if local_idx < n_locals {
                                            unsafe { *local_vars_ptr.add(local_idx) = arg.clone(); }
                                        }
                                    }
                                    let prev_class = self.current_class_name.clone();
                                    self.current_class_name = Some(class_name_str_rc.clone());
                                    {
                                        let ret_ip = ip;
                                        let saved_handler_len = self.exception_handlers.len();
                                        let saved_stack_len = unsafe { (*stack_ptr).len() };
                                        if self.frames.len() >= self.frame_depth_limit {
                                            runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                        }
                                        self.frames.push(Frame { arena_offset, arena_len: n_locals, return_ip: ret_ip, saved_handler_len, saved_stack_len });
                                        {
                                            let frame = self.frames.last().unwrap();
                                            locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                            locals_len = frame.arena_len;
                                        }
                                        self.current_class_name = prev_class;
                                        ip = body;
                                        continue;
                                    }
                                }
                                Some(nf @ Value::NativeFunction(_)) => {
                                    let nf_clone = nf.clone();
                                    let mut all_args = Vec::with_capacity(args.len() + 1);
                                    all_args.push(obj.clone());
                                    all_args.extend(args.iter().cloned());
                                    let result = try_or_catch!(nf_clone.call(&all_args, &[], self, module));
                                    {
                                        let frame = self.frames.last().unwrap();
                                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                        locals_len = frame.arena_len;
                                    }
                                    hot_push!(result);
                                    continue;
                                }
                                _ => {
                                    runtime_error!("استثناء_نوع", format!("ال_method {} ليست function", method_name_str));
                                }
                            }
                        }
                    }
                    match (&obj, method_name_str.as_str()) {
                        // List methods - Rc<RefCell> means mutations persist
                        (Value::List(items), "اضف") => {
                            let val = args.first().cloned().unwrap_or(Value::Null);
                            items.borrow_mut().push(val);
                            hot_push!(Value::Null);
                        }
                        (Value::List(items), "ادرج") => {
                            let idx = match args.first() {
                                Some(Value::Integer(n)) => *n as usize,
                                _ => items.borrow().len(),
                            };
                            let val = args.get(1).cloned().unwrap_or(Value::Null);
                            let i = idx.min(items.borrow().len());
                            items.borrow_mut().insert(i, val);
                            hot_push!(Value::Null);
                        }
                        (Value::List(items), "امسح") => {
                            if let Some(v) = args.first() {
                                items.borrow_mut().retain(|x| x != v);
                            }
                            hot_push!(Value::Null);
                        }
                        (Value::List(items), "رتب") => {
                            items.borrow_mut().sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                            hot_push!(Value::List(items.clone()));
                        }
                        (Value::List(items), "فهرس") => {
                            let val = args.first().cloned().unwrap_or(Value::Null);
                            let idx = items.borrow().iter().position(|x| x == &val).map(|i| i as i64).unwrap_or(-1);
                            hot_push!(Value::Integer(idx));
                        }
                        (Value::List(items), "عدد") => {
                            let val = args.first().cloned().unwrap_or(Value::Null);
                            let count = items.borrow().iter().filter(|x| *x == &val).count() as i64;
                            hot_push!(Value::Integer(count));
                        }
                        (Value::List(items), "احذف_عنصر") => {
                            let idx = args.first().and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None });
                            if let Some(i) = idx {
                                let len = items.borrow().len();
                                if i < len {
                                    items.borrow_mut().remove(i);
                                }
                            }
                            hot_push!(Value::Null);
                        }
                        (Value::List(items), "عكس") => {
                            items.borrow_mut().reverse();
                            hot_push!(Value::List(items.clone()));
                        }
                        (Value::List(items), "امسح_كل") => {
                            items.borrow_mut().clear();
                            hot_push!(Value::Null);
                        }
                        // Set methods
                        (Value::Set(items), "اضف") => {
                            let val = args.first().cloned().unwrap_or(Value::Null);
                            if !items.borrow().contains(&val) {
                                items.borrow_mut().push(val);
                            }
                            hot_push!(Value::Null);
                        }
                        (Value::Set(items), "اسحب") => {
                            let result = items.borrow_mut().pop().unwrap_or(Value::Null);
                            hot_push!(result);
                        }
                        // Dict methods
                        (Value::Dict(pairs), "احضر") => {
                            let key = args.first().cloned().unwrap_or(Value::Null);
                            let default = args.get(1).cloned();
                            let val = pairs.lookup(&key)
                                .or(default)
                                .unwrap_or(Value::Null);
                            hot_push!(val);
                        }
                        (Value::Dict(pairs), "مفاتيح") => {
                            let borrow = pairs.borrow();
                            let keys: Vec<Value> = borrow.iter().map(|(k, _)| k.clone()).collect();
                            drop(borrow);
                            hot_push!(Value::List(SharedList::new(keys)));
                        }
                        (Value::Dict(pairs), "قيم") => {
                            let borrow = pairs.borrow();
                            let vals: Vec<Value> = borrow.iter().map(|(_, v)| v.clone()).collect();
                            drop(borrow);
                            hot_push!(Value::List(SharedList::new(vals)));
                        }
                        (Value::Dict(pairs), "عناصر") => {
                            let borrow = pairs.borrow();
                            let items: Vec<Value> = borrow.iter().map(|(k, v)| Value::Tuple(Rc::new(vec![k.clone(), v.clone()]))).collect();
                            drop(borrow);
                            hot_push!(Value::List(SharedList::new(items)));
                        }
                        (Value::Dict(pairs), "حدث") => {
                            if let Some(Value::Dict(other)) = args.first() {
                                let other_borrow = other.pairs.borrow();
                                for (k, v) in other_borrow.iter() {
                                    pairs.insert(k.clone(), v.clone());
                                }
                            }
                            hot_push!(Value::Null);
                        }
                        (Value::Dict(pairs), "احذف_مفتاح") => {
                            let key = args.first().cloned().unwrap_or(Value::Null);
                            if let Some(v) = pairs.lookup(&key) {
                                let mut borrow = pairs.pairs.borrow_mut();
                                if let Some(pos) = borrow.iter().position(|(k, _)| k == &key) {
                                    borrow.remove(pos);
                                }
                                let mut index = pairs.index.borrow_mut();
                                index.remove(&key);
                                hot_push!(v);
                            } else {
                                hot_push!(Value::Null);
                            }
                        }
                        (Value::Dict(pairs), "امسح") => {
                            pairs.pairs.borrow_mut().clear();
                            pairs.index.borrow_mut().clear();
                            hot_push!(Value::Null);
                        }
                        // File methods
                        (Value::File(_), "__ادخل__") => {
                            hot_push!(obj.clone());
                        }
                        (Value::File(handle), "اكتب") => {
                            let content = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                            let mut borrow = handle.0.borrow_mut();
                            if let Some(ref mut file) = *borrow {
                                use std::io::Write;
                                file.write_all(content.as_bytes()).map_err(|e| format!("خطا في الكتابة: {}", e))?;
                            }
                            hot_push!(Value::Null);
                        }
                        (Value::File(handle), "اقرا") => {
                            let mut borrow = handle.0.borrow_mut();
                            if let Some(ref mut file) = *borrow {
                                use std::io::Read;
                                let size = args.first().and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None });
                                if let Some(sz) = size {
                                    let mut buf = vec![0u8; sz];
                                    let n = file.read(&mut buf).map_err(|e| format!("خطا في القراءة: {}", e))?;
                                    buf.truncate(n);
                                    let content = String::from_utf8_lossy(&buf).to_string();
                                    hot_push!(Value::String(content.into()));
                                } else {
                                    let mut content = String::new();
                                    file.read_to_string(&mut content).map_err(|e| format!("خطا في القراءة: {}", e))?;
                                    hot_push!(Value::String(content.into()));
                                }
                            } else {
                                hot_push!(Value::Null);
                            }
                        }
                        (Value::File(handle), "اقرا_سطر") => {
                            let mut borrow = handle.0.borrow_mut();
                            if let Some(ref mut file) = *borrow {
                                use std::io::BufRead;
                                let mut buf = String::new();
                                let mut reader = std::io::BufReader::new(&mut *file);
                                let n = reader.read_line(&mut buf).map_err(|e| format!("خطا في القراءة: {}", e))?;
                                if n == 0 { hot_push!(Value::Null); } else { hot_push!(Value::String(buf.into())); }
                            } else {
                                hot_push!(Value::Null);
                            }
                        }
                        (Value::File(handle), "اغلق") | (Value::File(handle), "__اترك__") => {
                            let file_opt = handle.0.borrow_mut().take();
                            drop(file_opt);
                            hot_push!(Value::Null);
                        }
                        // String methods
                        (Value::String(s), "افصل") => {
                            let sep = args.first().map(|a| a.to_string_value()).unwrap_or_else(|| " ".to_string());
                            let maxsplit = args.get(1).and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None });
                            let parts: Vec<Value> = if let Some(max) = maxsplit {
                                s.splitn(max + 1, &sep).map(|p| Value::String(Rc::new(p.to_string()))).collect()
                            } else {
                                s.split(&sep).map(|p| Value::String(Rc::new(p.to_string()))).collect()
                            };
                            hot_push!(Value::List(SharedList::new(parts)));
                        }
                        (Value::String(s), "اقسم") => {
                            let sep = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                            let parts: Vec<Value> = if sep.is_empty() {
                                s.chars().map(|c| Value::String(Rc::new(c.to_string()))).collect()
                            } else {
                                s.split(&sep).map(|p| Value::String(Rc::new(p.to_string()))).collect()
                            };
                            hot_push!(Value::Tuple(Rc::new(parts)));
                        }
                        (Value::String(s), "اوجد") => {
                            let sub = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                            let start = args.get(1).and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None }).unwrap_or(0);
                            let end = args.get(2).and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None }).unwrap_or(s.len());
                            let slice = &s[start.min(s.len())..end.min(s.len())];
                            match slice.find(&*sub) {
                                Some(pos) => hot_push!(Value::Integer((start + pos) as i64)),
                                None => hot_push!(Value::Integer(-1)),
                            }
                        }
                        (Value::String(s), "كم") => {
                            let sub = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                            let start = args.get(1).and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None }).unwrap_or(0);
                            let end = args.get(2).and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None }).unwrap_or(s.len());
                            let slice = &s[start.min(s.len())..end.min(s.len())];
                            let count = slice.matches(&*sub).count() as i64;
                            hot_push!(Value::Integer(count));
                        }
                        (Value::String(s), "اربط") => {
                            if let Some(list) = args.first() {
                                match list {
                                    Value::List(items) => {
                                        let strs: Vec<String> = items.borrow().iter().map(|v| v.to_string_value()).collect();
                                        hot_push!(Value::String(strs.join(s).into()));
                                    }
                                    Value::Tuple(items) => {
                                        let strs: Vec<String> = items.iter().map(|v| v.to_string_value()).collect();
                                        hot_push!(Value::String(strs.join(s).into()));
                                    }
                                    _ => hot_push!(Value::String(Rc::new(String::new()))),
                                }
                            } else {
                                hot_push!(Value::String(Rc::new(String::new())));
                            }
                        }
                        (Value::String(s), "استبدل") => {
                            let old = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                            let new = args.get(1).map(|a| a.to_string_value()).unwrap_or_default();
                            let count = args.get(2).and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None });
                            let result = match count {
                                Some(c) => {
                                    let mut r = s.to_string();
                                    let mut remaining = c;
                                    while remaining > 0 {
                                        if let Some(pos) = r.find(&old) {
                                            r.replace_range(pos..pos + old.len(), &new);
                                            remaining -= 1;
                                        } else { break; }
                                    }
                                    Value::String(r.into())
                                }
                                None => Value::String(Rc::new(s.replace(&old, &new))),
                            };
                            hot_push!(result);
                        }
                        (Value::String(s), "افصل_ع") => {
                            let sep = args.first().map(|a| a.to_string_value()).unwrap_or_else(|| " ".to_string());
                            let maxsplit = args.get(1).and_then(|a| match a { Value::Integer(n) => Some(*n as usize), _ => None });
                            let parts: Vec<Value> = if let Some(max) = maxsplit {
                                s.rsplitn(max + 1, &sep).map(|p| Value::String(Rc::new(p.to_string()))).collect()
                            } else {
                                s.rsplit(&sep).map(|p| Value::String(Rc::new(p.to_string()))).collect()
                            };
                            hot_push!(Value::List(SharedList::new(parts)));
                        }
                        (Value::String(s), "اقسم_ع") => {
                            let sep = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                            let mut parts = s.rsplitn(3, &sep);
                            let c = parts.next().unwrap_or("").to_string();
                            let b = parts.next().unwrap_or("").to_string();
                            let a = parts.next().unwrap_or("").to_string();
                            hot_push!(Value::Tuple(Rc::new(vec![Value::String(a.into()), Value::String(b.into()), Value::String(c.into())])));
                        }
                        (Value::String(s), "يبدا") => {
                            let prefix = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                            hot_push!(Value::Boolean(s.starts_with(&prefix)));
                        }
                        (Value::String(s), "ينتهي") => {
                            let suffix = args.first().map(|a| a.to_string_value()).unwrap_or_default();
                            hot_push!(Value::Boolean(s.ends_with(&suffix)));
                        }
                        (Value::String(s), "اكبر") => {
                            hot_push!(Value::String(Rc::new(s.to_uppercase())));
                        }
                        (Value::String(s), "اصغر") => {
                            hot_push!(Value::String(Rc::new(s.to_lowercase())));
                        }
                        (Value::String(_), "شطب") => {
                            let s = obj.to_string_value();
                            hot_push!(Value::String(s.trim().to_string().into()));
                        }
                        _ => {
                            // Fall through to method lookup (skip fields for faster dispatch)
                            let method = obj.get_method(&method_name_str).or_else(|| obj.get_attribute(&method_name_str));
                            match method {
                                Some(Value::FastMethod(fm)) => {
                                    if let Value::Instance(rc) = &obj {
                                        let (v1, v2) = (rc.get_field(fm.field1.as_str()).unwrap_or(Value::Null),
                                                         rc.get_field(fm.field2.as_str()).unwrap_or(Value::Null));
                                        let result = match (&v1, &v2, fm.op) {
                                            (Value::Integer(a), Value::Integer(b), FastMethodOp::Add) => Value::Integer(a + b),
                                            (Value::Float(a), Value::Float(b), FastMethodOp::Add) => Value::Float(a + b),
                                            (Value::Integer(a), Value::Float(b), FastMethodOp::Add) => Value::Float(*a as f64 + b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Add) => Value::Float(a + *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Sub) => Value::Integer(a - b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Sub) => Value::Float(a - b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Sub) => Value::Float(*a as f64 - b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Sub) => Value::Float(a - *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Mul) => Value::Integer(a * b),
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Mul) => Value::Float(a * b),
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Mul) => Value::Float(*a as f64 * b),
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Mul) => Value::Float(a * *b as f64),
                                        (Value::Integer(a), Value::Integer(b), FastMethodOp::Div) => { if *b == 0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); } Value::Integer(a / b) }
                                        (Value::Float(a), Value::Float(b), FastMethodOp::Div) => { if *b == 0.0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); } Value::Float(a / b) }
                                        (Value::Integer(a), Value::Float(b), FastMethodOp::Div) => { if *b == 0.0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); } Value::Float(*a as f64 / b) }
                                        (Value::Float(a), Value::Integer(b), FastMethodOp::Div) => { if *b == 0 { runtime_error!("استثناء_قسمة", "القسمة على صفر".to_string()); } Value::Float(a / *b as f64) }
                                        _ => Value::Null,
                                        };
                                        hot_push!(result);
                                        continue;
                                    }
                                    runtime_error!("استثناء_نوع", "الوصول الميداني يتطلب مثياً".to_string());
                                }
                                Some(Value::Function(f)) => {
                                    let param_indices = &f.param_indices;
                                    let body = f.body;
                                    let n_locals = f.num_locals.max(1);
                                    let arena_offset = self.locals_arena.len();
                                    self.locals_arena.resize(arena_offset + n_locals, Value::Null);
                                    let local_vars_ptr = unsafe { self.locals_arena.as_mut_ptr().add(arena_offset) };
                                    let this_val = if matches!(&obj, Value::Class(_)) {
                                        self.current_instance.clone().unwrap_or(obj.clone())
                                    } else {
                                        obj.clone()
                                    };
                                    if let Some(&idx) = param_indices.first() {
                                        unsafe { *local_vars_ptr.add(idx) = this_val; }
                                    }
                                    for (i, arg) in args.iter().enumerate() {
                                        let local_idx = param_indices.get(i + 1).copied().unwrap_or(i + 1);
                                        if local_idx < n_locals {
                                            unsafe { *local_vars_ptr.add(local_idx) = arg.clone(); }
                                        }
                                    }
                                    let prev_class = self.current_class_name.clone();
                                    if let Value::Instance(rc) = &obj {
                                        self.current_class_name = Some(rc.class.name.clone());
                                    }
                                    if f.module_index.is_none() && f.varargs_param.is_none() && f.kwargs_param.is_none() && !f.is_generator {
                                        let ret_ip = ip;
                                        let saved_handler_len = self.exception_handlers.len();
                                        let saved_stack_len = unsafe { (*stack_ptr).len() };
                                        if self.frames.len() >= self.frame_depth_limit {
                                            runtime_error!("استثناء_بنية", format!("تجاوزت عمق الاستدعاء الحد الاقصى ({})", self.frame_depth_limit));
                                        }
                                        self.frames.push(Frame { arena_offset, arena_len: n_locals, return_ip: ret_ip, saved_handler_len, saved_stack_len });
                                        {
                                            let frame = self.frames.last().unwrap();
                                            locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                                            locals_len = frame.arena_len;
                                        }
                                        self.current_class_name = prev_class;
                                        ip = body;
                                        continue;
                                    } else {
                                        self.push_frame(arena_offset, n_locals, body)?;
                                        let result = try_or_catch!(self.run_frame(module));
                                        self.pop_frame();
                                        self.current_class_name = prev_class;
                                        hot_push!(result);
                                        continue;
                                    }
                                }
                                Some(nf @ Value::NativeFunction(_)) => {
                                    let all_args = if matches!(&obj, Value::Class(_)) {
                                        args.to_vec()
                                    } else {
                                        let mut a = vec![obj];
                                        a.extend_from_slice(args);
                                        a
                                    };
                                    let result = try_or_catch!(nf.call(&all_args, &[], self, module));
                                    hot_push!(result);
                                }
                                _ => {
                                    runtime_error!("استثناء_اسم", format!("ال method {} غير موجود على {}", method_name_str, obj.type_name()));
                                }
                            }
                        }
                    }
                    // Re-acquire locals pointer after frame change in CallMethod
                    {
                        let frame = self.frames.last().unwrap();
                        locals_ptr = unsafe { self.locals_arena.as_mut_ptr().add(frame.arena_offset) };
                        locals_len = frame.arena_len;
                    }
                }

                OP_CALL_METHOD_VOID => {
                    runtime_error!("خطا", "CallMethodVoid should not be reached — disable peephole pass 32".to_string());
                }

                OP_IMPORT_MODULE => {
                    let module_name_val = hot_pop!();
                    let module_name = match &module_name_val {
                        Value::String(s) => s.clone(),
                        _ => {
                            runtime_error!("استثناء_نوع", "اسم الوحدة يجب ان يكون نصاً".to_string());
                        }
                    };

                    // Handle dotted module names (e.g., "حساب.نوع")
                    let parts: Vec<&str> = module_name.split('.').collect();
                    let root_name = parts[0].to_string();

                    let root_obj = if let Some(cached) = self.module_cache.get(&root_name) {
                        cached.clone()
                    } else {
                        let built = match root_name.as_str() {
                        "حساب" => {
                            let math_fns = vec![
                                ("جيب", "جيب"), ("تجيب", "تجيب"), ("ظل", "ظل"),
                                ("جيب_عكسي", "جيب_عكسي"), ("تجيب_عكسي", "تجيب_عكسي"), ("ظل_عكسي", "ظل_عكسي"),
                                ("جذر", "جذر"), ("مربع", "مربع"), ("مكعب", "مكعب"), ("جذر_مكعب", "جذر_مكعب"),
                                ("قوة", "قوة"), ("مطلق", "مطلق"), ("قيمة_مطلقة", "قيمة_مطلقة"),
                                ("ارضية", "ارضية"), ("سقف", "سقف"), ("تقريب", "تقريب"),
                                ("لوغ", "لوغ"), ("لوغ10", "لوغ10"), ("لوغ2", "لوغ2"),
                                ("مضروب", "مضروب"), ("قم_اكبر", "قم_اكبر"), ("قم_اصغر", "قم_اصغر"),
                                ("حد_اعلى", "حد_اعلى"), ("حد_ادنى", "حد_ادنى"),
                                ("راديان", "راديان"), ("درجة", "درجة"),
                                ("متوسط", "متوسط"), ("مسافة", "مسافة"),
                                ("اكبر_قاسم", "اكبر_قاسم"), ("اصغر_مضاعف", "اصغر_مضاعف"), ("هل_اولية", "هل_اولية"), ("اولية", "اولية"),
                                ("فيبوناتشي", "فيبوناتشي"), ("ترتيبي", "ترتيبي"), ("تركيبي", "تركيبي"),
                                ("انحراف_معياري", "انحراف_معياري"), ("وسيط", "وسيط"),
                                ("قسمة_ومعظم", "قسمة_ومعظم"), ("مغلق", "مغلق"),
                                ("لانهاية", "لانهاية"), ("ليس_رقم", "ليس_رقم"),
                                ("مجموع_مربعات", "مجموع_مربعات"), ("متوسط_وزني", "متوسط_وزني"),
                            ];
                            let mut methods = HashMap::new();
                            for (name, fn_name) in math_fns {
                                methods.insert(name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: fn_name.to_string(), arity: 0 })));
                            }
                            let mut fields = HashMap::new();
                            fields.insert("ط".to_string(), Value::Float(std::f64::consts::PI));
                            fields.insert("ه".to_string(), Value::Float(std::f64::consts::E));
                            fields.insert("ن".to_string(), Value::Float(std::f64::consts::TAU));
                            Value::Class(Rc::new(ClassData {
                                name: Rc::from(root_name.as_str()),
                                methods: Rc::new(methods),
                                fields,
                                parents: Vec::new(),
                                field_names: Vec::new(),
                                field_index: Rc::new(HashMap::new()),
                            }))
                        }
                        "وقت" => {
                            let mut methods = HashMap::new();
                            let time_fns = vec![
                                ("الآن", "الآن"), ("مللي", "مللي"),
                                ("غفوة", "غفوة"), ("وقت", "وقت"),
                                ("توقيت", "توقيت"), ("تاريخ", "تاريخ"),
                                ("قائمة", "قائمة"), ("منسق", "منسق"),
                                ("عداد", "عداد"),
                                ("سنة", "سنة"), ("شهر", "شهر"), ("يوم", "يوم"),
                                ("ساعة", "ساعة"), ("دقيقة", "دقيقة"), ("ثانية", "ثانية"),
                                ("يوم_الاسبوع", "يوم_الاسبوع"), ("هل_سنة_كبيسة", "هل_سنة_كبيسة"),
                                ("ايام_الشهر", "ايام_الشهر"),
                            ];
                            for (name, fn_name) in time_fns {
                                methods.insert(name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: fn_name.to_string(), arity: 0 })));
                            }
                            Value::Class(Rc::new(ClassData {
                                name: Rc::from(root_name.as_str()),
                                methods: Rc::new(methods),
                                fields: HashMap::new(),
                                parents: Vec::new(),
                                field_names: Vec::new(),
                                field_index: Rc::new(HashMap::new()),
                            }))
                        }
                        "نص" => {
                            let mut methods = HashMap::new();
                            let str_fns = vec![
                                ("استبدل", "استبدل"), ("اقسم", "اقسم"),
                                ("يحتوي", "يحتوي"), ("شطب", "شطب"),
                                ("طول", "طول"), ("اوجد", "اوجد"),
                                ("يبدا", "يبدا"), ("ينتهي", "ينتهي"),
                                ("اكبر", "اكبر"), ("اصغر", "اصغر"),
                                ("كرر", "كرر"), ("حرف", "حرف"),
                                ("اربط", "اربط"), ("قص", "قص"),
                                ("مكرر", "كرر"), ("كم", "كم"),
                                ("بداية_بـ", "بداية_بـ"), ("نهاية_بـ", "نهاية_بـ"), ("اعلى", "اعلى"), ("اسفل", "اسفل"),
                                ("تكرار_اعلى", "تكرار_اعلى"), ("ملء", "ملء"), ("توسيط", "توسيط"), ("قطع", "قطع"),
                                ("عدد", "عدد"), ("اوجد_النهاية", "اوجد_النهاية"), ("حرف_البداية", "حرف_البداية"),
                                ("حرف_النهاية", "حرف_النهاية"), ("معكوس_نص", "معكوس_نص"), ("تكرار", "تكرار"),
                                ("تحقق_من_الحرف", "تحقق_من_الحرف"), ("اقلب", "اقلب"), ("ملء_صفر", "ملء_صفر"),
                                ("اول_حرف_كبير", "اول_حرف_كبير"), ("كل_اول_حرف_كبير", "كل_اول_حرف_كبير"),
                                ("تجزئة", "تجزئة"), ("تحويل_لارقام", "تحويل_لارقام"), ("تحويل_من_ارقام", "تحويل_من_ارقام"),
                                ("تنسيق", "تنسيق"), ("يحتوي_اي", "يحتوي_اي"), ("تقطيع", "تقطيع"),
                            ];
                            for (name, fn_name) in str_fns {
                                methods.insert(name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: fn_name.to_string(), arity: 0 })));
                            }
                            Value::Class(Rc::new(ClassData {
                                name: Rc::from(root_name.as_str()),
                                methods: Rc::new(methods),
                                fields: HashMap::new(),
                                parents: Vec::new(),
                                field_names: Vec::new(),
                                field_index: Rc::new(HashMap::new()),
                            }))
                        }
                        "عشوائي" => {
                            let mut methods = HashMap::new();
                            methods.insert("عشوائي".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "عشوائي".to_string(), arity: 0 })));
                            methods.insert("عشوائي_صحيح".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "عشوائي_صحيح".to_string(), arity: 0 })));
                            methods.insert("بذرة".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "بذرة".to_string(), arity: 0 })));
                            methods.insert("منتظم".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "منتظم".to_string(), arity: 0 })));
                            methods.insert("اختيار".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "اختيار".to_string(), arity: 0 })));
                            methods.insert("عينة".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "عينة".to_string(), arity: 0 })));
                            methods.insert("خلط".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "خلط".to_string(), arity: 0 })));
                            methods.insert("طبيعي".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "طبيعي".to_string(), arity: 0 })));
                            methods.insert(" برنولي".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: " برنولي".to_string(), arity: 0 })));
                            methods.insert("برنولي".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "برنولي".to_string(), arity: 0 })));
                            methods.insert("عشوائي_نطاق".to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: "عشوائي_نطاق".to_string(), arity: 0 })));
                            Value::Class(Rc::new(ClassData {
                                name: Rc::from(root_name.as_str()),
                                methods: Rc::new(methods),
                                fields: HashMap::new(),
                                parents: Vec::new(),
                                field_names: Vec::new(),
                                field_index: Rc::new(HashMap::new()),
                            }))
                        }
                        "كائن" => {
                            let mut methods = HashMap::new();
                            let json_fns = vec![
                                ("تحليل", "كائن_تحليل"),
                                ("تحويل", "كائن_تحويل"),
                                ("تحليل_ملف", "كائن_تحليل_ملف"),
                                ("اخرج_ملف", "كائن_اخرج_ملف"),
                                ("جميل", "جميل"), ("تحقق", "تحقق"),
                            ];
                            for (name, fn_name) in json_fns {
                                methods.insert(name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: fn_name.to_string(), arity: 0 })));
                            }
                            Value::Class(Rc::new(ClassData {
                                name: Rc::from(root_name.as_str()),
                                methods: Rc::new(methods),
                                fields: HashMap::new(),
                                parents: Vec::new(),
                                field_names: Vec::new(),
                                field_index: Rc::new(HashMap::new()),
                            }))
                        }
                        "نظام" => {
                            let mut methods = HashMap::new();
                            let os_fns = vec![
                                ("ادارة", "نظام_ادارة"),
                                ("ادخل_ادارة", "نظام_ادخل_ادارة"),
                                ("قائمة_ادارة", "نظام_قائمة_ادارة"),
                                ("اداء", "نظام_اداء"),
                                ("وجود", "نظام_وجود"),
                                ("انشئ_ادارة", "نظام_انشئ_ادارة"),
                                ("احذف_ملف", "نظام_احذف_ملف"),
                                ("اسم_النظام", "نظام_اسم"),
                                ("ادخال", "نظام_ادخال"),
                                ("احصل_على", "نظام_احصل_على"),
                                ("ادخل_بيئة", "نظام_ادخل_بيئة"),
                                ("اسم_ملف", "نظام_اسم_ملف"),
                                ("ادارة_ملف", "نظام_ادارة_ملف"),
                                ("امتداد", "نظام_امتداد"),
                                ("انضم", "انضم"), ("مسار_مطلق", "مسار_مطلق"), ("نسخ", "نسخ"), ("نقل", "نقل"),
                                ("قائمة_مجلد", "قائمة_مجلد"), ("مشي", "مشي"), ("متغير_بيئي", "متغير_بيئي"),
                                ("اقرا_ملف", "اقرا_ملف"), ("اكتب_ملف", "اكتب_ملف"), ("اضف_ملف", "اضف_ملف"),
                                ("يوجد", "يوجد"), ("ملف_الحجم", "ملف_الحجم"), ("التمس", "التمس"),
                                ("اقرا_اسطر", "اقرا_اسطر"), ("اسم_الملف", "اسم_الملف"),
                                ("امتداد_ملف", "امتداد_ملف"), ("المسار_المجلد", "المسار_المجلد"),
                            ];
                            for (name, fn_name) in os_fns {
                                methods.insert(name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: fn_name.to_string(), arity: 0 })));
                            }
                            Value::Class(Rc::new(ClassData {
                                name: Rc::from(root_name.as_str()),
                                methods: Rc::new(methods),
                                fields: HashMap::new(),
                                parents: Vec::new(),
                                field_names: Vec::new(),
                                field_index: Rc::new(HashMap::new()),
                            }))
                        }
                        "تاريخ" => {
                            let mut methods = HashMap::new();
                            let datetime_fns = vec![
                                ("زمان", "زمان"), ("تحويل", "تحويل"),
                                ("فرق", "فرق"), ("تاريخ", "تاريخ"),
                                ("الآن", "الآن"), ("توقيت", "توقيت"),
                                ("سنة", "سنة"), ("شهر", "شهر"), ("يوم", "يوم"),
                                ("ساعة", "ساعة"), ("دقيقة", "دقيقة"), ("ثانية", "ثانية"),
                                ("يوم_الاسبوع", "يوم_الاسبوع"), ("هل_سنة_كبيسة", "هل_سنة_كبيسة"),
                                ("ايام_الشهر", "ايام_الشهر"),
                            ];
                            for (name, fn_name) in datetime_fns {
                                methods.insert(name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: fn_name.to_string(), arity: 0 })));
                            }
                            Value::Class(Rc::new(ClassData {
                                name: Rc::from(root_name.as_str()),
                                methods: Rc::new(methods),
                                fields: HashMap::new(),
                                parents: Vec::new(),
                                field_names: Vec::new(),
                                field_index: Rc::new(HashMap::new()),
                            }))
                        }
                        "نمط" => {
                            let mut methods = HashMap::new();
                            let regex_fns = vec![
                                ("طابق", "نمط_طابق"), ("ابحث", "نمط_ابحث"),
                                ("استبدل", "نمط_استبدل"), ("قسم", "نمط_قسم"),
                                ("جميع", "نمط_جميع"), ("كل_التطابقات", "نمط_كل_التطابقات"),
                            ];
                            for (name, fn_name) in regex_fns {
                                methods.insert(name.to_string(), Value::NativeFunction(Rc::new(NativeFunctionData { name: fn_name.to_string(), arity: 0 })));
                            }
                            Value::Class(Rc::new(ClassData {
                                name: Rc::from(root_name.as_str()),
                                methods: Rc::new(methods),
                                fields: HashMap::new(),
                                parents: Vec::new(),
                                field_names: Vec::new(),
                                field_index: Rc::new(HashMap::new()),
                            }))
                        }
                        _ => {
                            // Try file-based import from search directories (LAZY)
                            let mut found_path = None;
                            let mut all_search_dirs = self.search_dirs.clone();
                            for dir in &self.search_dirs {
                                let pakgs_dir = dir.join("__pakgs__");
                                if pakgs_dir.is_dir() {
                                    all_search_dirs.push(pakgs_dir);
                                }
                            }
                            for dir in &all_search_dirs {
                                // 1) Try as package directory: <dir>/<name>/__بداية__.txt
                                let pkg_dir = dir.join(&root_name);
                                if pkg_dir.is_dir() {
                                    for ext in &["txt", "عربي"] {
                                        let init_path = pkg_dir.join(format!("__بداية__.{}", ext));
                                        if init_path.exists() {
                                            found_path = Some(init_path);
                                            break;
                                        }
                                    }
                                }
                                if found_path.is_some() { break; }
                                // 2) Try as plain file: <dir>/<name>.txt
                                for ext in &["txt", "عربي"] {
                                    let path = dir.join(format!("{}.{}", root_name, ext));
                                    if path.exists() {
                                        found_path = Some(path);
                                        break;
                                    }
                                }
                                if found_path.is_some() { break; }
                            }
                            if let Some(path) = found_path {
                                // Create a lazy module — defer compilation until first attribute access
                                let lazy = Value::LazyModule(Box::new(crate::frame::LazyModuleData {
                                    name: root_name.clone(),
                                    path: path.clone(),
                                    search_dirs: self.search_dirs.clone(),
                                    loaded: Rc::new(RefCell::new(None)),
                                }));
                                self.module_cache.insert(root_name.clone(), lazy.clone());
                                lazy
                            } else {
                                runtime_error!("استثناء_اسم", format!("الوحدة '{}' غير موجودة", root_name));
                            }
                        }
                    };
                        self.module_cache.insert(root_name.clone(), built.clone());
                        built
                    };

                    // Traverse dotted path if needed
                    let module_obj = if parts.len() > 1 {
                        let mut current = root_obj;
                        for part in &parts[1..] {
                            match current.get_attribute(part) {
                                Some(val) => current = val,
                                None => runtime_error!("استثناء_اسم", format!("النوع '{}' غير موجود في الوحدة", part)),
                            }
                        }
                        current
                    } else {
                        root_obj
                    };

                    hot_push!(module_obj);
                }

                // Exception handling
                OP_SETUP_EXCEPT => {
                    self.exception_handlers.push(c);
                }
                OP_SETUP_FINALLY => {
                    self.exception_handlers.push(c);
                }
                OP_END_EXCEPT => {
                    self.exception_handlers.pop();
                }
                OP_CHECK_EXCEPTION_TYPE => {
                    let target = if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(b) {
                        s.clone()
                    } else {
                        runtime_error!("استثناء_بنية".to_string(), "CheckExceptionType: target not found".to_string());
                    };
                    let exc_val = hot_pop!();
                    let exc_type = match &exc_val {
                        Value::Exception(d) => d.class_name.clone(),
                        _ => {
                            runtime_error!("استثناء_نوع".to_string(), "القيمة ليست استثناء".to_string());
                        }
                    };
                    let matches = self.is_exception_child(&exc_type, &target);
                    if matches {
                        hot_push!(exc_val);
                        hot_push!(Value::Boolean(true));
                    } else {
                        hot_push!(exc_val);
                        hot_push!(Value::Boolean(false));
                    }
                }
                OP_REGISTER_EXCEPTION_CLASS => {
                    if let Some(arabi_compiler::compiler::Value::String(s)) = hot_const!().get(b) {
                        let parts: Vec<&str> = s.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            let child = parts[0].to_string();
                            let parents: Vec<String> = parts[1].split(',').map(String::from).collect();
                            self.exception_hierarchy.insert(child, parents);
                        }
                    }
                }
                OP_RAISE => {
                    let exc_val = hot_pop!();
                    let exc = match &exc_val {
                        Value::Exception(_) => exc_val,
                        Value::Instance(rc) => {
                            let is_exc = self.is_exception_child(rc.class.name.as_ref(), "استثناء");
                            if is_exc {
                                let exc = ExceptionData {
                                    class_name: rc.class.name.to_string(),
                                    message: exc_val.to_string_value(),
                                    line: None,
                                    call_stack: Vec::new(),
                                };
                                Value::Exception(Box::new(exc))
                            } else {
                                Value::Exception(Box::new(ExceptionData {
                                    class_name: "استثناء".to_string(),
                                    message: exc_val.to_string_value(),
                                    line: None,
                                    call_stack: Vec::new(),
                                }))
                            }
                        }
                        Value::Null => {
                            // Re-raise current exception
                            self.current_exception.clone().unwrap_or_else(|| {
                                RuntimeError::new("لا يوجد استثناء لاعادة رميه").into_value()
                            })
                        }
                        other => {
                            // Convert to exception
                            Value::Exception(Box::new(ExceptionData {
                                class_name: "استثناء".to_string(),
                                message: other.to_string_value(),
                                line: None,
                                call_stack: Vec::new(),
                            }))
                        }
                    };
                    self.current_exception = Some(exc.clone());

                    // Unwind to handler (only in current frame)
                    let frame_depth = self.frames.len();
                    let saved_len = if frame_depth > 0 { self.frames[frame_depth - 1].saved_handler_len } else { 0 };
                    if self.exception_handlers.len() > saved_len {
                        if let Some(handler_ip) = self.exception_handlers.last().copied() {
                            self.exception_handlers.pop();
                            hot_push!(exc);
                            ip = handler_ip;
                            continue;
                        }
                    }
                    return Err(RuntimeError::new_typed(exc.type_name(), format!("استثناء غير مُعالَج: {}", exc.to_string_value())).with_line(if ip > 0 { module.lines.get(ip - 1).copied().unwrap_or(0) as usize } else { 0 }));
                }

                _ => {}
            }
        }

        Ok(result)
    }

    #[inline]
    fn current_frame(&self) -> &Frame {
        self.frames.last().expect("BUG: لا يوجد اطار نشط")
    }

    fn load_lazy_module(&mut self, data: &crate::frame::LazyModuleData) -> Result<Value, RuntimeError> {
        // Check if already loaded
        if let Some(loaded) = data.loaded.borrow().as_ref() {
            return Ok(loaded.clone());
        }

        let path = &data.path;
        let root_name = &data.name;

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return Err(RuntimeError::new_typed("استثناء_نوع", format!("خطا في قراءة الوحدة '{}': {}", root_name, e))),
        };
        let mut lexer = arabi_lexer::Lexer::new(&source);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(e) => return Err(RuntimeError::new_typed("استثناء_نوع", format!("خطا في تحليل الوحدة '{}': {}", root_name, e))),
        };
        let mut parser = arabi_parser::Parser::new(tokens);
        let ast = match parser.parse() {
            Ok(a) => a,
            Err(e) => return Err(RuntimeError::new_typed("استثناء_نوع", format!("خطا في توزيع الوحدة '{}': {}", root_name, e))),
        };
        let mut compiler = arabi_compiler::Compiler::new();
        let mut module = match compiler.compile(&ast) {
            Ok(m) => m,
            Err(e) => return Err(RuntimeError::new_typed("استثناء_نوع", format!("خطا في ترجمة الوحدة '{}': {}", root_name, e))),
        };
        let mut child_vm = VM::new();
        child_vm.search_dirs = data.search_dirs.clone();
        match child_vm.execute(&mut module) {
            Ok(_) => {}
            Err(e) => return Err(RuntimeError::new_typed("استثناء_نوع", format!("خطا في تنفيذ الوحدة '{}': {}", root_name, e))),
        }

        let module_ref = std::cell::RefCell::new(module);
        let module_index = self.imported_modules.len();
        self.imported_modules.push(module_ref);

        let mut fields = HashMap::new();
        for (k, v) in &child_vm.globals {
            if !BUILTIN_NAMES.contains(k.as_str()) && !k.starts_with("استثناء") {
                let tagged_value = match v {
                    Value::Function(f) => {
                        let has_varargs = f.varargs_param.is_some();
                        let has_kwargs = f.kwargs_param.is_some();
                        let npc = f.params.len()
                            - if has_varargs { 1 } else { 0 }
                            - if has_kwargs { 1 } else { 0 };
                        Value::Function(Rc::new(FunctionData {
                            name: f.name.clone(),
                            params: f.params.clone(),
                            param_indices: f.param_indices.clone(),
                            defaults: f.defaults.clone(),
                            body: f.body,
                            closure: f.closure.clone(),
                            varargs_param: f.varargs_param.clone(),
                            kwargs_param: f.kwargs_param.clone(),
                            is_generator: f.is_generator,
                            module_index: Some(module_index),
                            num_locals: f.num_locals,
                            normal_param_count: npc,
                            call_count: Cell::new(0),
                            jit_entry: Cell::new(None),
                            jit_attempted: Cell::new(false),
                        }))
                    }
                    other => other.clone(),
                };
                fields.insert(k.clone(), tagged_value);
            }
        }

        let built = Value::Class(Rc::new(ClassData {
            name: Rc::from(root_name.as_str()),
            methods: Rc::new(HashMap::new()),
            fields,
            parents: Vec::new(),
            field_names: Vec::new(),
            field_index: Rc::new(HashMap::new()),
        }));

        // Cache in module_cache and store in lazy data
        self.module_cache.insert(root_name.clone(), built.clone());
        *data.loaded.borrow_mut() = Some(built.clone());

        Ok(built)
    }

    #[inline(always)]
    fn convert_value(&self, val: &arabi_compiler::compiler::Value) -> Value {
        match val {
            arabi_compiler::compiler::Value::Integer(n) => Value::Integer(*n),
            arabi_compiler::compiler::Value::Float(f) => Value::Float(*f),
            arabi_compiler::compiler::Value::String(s) => Value::String(s.clone().into()),
            arabi_compiler::compiler::Value::Boolean(b) => Value::Boolean(*b),
            arabi_compiler::compiler::Value::Null => Value::Null,
        }
    }

    #[inline(always)]
    fn add(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 + b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a + b as f64)),
            (Value::String(a), Value::String(b)) => Ok(Value::String(Rc::new(format!("{}{}", a, b)))),
            (Value::List(a), Value::List(b)) => {
                let mut result = a.borrow().iter().cloned().collect::<Vec<_>>();
                result.extend(b.borrow().iter().cloned());
                Ok(Value::List(SharedList::new(result)))
            }
            (Value::Set(a), Value::Set(b)) => {
                let a_borrow = a.borrow();
                let b_borrow = b.borrow();
                let mut merged: Vec<Value> = a_borrow.clone();
                for item in b_borrow.iter() {
                    if !merged.iter().any(|u| self.equals(u, item)) {
                        merged.push(item.clone());
                    }
                }
                Ok(Value::Set(SharedSet::new(merged)))
            }
            _ => Err(RuntimeError::new_typed("استثناء_نوع", "عملية جمع غير مدعومة")),
        }
    }

    #[inline(always)]
    fn sub(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 - b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a - b as f64)),
            _ => Err(RuntimeError::new_typed("استثناء_نوع", "عملية طرح غير مدعومة")),
        }
    }

    #[inline(always)]
    fn mul(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 * b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a * b as f64)),
            (Value::String(a), Value::Integer(b)) => Ok(Value::String(Rc::new(a.repeat(b as usize)))),
            (Value::List(a), Value::Integer(b)) => {
                let borrowed = a.borrow();
                let mut result = Vec::new();
                for _ in 0..b as usize {
                    result.extend(borrowed.iter().cloned());
                }
                Ok(Value::List(SharedList::new(result)))
            }
            _ => Err(RuntimeError::new_typed("استثناء_نوع", "عملية ضرب غير مدعومة")),
        }
    }

    #[inline(always)]
    fn div(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if b == 0 { return Err(RuntimeError::new_typed("استثناء_قسمة", "القسمة على صفر")); }
                Ok(Value::Float(a as f64 / b as f64))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float(a as f64 / b)),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a / b as f64)),
            _ => Err(RuntimeError::new_typed("استثناء_نوع", "عملية قسمة غير مدعومة")),
        }
    }

    #[inline(always)]
    fn floor_div(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if b == 0 { return Err(RuntimeError::new_typed("استثناء_قسمة", "القسمة على صفر")); }
                Ok(Value::Integer(a / b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float((a / b).floor())),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((a as f64 / b).floor())),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float((a / b as f64).floor())),
            _ => Err(RuntimeError::new_typed("استثناء_نوع", "عملية قسمة صحيحة غير مدعومة")),
        }
    }

    #[inline(always)]
    fn pow(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => Ok(Value::Integer(a.pow(b as u32))),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.powf(b))),
            (Value::Integer(a), Value::Float(b)) => Ok(Value::Float((a as f64).powf(b))),
            (Value::Float(a), Value::Integer(b)) => Ok(Value::Float(a.powf(b as f64))),
            _ => Err(RuntimeError::new_typed("استثناء_نوع", "عملية اس غير مدعومة")),
        }
    }

    #[inline(always)]
    fn modulo(&self, left: Value, right: Value) -> Result<Value, RuntimeError> {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => {
                if b == 0 { return Err(RuntimeError::new_typed("استثناء_قسمة", "القسمة على صفر")); }
                Ok(Value::Integer(a % b))
            }
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
            _ => Err(RuntimeError::new_typed("استثناء_نوع", "عملية باقي القسمة غير مدعومة")),
        }
    }

    #[inline(always)]
    fn neg(&self, value: Value) -> Result<Value, RuntimeError> {
        match value {
            Value::Integer(n) => Ok(Value::Integer(-n)),
            Value::Float(f) => Ok(Value::Float(-f)),
            _ => Err(RuntimeError::new_typed("استثناء_نوع", "عملية سالب غير مدعومة")),
        }
    }

    #[inline(always)]
    fn is_identical(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::String(a), Value::String(b)) => a.as_ptr() == b.as_ptr() && a.len() == b.len(),
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(&a.0, &b.0),
            (Value::Dict(a), Value::Dict(b)) => Rc::ptr_eq(&a.pairs, &b.pairs),
            (Value::Set(a), Value::Set(b)) => Rc::ptr_eq(&a.0, &b.0),
            (Value::Tuple(a), Value::Tuple(b)) => Rc::ptr_eq(a, b),
            (Value::Instance(a), Value::Instance(b)) => {
                Rc::ptr_eq(a, b)
            }
            (Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
            (Value::Cell(a), Value::Cell(b)) => Rc::ptr_eq(a, b),
            (Value::Generator(a), Value::Generator(b)) => Rc::ptr_eq(a, b),
            (Value::File(a), Value::File(b)) => Rc::ptr_eq(&a.0, &b.0),
            _ => false,
        }
    }

    #[inline(always)]
    fn equals(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Tuple(a), Value::Tuple(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| self.equals(x, y))
            }
            (Value::List(a), Value::List(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| self.equals(x, y))
            }
            (Value::Dict(a), Value::Dict(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len() && a.iter().all(|(k, v)| {
                    b.iter().any(|(bk, bv)| self.equals(k, bk) && self.equals(v, bv))
                })
            }
            (Value::Set(a), Value::Set(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                a.len() == b.len() && a.iter().all(|x| b.iter().any(|y| self.equals(x, y)))
            }
            _ => false,
        }
    }

    #[inline(always)]
    fn less_than(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a < b,
            (Value::Float(a), Value::Float(b)) => a < b,
            (Value::String(a), Value::String(b)) => a < b,
            _ => false,
        }
    }

    #[inline(always)]
    fn greater_than(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::Integer(a), Value::Integer(b)) => a > b,
            (Value::Float(a), Value::Float(b)) => a > b,
            (Value::String(a), Value::String(b)) => a > b,
            _ => false,
        }
    }
}

fn apply_format(val: &Value, fmt: &str) -> String {
    if fmt.is_empty() {
        return val.to_string_value();
    }
    let mut chars = fmt.chars().peekable();
    let mut width: Option<usize> = None;
    let mut precision: Option<usize> = None;
    let mut align = ' '; // default: right align
    let mut fill = ' ';
    let mut spec_char = None;

    // Parse fill and align
    if let Some(&c) = chars.peek() {
        if c == '<' || c == '>' || c == '^' || c == '=' || c == ':' || c == '.' || c.is_ascii_digit() {
            // These start the main parsing, no fill+align prefix
        } else {
            chars.next();
            if let Some(&next) = chars.peek() {
                if next == '<' || next == '>' || next == '^' {
                    fill = c;
                    chars.next();
                    align = next;
                } else {
                    // Not a fill+align, push back
                }
            }
        }
    }

    while let Some(c) = chars.next() {
        match c {
            '<' => align = '<',
            '>' => align = '>',
            '^' => align = '^',
            '0' if width.is_none() && precision.is_none() => {
                fill = '0';
            }
            '.' => {
                precision = Some(0);
                while let Some(&d) = chars.peek() {
                    if d.is_ascii_digit() {
                        chars.next();
                        precision = Some(precision.unwrap_or(0) * 10 + d as usize - '0' as usize);
                    } else {
                        break;
                    }
                }
            }
            'f' | 'F' => spec_char = Some('f'),
            'd' | 'D' | 'ع' => spec_char = Some('d'),
            'e' | 'E' | 'س' => spec_char = Some('e'),
            'g' | 'G' | 'ن' => spec_char = Some('g'),
            'p' | 'P' | 'ف' => spec_char = Some('f'),
            ',' => spec_char = Some(','),
            '%' => spec_char = Some('%'),
            _ if c.is_ascii_digit() => {
                if width.is_none() {
                    width = Some(0);
                }
                width = Some(width.unwrap_or(0) * 10 + c as usize - '0' as usize);
            }
            _ => {}
        }
    }

    let raw = match val {
        Value::Integer(n) => match spec_char {
            Some('d') => format!("{}", n),
            Some('f') => format!("{:.prec$}", *n as f64, prec = precision.unwrap_or(0)),
            Some('e') | Some('g') => format!("{:.prec$}", *n as f64, prec = precision.unwrap_or(0)),
            Some(',') => {
                let s = format!("{}", n);
                let mut result = String::new();
                let chars: Vec<char> = s.chars().collect();
                for (i, c) in chars.iter().enumerate() {
                    if i > 0 && (chars.len() - i).is_multiple_of(3) {
                        result.push(',');
                    }
                    result.push(*c);
                }
                result
            }
            Some('%') => format!("{:.prec$}%", *n as f64 * 100.0, prec = precision.unwrap_or(0)),
            _ => format!("{}", n),
        },
        Value::Float(f) => match spec_char {
            Some('f') | Some('F') => format!("{:.prec$}", f, prec = precision.unwrap_or(6)),
            Some('e') | Some('E') | Some('س') => format!("{:.prec$e}", f, prec = precision.unwrap_or(6)),
            Some('g') | Some('G') | Some('ن') => format!("{:.prec$}", f, prec = precision.unwrap_or(6)),
            Some('%') => format!("{:.prec$}%", f * 100.0, prec = precision.unwrap_or(0)),
            _ => format!("{:.prec$}", f, prec = precision.unwrap_or(6)),
        },
        _ => val.to_string_value(),
    };

    // Apply width and alignment
    if let Some(w) = width {
        if raw.len() < w {
            let padding = w - raw.len();
            let pad_str = if fill == '0' && spec_char != Some(',') { "0" } else { " " };
            match align {
                '<' => format!("{}{}", raw, pad_str.repeat(padding)),
                '^' => {
                    let left_pad = padding / 2;
                    let right_pad = padding - left_pad;
                    format!("{}{}{}", pad_str.repeat(left_pad), raw, pad_str.repeat(right_pad))
                }
                _ => format!("{}{}", pad_str.repeat(padding), raw),
            }
        } else {
            raw
        }
    } else {
        raw
    }
}
