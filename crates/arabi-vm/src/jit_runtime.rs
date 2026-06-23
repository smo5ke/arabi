use crate::frame::{Value, SharedList};
use std::cell::Cell;
use std::rc::Rc;
use std::time::SystemTime;

// Thread-local pointers for JIT → VM fallback calls
thread_local! {
    static VM_PTR: Cell<*mut std::ffi::c_void> = Cell::new(std::ptr::null_mut());
    static MODULE_PTR: Cell<*mut std::ffi::c_void> = Cell::new(std::ptr::null_mut());
}

pub unsafe fn set_jit_context(vm: *mut std::ffi::c_void, module: *mut std::ffi::c_void) {
    VM_PTR.with(|c| c.set(vm));
    MODULE_PTR.with(|c| c.set(module));
}

pub fn clear_jit_context() {
    VM_PTR.with(|c| c.set(std::ptr::null_mut()));
    MODULE_PTR.with(|c| c.set(std::ptr::null_mut()));
}

#[no_mangle]
pub extern "C" fn arabi_jit_load_int(locals: *const Value, idx: u32) -> i64 {
    unsafe {
        let slot = &*locals.add(idx as usize);
        if let Value::Integer(v) = slot { *v } else { 0 }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_store_int(locals: *mut Value, idx: u32, val: i64) {
    unsafe {
        let slot = &mut *locals.add(idx as usize);
        *slot = Value::Integer(val);
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_load_float_as_int(locals: *const Value, idx: u32) -> f64 {
    unsafe {
        let slot = &*locals.add(idx as usize);
        match slot {
            Value::Float(f) => *f,
            Value::Integer(n) => *n as f64,
            _ => 0.0,
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_store_float(locals: *mut Value, idx: u32, val: f64) {
    unsafe {
        let slot = &mut *locals.add(idx as usize);
        *slot = Value::Float(val);
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_build_list(locals: *mut Value, idx: u32) {
    unsafe {
        let slot = &mut *locals.add(idx as usize);
        *slot = Value::List(SharedList::new(Vec::new()));
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_list_append(locals: *mut Value, list_idx: u32, val_idx: u32) {
    unsafe {
        let list_val = &*locals.add(list_idx as usize);
        let val_val = &*locals.add(val_idx as usize);
        if let Value::List(list) = list_val {
            list.push(val_val.clone());
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_subscript_int(locals: *const Value, list_idx: u32, key_idx: u32) -> i64 {
    unsafe {
        let list_val = &*locals.add(list_idx as usize);
        let key_val = &*locals.add(key_idx as usize);
        match (list_val, key_val) {
            (Value::List(list), Value::Integer(key)) => {
                let idx = (*key).max(0) as usize;
                if idx < list.len() {
                    match list.get_unchecked(idx) {
                        Value::Integer(v) => *v,
                        _ => 0,
                    }
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_subscript_int_compare(
    locals: *const Value,
    list_idx: u32,
    key_idx: u32,
    cmp_idx: u32,
) -> i64 {
    unsafe {
        let list_val = &*locals.add(list_idx as usize);
        let key_val = &*locals.add(key_idx as usize);
        let cmp_val = &*locals.add(cmp_idx as usize);
        match (list_val, key_val) {
            (Value::List(list), Value::Integer(i)) => {
                let len = list.len();
                let idx = if *i < 0 { len as i64 + i } else { *i } as usize;
                if idx < len {
                    let elem = list.get_unchecked(idx);
                    let are_equal = match (elem, cmp_val) {
                        (Value::Integer(a), Value::Integer(b)) => *a == *b,
                        (Value::Float(a), Value::Float(b)) => *a == *b,
                        (Value::Integer(a), Value::Float(b)) => *a as f64 == *b,
                        (Value::Float(a), Value::Integer(b)) => *a == *b as f64,
                        (Value::Boolean(a), Value::Boolean(b)) => *a == *b,
                        (Value::Null, Value::Null) => true,
                        (Value::String(a), Value::String(b)) => **a == **b,
                        _ => false,
                    };
                    if are_equal { 1 } else { 0 }
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_subscript_int_imm(
    locals: *const Value,
    list_idx: u32,
    key_idx: u32,
    imm: i64,
) -> i64 {
    unsafe {
        let list_val = &*locals.add(list_idx as usize);
        let key_val = &*locals.add(key_idx as usize);
        match (list_val, key_val) {
            (Value::List(list), Value::Integer(key)) => {
                let idx = (*key + imm).max(0) as usize;
                if idx < list.len() {
                    match list.get_unchecked(idx) {
                        Value::Integer(v) => *v,
                        _ => 0,
                    }
                } else {
                    0
                }
            }
            _ => 0,
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_store_subscript(
    locals: *mut Value,
    list_idx: u32,
    key_idx: u32,
    val_idx: u32,
) {
    unsafe {
        let list_val = &*locals.add(list_idx as usize);
        let key_val = &*locals.add(key_idx as usize);
        let val_val = &*locals.add(val_idx as usize);
        if let (Value::List(list), Value::Integer(key)) = (list_val, key_val) {
            let idx = (*key).max(0) as usize;
            if idx < list.len() {
                list.set_unchecked(idx, val_val.clone());
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_store_subscript_int(
    locals: *mut Value,
    list_idx: u32,
    key_idx: u32,
    val: i64,
) {
    unsafe {
        let list_val = &*locals.add(list_idx as usize);
        let key_val = &*locals.add(key_idx as usize);
        if let (Value::List(list), Value::Integer(key)) = (list_val, key_val) {
            let idx = (*key).max(0) as usize;
            if idx < list.len() {
                list.set_unchecked(idx, Value::Integer(val));
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_store_subscript_add_imm(
    locals: *mut Value,
    list_idx: u32,
    key_idx: u32,
    imm: i64,
    val_idx: u32,
) {
    unsafe {
        let list_val = &*locals.add(list_idx as usize);
        let key_val = &*locals.add(key_idx as usize);
        let val_val = &*locals.add(val_idx as usize);
        if let (Value::List(list), Value::Integer(key)) = (list_val, key_val) {
            let idx = (*key + imm).max(0) as usize;
            if idx < list.len() {
                list.set_unchecked(idx, val_val.clone());
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_swap_adjacent(locals: *mut Value, list_idx: u32, key_idx: u32) {
    unsafe {
        let list_val = &*locals.add(list_idx as usize);
        let key_val = &*locals.add(key_idx as usize);
        if let (Value::List(list), Value::Integer(key)) = (list_val, key_val) {
            let idx = (*key).max(0) as usize;
            if idx + 1 < list.len() {
                list.swap_unchecked(idx, idx + 1);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_subscript_gt(
    locals: *const Value,
    list_idx: u32,
    key_idx: u32,
    imm: i64,
) -> i64 {
    let a = arabi_jit_subscript_int(locals, list_idx, key_idx);
    let b = arabi_jit_subscript_int_imm(locals, list_idx, key_idx, imm);
    if a > b {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_get_time() -> f64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

#[no_mangle]
pub extern "C" fn arabi_jit_print_int(val: i64) {
    print!("{}", val);
}

#[no_mangle]
pub extern "C" fn arabi_jit_print_float(val: f64) {
    print!("{}", val);
}

#[no_mangle]
pub extern "C" fn arabi_jit_print_str(locals: *const Value, idx: u32) {
    unsafe {
        let slot = &*locals.add(idx as usize);
        match slot {
            Value::String(s) => print!("{}", s),
            _ => print!("{:?}", slot),
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_print_sep() {
    print!(" ");
}

#[no_mangle]
pub extern "C" fn arabi_jit_print_newline() {
    println!();
}

#[no_mangle]
pub extern "C" fn arabi_jit_for_range_next(
    locals: *mut Value,
    iter_local: u32,
    idx_local: u32,
    target_local: u32,
) -> i64 {
    unsafe {
        let range_val = &*locals.add(iter_local as usize);
        let idx_val = &*locals.add(idx_local as usize);
        if let (Value::Range(range), Value::Integer(counter)) = (range_val, idx_val) {
            let current = range.start + counter * range.step;
            let in_range = if range.step > 0 {
                current < range.end
            } else if range.step < 0 {
                current > range.end
            } else {
                false
            };
            if in_range {
                let slot_target = &mut *locals.add(target_local as usize);
                *slot_target = Value::Integer(current);
                let slot_idx = &mut *locals.add(idx_local as usize);
                *slot_idx = Value::Integer(counter + 1);
                return current;
            }
        }
        -1
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_for_range_init(
    locals: *mut Value,
    iter_local: u32,
    idx_local: u32,
    target_local: u32,
) -> i64 {
    unsafe {
        let range_val = &*locals.add(iter_local as usize);
        if let Value::Range(range) = range_val {
            let slot_idx = &mut *locals.add(idx_local as usize);
            *slot_idx = Value::Integer(0);
            let slot_target = &mut *locals.add(target_local as usize);
            *slot_target = Value::Integer(range.start);
            return range.start;
        }
        -1
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_pop_jump_if_subscript_gt(
    locals: *const Value,
    list_idx: u32,
    key_idx: u32,
    imm: i64,
) -> i64 {
    let a = arabi_jit_subscript_int(locals, list_idx, key_idx);
    let b = arabi_jit_subscript_int_imm(locals, list_idx, key_idx, imm);
    if a > b { 1 } else { 0 }
}

#[no_mangle]
pub extern "C" fn arabi_jit_print_value(locals: *const Value, idx: u32) {
    unsafe {
        let slot = &*locals.add(idx as usize);
        match slot {
            Value::Integer(n) => print!("{}", n),
            Value::Float(f) => print!("{}", f),
            Value::String(s) => print!("{}", s),
            Value::Boolean(b) => print!("{}", b),
            Value::Null => print!("عدم"),
            _ => print!("{:?}", slot),
        }
    }
}

#[no_mangle]
pub extern "C" fn arabi_jit_store_name(
    _locals: *mut Value,
    _name_idx: u32,
    _val_idx: u32,
) {
    // For module-level StoreName - handled by VM fallback
}

#[no_mangle]
pub extern "C" fn arabi_jit_load_global(
    _globals: *mut std::ffi::c_void,
    _name_idx: u32,
) -> i64 {
    0
}

/// Create a class instance from JIT code.
/// name_idx: index into module.names for the class name
/// locals: pointer to the locals arena
/// result_slot: slot to store the result
#[no_mangle]
pub extern "C" fn arabi_jit_create_instance(
    name_idx: u32,
    locals: *mut Value,
    result_slot: u32,
) {
    let vm_ptr = VM_PTR.with(|c| c.get());
    let module_ptr = MODULE_PTR.with(|c| c.get());
    if vm_ptr.is_null() || module_ptr.is_null() { return; }
    unsafe {
        let vm = &*(vm_ptr as *const crate::vm::VM);
        let module = &*(module_ptr as *const arabi_compiler::bytecode::BytecodeModule);
        if let Some(name) = module.names.get(name_idx as usize) {
            if let Some(Value::Class(d)) = vm.globals.get(name) {
                let nf = d.field_names.len().min(4) as u8;
                let instance = Value::Instance(Rc::new(crate::frame::InstanceData {
                    class: Rc::clone(d),
                    inline_fields: std::cell::RefCell::new(core::array::from_fn(|_| Value::Null)),
                    num_inline_fields: nf,
                    extra_fields: std::cell::RefCell::new(None),
                }));
                *locals.add(result_slot as usize) = instance;
            }
        }
    }
}
#[no_mangle]
pub extern "C" fn arabi_jit_load_global_to_local(
    name_idx: u32,
    locals: *mut Value,
    slot: u32,
) {
    let vm_ptr = VM_PTR.with(|c| c.get());
    let module_ptr = MODULE_PTR.with(|c| c.get());
    if vm_ptr.is_null() || module_ptr.is_null() { return; }
    unsafe {
        let vm = &*(vm_ptr as *const crate::vm::VM);
        let module = &*(module_ptr as *const arabi_compiler::bytecode::BytecodeModule);
        if let Some(name) = module.names.get(name_idx as usize) {
            if let Some(val) = vm.globals.get(name) {
                *locals.add(slot as usize) = val.clone();
            }
        }
    }
}

/// Call any function (builtin or user-defined) from JIT code.
/// func_slot: locals index where the function Value is stored
/// first_arg_slot: locals index where the first arg is stored
/// argc: number of arguments
/// Returns: integer result (0 for non-integer results)
#[no_mangle]
pub extern "C" fn arabi_jit_call_func(
    locals: *const Value,
    func_slot: u32,
    first_arg_slot: u32,
    argc: u32,
) -> i64 {
    let vm_ptr = VM_PTR.with(|c| c.get());
    if vm_ptr.is_null() { return 0; }
    if locals.is_null() { return 0; }
    unsafe {
        let func_val = (*locals.add(func_slot as usize)).clone();
        let mut args: Vec<Value> = Vec::with_capacity(argc as usize);
        for i in 0..argc {
            args.push((*locals.add(first_arg_slot as usize + i as usize)).clone());
        }
        let vm = &mut *(vm_ptr as *mut crate::vm::VM);
        let last = vm.modules.len().saturating_sub(1);
        let module_ptr = vm.modules.as_mut_ptr().add(last);
        let func_ptr = &func_val as *const Value;
        match (*func_ptr).call(&args, &[], vm, &mut *module_ptr) {
            Ok(val) => match val {
                Value::Integer(n) => n,
                Value::Float(f) => f as i64,
                Value::Boolean(b) => if b { 1 } else { 0 },
                Value::Null => 0,
                _ => 0,
            },
            Err(e) => {
                vm.current_exception = Some(e.into_value());
                0
            }
        }
    }
}

/// Generic binary add: loads two Values from locals, adds them, stores result.
/// result_slot = locals[a_slot] + locals[b_slot]
#[no_mangle]
pub extern "C" fn arabi_jit_binary_add_generic(
    locals: *mut Value, a_slot: u32, b_slot: u32, result_slot: u32,
) {
    unsafe {
        let a = &*locals.add(a_slot as usize);
        let b = &*locals.add(b_slot as usize);
        let result = match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Value::Integer(x.wrapping_add(*y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(*x + *y),
            (Value::Integer(x), Value::Float(y)) => Value::Float(*x as f64 + *y),
            (Value::Float(x), Value::Integer(y)) => Value::Float(*x + *y as f64),
            (Value::String(x), Value::String(y)) => Value::String(format!("{}{}", x, y).into()),
            _ => {
                let a = a.clone();
                let b = b.clone();
                match (&a, &b) {
                    _ => Value::Null,
                }
            }
        };
        *locals.add(result_slot as usize) = result;
    }
}

/// Generic binary subtract
#[no_mangle]
pub extern "C" fn arabi_jit_binary_sub_generic(
    locals: *mut Value, a_slot: u32, b_slot: u32, result_slot: u32,
) {
    unsafe {
        let a = &*locals.add(a_slot as usize);
        let b = &*locals.add(b_slot as usize);
        let result = match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Value::Integer(x.wrapping_sub(*y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(*x - *y),
            (Value::Integer(x), Value::Float(y)) => Value::Float(*x as f64 - *y),
            (Value::Float(x), Value::Integer(y)) => Value::Float(*x - *y as f64),
            _ => Value::Null,
        };
        *locals.add(result_slot as usize) = result;
    }
}

/// Generic binary multiply
#[no_mangle]
pub extern "C" fn arabi_jit_binary_mul_generic(
    locals: *mut Value, a_slot: u32, b_slot: u32, result_slot: u32,
) {
    unsafe {
        let a = &*locals.add(a_slot as usize);
        let b = &*locals.add(b_slot as usize);
        let result = match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => Value::Integer(x.wrapping_mul(*y)),
            (Value::Float(x), Value::Float(y)) => Value::Float(*x * *y),
            (Value::Integer(x), Value::Float(y)) => Value::Float(*x as f64 * *y),
            (Value::Float(x), Value::Integer(y)) => Value::Float(*x * *y as f64),
            _ => Value::Null,
        };
        *locals.add(result_slot as usize) = result;
    }
}

/// Generic binary divide
#[no_mangle]
pub extern "C" fn arabi_jit_binary_div_generic(
    locals: *mut Value, a_slot: u32, b_slot: u32, result_slot: u32,
) {
    unsafe {
        let a = &*locals.add(a_slot as usize);
        let b = &*locals.add(b_slot as usize);
        let result = match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => {
                if *y == 0 { Value::Null } else { Value::Integer(x / y) }
            }
            (Value::Float(x), Value::Float(y)) => Value::Float(*x / *y),
            (Value::Integer(x), Value::Float(y)) => Value::Float(*x as f64 / *y),
            (Value::Float(x), Value::Integer(y)) => Value::Float(*x / *y as f64),
            _ => Value::Null,
        };
        *locals.add(result_slot as usize) = result;
    }
}

/// Generic binary modulo
#[no_mangle]
pub extern "C" fn arabi_jit_binary_mod_generic(
    locals: *mut Value, a_slot: u32, b_slot: u32, result_slot: u32,
) {
    unsafe {
        let a = &*locals.add(a_slot as usize);
        let b = &*locals.add(b_slot as usize);
        let result = match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => {
                if *y == 0 { Value::Null } else { Value::Integer(x % y) }
            }
            (Value::Float(x), Value::Float(y)) => Value::Float(*x % *y),
            (Value::Integer(x), Value::Float(y)) => Value::Float(*x as f64 % *y),
            (Value::Float(x), Value::Integer(y)) => Value::Float(*x % *y as f64),
            _ => Value::Null,
        };
        *locals.add(result_slot as usize) = result;
    }
}

/// Generic comparison: returns -1, 0, or 1
#[no_mangle]
pub extern "C" fn arabi_jit_compare_generic(
    locals: *const Value, a_slot: u32, b_slot: u32, cmp_op: u32,
) -> i64 {
    unsafe {
        let a = &*locals.add(a_slot as usize);
        let b = &*locals.add(b_slot as usize);
        let ord = match (a, b) {
            (Value::Integer(x), Value::Integer(y)) => x.cmp(y),
            (Value::Float(x), Value::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Integer(x), Value::Float(y)) => (*x as f64).partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
            (Value::Float(x), Value::Integer(y)) => x.partial_cmp(&(*y as f64)).unwrap_or(std::cmp::Ordering::Equal),
            (Value::String(x), Value::String(y)) => x.cmp(y),
            _ => std::cmp::Ordering::Equal,
        };
        let result = match cmp_op {
            0 => ord == std::cmp::Ordering::Equal,           // EQ
            1 => ord != std::cmp::Ordering::Equal,           // NOT_EQ
            2 => ord == std::cmp::Ordering::Less,            // LT
            3 => ord == std::cmp::Ordering::Greater,         // GT
            4 => ord != std::cmp::Ordering::Greater,         // LE
            5 => ord != std::cmp::Ordering::Less,            // GE
            _ => false,
        };
        if result { 1 } else { 0 }
    }
}

/// Get instance field by offset (inline field fast path)
#[no_mangle]
pub extern "C" fn arabi_jit_get_instance_field(
    locals: *mut Value, obj_slot: u32, field_offset: u32, result_slot: u32,
) {
    unsafe {
        let obj_val = &*locals.add(obj_slot as usize);
        if let Value::Instance(rc) = obj_val {
            let idx = field_offset as usize;
            if idx < rc.num_inline_fields as usize {
                let val = (*rc.inline_fields.as_ptr())[idx].clone();
                *locals.add(result_slot as usize) = val;
            } else if let Some(name) = rc.class.field_names.get(idx) {
                if let Some(val) = rc.extra_fields.borrow().as_ref().and_then(|f| f.get(name.as_str()).cloned()) {
                    *locals.add(result_slot as usize) = val;
                } else {
                    *locals.add(result_slot as usize) = Value::Null;
                }
            } else {
                *locals.add(result_slot as usize) = Value::Null;
            }
        } else {
            *locals.add(result_slot as usize) = Value::Null;
        }
    }
}

/// Set instance field by offset
#[no_mangle]
pub extern "C" fn arabi_jit_set_instance_field(
    locals: *mut Value, obj_slot: u32, field_offset: u32, val_slot: u32,
) {
    unsafe {
        let obj_val = &*locals.add(obj_slot as usize);
        let val_val = (*locals.add(val_slot as usize)).clone();
        if let Value::Instance(rc) = obj_val {
            let idx = field_offset as usize;
            if idx < rc.num_inline_fields as usize {
                (*rc.inline_fields.as_ptr())[idx] = val_val;
            } else if let Some(name) = rc.class.field_names.get(idx) {
                rc.extra_fields.borrow_mut().get_or_insert_with(|| Box::new(std::collections::HashMap::new())).insert(name.clone(), val_val);
            }
        }
    }
}

/// Store attribute: locals[obj_slot].set_attribute(name, locals[val_slot])
/// name is read from locals[name_slot] as a String value.
#[no_mangle]
pub extern "C" fn arabi_jit_store_attr(
    locals: *mut Value, obj_slot: u32, name_slot: u32, val_slot: u32,
) {
    unsafe {
        let obj_val = (*locals.add(obj_slot as usize)).clone();
        let name_val = &*locals.add(name_slot as usize);
        let val_val = (*locals.add(val_slot as usize)).clone();
        if let Value::Instance(_rc) = &obj_val {
            if let Value::String(name) = name_val {
                let mut mutable = (*locals.add(obj_slot as usize)).clone();
                mutable.set_attribute(name.to_string(), val_val);
            }
        }
    }
}

/// Get instance field by name (hash lookup)
#[no_mangle]
pub extern "C" fn arabi_jit_get_instance_field_by_name(
    locals: *mut Value, obj_slot: u32, name_idx: u32, module_names: *const Vec<String>,
    result_slot: u32,
) {
    unsafe {
        let obj_val = &*locals.add(obj_slot as usize);
        let module = &*module_names;
        if let Value::Instance(rc) = obj_val {
            if let Some(name) = module.get(name_idx as usize) {
                let val = rc.get_field(name).unwrap_or(Value::Null);
                *locals.add(result_slot as usize) = val;
            }
        }
    }
}

/// Get instance field by name read from a locals slot
#[no_mangle]
pub extern "C" fn arabi_jit_get_field_by_name_slot(
    locals: *mut Value, obj_slot: u32, name_slot: u32, result_slot: u32,
) {
    unsafe {
        let obj_val = &*locals.add(obj_slot as usize);
        let name_val = &*locals.add(name_slot as usize);
        if let Value::Instance(rc) = obj_val {
            if let Value::String(name) = name_val {
                let val = rc.get_field(name.as_str()).unwrap_or(Value::Null);
                *locals.add(result_slot as usize) = val;
            } else {
                *locals.add(result_slot as usize) = Value::Null;
            }
        } else {
            *locals.add(result_slot as usize) = Value::Null;
        }
    }
}

/// Call method: store result in result_slot (not return as i64)
/// Returns 1 on success, 0 on failure.
#[no_mangle]
pub extern "C" fn arabi_jit_call_method_to_slot(
    locals: *mut Value, obj_slot: u32, method_name_slot: u32,
    first_arg_slot: u32, argc: u32, result_slot: u32,
) -> i64 {
    let vm_ptr = VM_PTR.with(|c| c.get());
    if vm_ptr.is_null() { return 0; }
    unsafe {
        let method_name_ref = &*locals.add(method_name_slot as usize);
        let method_name: &str = match method_name_ref {
            Value::String(s) => s.as_str(),
            _ => return 0,
        };
        let obj_ref = &*locals.add(obj_slot as usize);
        if let Value::Instance(rc) = obj_ref {
            if let Some(method_val_ref) = rc.class.methods.get(method_name) {
                if let Value::Function(f) = method_val_ref {
                let vm = &mut *(vm_ptr as *mut crate::vm::VM);
                if let Some(jit_entry) = f.jit_entry.get() {
                    if let Some(module_ref) = vm.modules.last() {
                        if module_ref.packed.get(f.body).is_some() {
                            let num_locals = f.num_locals.max(1);
                            let arena_offset = vm.locals_arena.len();
                            let new_len = arena_offset + num_locals;
                            if new_len > vm.locals_arena.capacity() {
                                vm.locals_arena.resize(new_len, Value::Null);
                            } else {
                                vm.locals_arena.set_len(new_len);
                            }
                            let target_ptr = vm.locals_arena.as_mut_ptr().add(arena_offset);
                            if !f.closure.is_empty() {
                                for (idx, val) in f.closure.iter().cloned() {
                                    if idx < num_locals { *target_ptr.add(idx) = val; }
                                }
                            }
                            if !f.param_indices.is_empty() {
                                std::ptr::copy_nonoverlapping(locals.add(obj_slot as usize), target_ptr.add(f.param_indices[0]), 1);
                            }
                            for i in 0..argc as usize {
                                if i + 1 < f.param_indices.len() {
                                    let p = f.param_indices[i + 1];
                                    if p < num_locals {
                                        std::ptr::copy_nonoverlapping(locals.add(first_arg_slot as usize + i), target_ptr.add(p), 1);
                                    }
                                }
                            }
                            let func_ptr: extern "C" fn(i64) -> i64 = std::mem::transmute(jit_entry);
                            let result_i64 = func_ptr(target_ptr as i64);
                            if vm.current_exception.is_some() {
                                vm.locals_arena.truncate(arena_offset);
                                return 0;
                            }
                            std::ptr::copy_nonoverlapping(target_ptr.add(result_i64 as usize), locals.add(result_slot as usize), 1);
                            vm.locals_arena.truncate(arena_offset);
                            return 1;
                        }
                    }
                }
                }
                let mut args: Vec<Value> = Vec::with_capacity(argc as usize + 1);
                args.push(obj_ref.clone());
                for i in 0..argc {
                    args.push((*locals.add(first_arg_slot as usize + i as usize)).clone());
                }
                let vm = &mut *(vm_ptr as *mut crate::vm::VM);
                let last = vm.modules.len().saturating_sub(1);
                let module_ptr = vm.modules.as_mut_ptr().add(last);
                match method_val_ref.call(&args, &[], vm, &mut *module_ptr) {
                    Ok(val) => {
                        *locals.add(result_slot as usize) = val;
                        1
                    }
                    Err(e) => {
                        vm.current_exception = Some(e.into_value());
                        0
                    }
                }
            } else { 0 }
        } else { 0 }
    }
}

/// Call method and return result as i64 (for integer-only results)
#[no_mangle]
pub extern "C" fn arabi_jit_call_method_i64(
    locals: *mut Value, obj_slot: u32, method_name_slot: u32,
    first_arg_slot: u32, argc: u32, result_slot: u32,
) -> i64 {
    let vm_ptr = VM_PTR.with(|c| c.get());
    if vm_ptr.is_null() { return 0; }
    unsafe {
        let obj_val = (*locals.add(obj_slot as usize)).clone();
        let method_name_val = (*locals.add(method_name_slot as usize)).clone();
        let method_name = match &method_name_val {
            Value::String(s) => s.as_str().to_string(),
            _ => return 0,
        };
        if let Value::Instance(rc) = &obj_val {
            if let Some(method_val) = rc.class.methods.get(&method_name) {
                let mut args: Vec<Value> = Vec::with_capacity(argc as usize + 1);
                args.push(obj_val.clone());
                for i in 0..argc {
                    args.push((*locals.add(first_arg_slot as usize + i as usize)).clone());
                }
                let vm = &mut *(vm_ptr as *mut crate::vm::VM);
                let last = vm.modules.len().saturating_sub(1);
                let module_ptr = vm.modules.as_mut_ptr().add(last);
                match method_val.call(&args, &[], vm, &mut *module_ptr) {
                    Ok(val) => {
                        *locals.add(result_slot as usize) = val.clone();
                        match val {
                            Value::Integer(n) => n,
                            Value::Boolean(b) => if b { 1 } else { 0 },
                            _ => 0,
                        }
                    }
                    Err(e) => {
                        vm.current_exception = Some(e.into_value());
                        0
                    }
                }
            } else { 0 }
        } else { 0 }
    }
}

/// Generic subscript: locals[result_slot] = locals[list_slot][locals[key_slot]]
#[no_mangle]
pub extern "C" fn arabi_jit_subscript_generic(
    locals: *mut Value, list_slot: u32, key_slot: u32, result_slot: u32,
) {
    unsafe {
        let list_val = &*locals.add(list_slot as usize);
        let key_val = &*locals.add(key_slot as usize);
        let result = match (list_val, key_val) {
            (Value::List(list), Value::Integer(idx)) => {
                let borrow = list.borrow();
                let i = if *idx < 0 { borrow.len() as i64 + idx } else { *idx };
                if i >= 0 && (i as usize) < borrow.len() {
                    borrow[i as usize].clone()
                } else { Value::Null }
            }
            (Value::Dict(dict), _) => {
                dict.lookup(key_val).unwrap_or(Value::Null)
            }
            (Value::String(s), Value::Integer(idx)) => {
                let chars: Vec<char> = s.chars().collect();
                let i = if *idx < 0 { chars.len() as i64 + idx } else { *idx };
                if i >= 0 && (i as usize) < chars.len() {
                    Value::String(Rc::new(chars[i as usize].to_string()))
                } else { Value::Null }
            }
            _ => Value::Null,
        };
        *locals.add(result_slot as usize) = result;
    }
}

/// Store subscript: locals[list_slot][locals[key_slot]] = locals[val_slot]
#[no_mangle]
pub extern "C" fn arabi_jit_store_subscript_generic(
    locals: *mut Value, list_slot: u32, key_slot: u32, val_slot: u32,
) {
    unsafe {
        let list_val = &*locals.add(list_slot as usize);
        let key_val = &*locals.add(key_slot as usize);
        let val_val = &*locals.add(val_slot as usize);
        if let (Value::List(list), Value::Integer(idx)) = (list_val, key_val) {
            let mut borrow = list.borrow_mut();
            let i = if *idx < 0 { borrow.len() as i64 + idx } else { *idx };
            if i >= 0 && (i as usize) < borrow.len() {
                borrow[i as usize] = val_val.clone();
            }
        }
    }
}

/// Call method on instance: obj.method_name(args...)
/// method_name is read from locals[method_name_slot] as a String value.
/// Stores result in result_slot. Returns 1 on success, 0 on failure.
#[no_mangle]
pub extern "C" fn arabi_jit_call_method(
    locals: *mut Value, obj_slot: u32, method_name_slot: u32,
    first_arg_slot: u32, argc: u32, result_slot: u32,
) -> i64 {
    let vm_ptr = VM_PTR.with(|c| c.get());
    if vm_ptr.is_null() { return 0; }
    unsafe {
        let obj_val = (*locals.add(obj_slot as usize)).clone();
        let method_name_val = (*locals.add(method_name_slot as usize)).clone();
        let method_name = match &method_name_val {
            Value::String(s) => s.as_str().to_string(),
            _ => return 0,
        };
        if let Value::Instance(rc) = &obj_val {
            if let Some(method_val) = rc.class.methods.get(&method_name) {
                let mut args: Vec<Value> = Vec::with_capacity(argc as usize + 1);
                args.push(obj_val.clone());
                for i in 0..argc {
                    args.push((*locals.add(first_arg_slot as usize + i as usize)).clone());
                }
                let vm = &mut *(vm_ptr as *mut crate::vm::VM);
                let last = vm.modules.len().saturating_sub(1);
                let module_ptr = vm.modules.as_mut_ptr().add(last);
                match method_val.call(&args, &[], vm, &mut *module_ptr) {
                    Ok(val) => {
                        *locals.add(result_slot as usize) = val;
                        1
                    }
                    Err(e) => {
                        vm.current_exception = Some(e.into_value());
                        0
                    }
                }
            } else { 0 }
        } else { 0 }
    }
}

/// Store a string constant into a locals slot.
/// The pointer should point to a null-terminated UTF-8 string that remains valid
/// for the lifetime of this module (typically a &'static str).
#[no_mangle]
pub extern "C" fn arabi_jit_store_string(
    locals: *mut Value, idx: u32, ptr: *const u8, len: u32,
) {
    unsafe {
        let bytes = std::slice::from_raw_parts(ptr, len as usize);
        let s = String::from_utf8_unchecked(bytes.to_vec());
        *locals.add(idx as usize) = Value::String(Rc::new(s));
    }
}

/// Copy Value from src_slot to dst_slot (preserves type)
#[no_mangle]
pub extern "C" fn arabi_jit_store_value(locals: *mut Value, src_slot: u32, dst_slot: u32) {
    unsafe {
        std::ptr::copy_nonoverlapping(locals.add(src_slot as usize), locals.add(dst_slot as usize), 1);
    }
}

/// Call generic function (user-defined or builtin) from locals slot
/// Fast path: if the target function has a JIT entry, bypass the interpreter entirely
/// Stores result in result_slot. Returns 1 on success.
#[no_mangle]
pub extern "C" fn arabi_jit_call_func_to_slot(
    locals: *mut Value, func_slot: u32, first_arg_slot: u32,
    argc: u32, result_slot: u32,
) -> i64 {
    let vm_ptr = VM_PTR.with(|c| c.get());
    if vm_ptr.is_null() { return 0; }
    unsafe {
        let func_ref = &*locals.add(func_slot as usize);
        if let Value::Function(f) = func_ref {
            if let Some(jit_entry) = f.jit_entry.get() {
                if let Some(module_ref) = (&*vm_ptr.cast::<crate::vm::VM>()).modules.last() {
                    if module_ref.packed.get(f.body).is_some() {
                        let num_locals = f.num_locals.max(1);
                        let vm = &mut *(vm_ptr as *mut crate::vm::VM);
                        let arena_offset = vm.locals_arena.len();
                        vm.locals_arena.resize(arena_offset + num_locals, Value::Null);
                        let target_ptr = vm.locals_arena.as_mut_ptr().add(arena_offset);
                        if !f.closure.is_empty() {
                            for (idx, val) in f.closure.iter().cloned() {
                                if idx < num_locals { *target_ptr.add(idx) = val; }
                            }
                        }
                        for (i, &p) in f.param_indices.iter().enumerate() {
                            if i < argc as usize && p < num_locals {
                                std::ptr::copy_nonoverlapping(locals.add(first_arg_slot as usize + i), target_ptr.add(p), 1);
                            }
                        }
                        let func_ptr: extern "C" fn(i64) -> i64 = std::mem::transmute(jit_entry);
                        let result_i64 = func_ptr(target_ptr as i64);
                        if vm.current_exception.is_some() {
                            vm.locals_arena.truncate(arena_offset);
                            return 0;
                        }
                        std::ptr::copy_nonoverlapping(target_ptr.add(result_i64 as usize), locals.add(result_slot as usize), 1);
                        vm.locals_arena.truncate(arena_offset);
                        return 1;
                    }
                }
            }
        }
        let func_val = (*locals.add(func_slot as usize)).clone();
        let mut args: Vec<Value> = Vec::with_capacity(argc as usize);
        for i in 0..argc {
            args.push((*locals.add(first_arg_slot as usize + i as usize)).clone());
        }
        let vm = &mut *(vm_ptr as *mut crate::vm::VM);
        let last = vm.modules.len().saturating_sub(1);
        let module_ptr = vm.modules.as_mut_ptr().add(last);
        match func_val.call(&args, &[], vm, &mut *module_ptr) {
            Ok(val) => {
                *locals.add(result_slot as usize) = val;
                1
            }
            Err(e) => {
                vm.current_exception = Some(e.into_value());
                0
            }
        }
    }
}

/// Call function with 2 args (stack-allocated, no Vec)
/// Fast path: if the target function has a JIT entry, bypass the interpreter entirely
#[no_mangle]
pub extern "C" fn arabi_jit_call_func_2_to_slot(
    locals: *mut Value, func_slot: u32, arg0_slot: u32, arg1_slot: u32,
    result_slot: u32,
) -> i64 {
    let vm_ptr = VM_PTR.with(|c| c.get());
    if vm_ptr.is_null() { return 0; }
    unsafe {
        let func_ref = &*locals.add(func_slot as usize);
        if let Value::Function(f) = func_ref {
            if let Some(jit_entry) = f.jit_entry.get() {
                if let Some(module_ref) = (&*vm_ptr.cast::<crate::vm::VM>()).modules.last() {
                    if module_ref.packed.get(f.body).is_some() {
                        let num_locals = f.num_locals.max(1);
                        let vm = &mut *(vm_ptr as *mut crate::vm::VM);
                        let arena_offset = vm.locals_arena.len();
                        vm.locals_arena.resize(arena_offset + num_locals, Value::Null);
                        let target_ptr = vm.locals_arena.as_mut_ptr().add(arena_offset);
                        if !f.closure.is_empty() {
                            for (idx, val) in f.closure.iter().cloned() {
                                if idx < num_locals { *target_ptr.add(idx) = val; }
                            }
                        }
                        if !f.param_indices.is_empty() {
                            let p0 = f.param_indices[0];
                            if p0 < num_locals { std::ptr::copy_nonoverlapping(locals.add(arg0_slot as usize), target_ptr.add(p0), 1); }
                        }
                        if f.param_indices.len() > 1 {
                            let p1 = f.param_indices[1];
                            if p1 < num_locals { std::ptr::copy_nonoverlapping(locals.add(arg1_slot as usize), target_ptr.add(p1), 1); }
                        }
                        let func_ptr: extern "C" fn(i64) -> i64 = std::mem::transmute(jit_entry);
                        let result_i64 = func_ptr(target_ptr as i64);
                        if vm.current_exception.is_some() {
                            vm.locals_arena.truncate(arena_offset);
                            return 0;
                        }
                        std::ptr::copy_nonoverlapping(target_ptr.add(result_i64 as usize), locals.add(result_slot as usize), 1);
                        vm.locals_arena.truncate(arena_offset);
                        return 1;
                    }
                }
            }
        }
        let func_val2 = (*locals.add(func_slot as usize)).clone();
        let mut args: Vec<Value> = Vec::with_capacity(2);
        args.push((*locals.add(arg0_slot as usize)).clone());
        args.push((*locals.add(arg1_slot as usize)).clone());
        let vm = &mut *(vm_ptr as *mut crate::vm::VM);
        let last = vm.modules.len().saturating_sub(1);
        let module_ptr = vm.modules.as_mut_ptr().add(last);
        match func_val2.call(&args, &[], vm, &mut *module_ptr) {
            Ok(val) => {
                *locals.add(result_slot as usize) = val;
                1
            }
            Err(e) => {
                vm.current_exception = Some(e.into_value());
                0
            }
        }
    }
}

/// Call function with 3 args (stack-allocated, no Vec)
/// Fast path: if the target function has a JIT entry, bypass the interpreter entirely
#[no_mangle]
pub extern "C" fn arabi_jit_call_func_3_to_slot(
    locals: *mut Value, func_slot: u32, arg0_slot: u32, arg1_slot: u32, arg2_slot: u32,
    result_slot: u32,
) -> i64 {
    let vm_ptr = VM_PTR.with(|c| c.get());
    if vm_ptr.is_null() { return 0; }
    unsafe {
        let func_ref = &*locals.add(func_slot as usize);
        if let Value::Function(f) = func_ref {
            if let Some(jit_entry) = f.jit_entry.get() {
                if let Some(module_ref) = (&*vm_ptr.cast::<crate::vm::VM>()).modules.last() {
                    if module_ref.packed.get(f.body).is_some() {
                        let num_locals = f.num_locals.max(1);
                        let vm = &mut *(vm_ptr as *mut crate::vm::VM);
                        let arena_offset = vm.locals_arena.len();
                        vm.locals_arena.resize(arena_offset + num_locals, Value::Null);
                        let target_ptr = vm.locals_arena.as_mut_ptr().add(arena_offset);
                        if !f.closure.is_empty() {
                            for (idx, val) in f.closure.iter().cloned() {
                                if idx < num_locals { *target_ptr.add(idx) = val; }
                            }
                        }
                        if !f.param_indices.is_empty() {
                            let p0 = f.param_indices[0];
                            if p0 < num_locals { std::ptr::copy_nonoverlapping(locals.add(arg0_slot as usize), target_ptr.add(p0), 1); }
                        }
                        if f.param_indices.len() > 1 {
                            let p1 = f.param_indices[1];
                            if p1 < num_locals { std::ptr::copy_nonoverlapping(locals.add(arg1_slot as usize), target_ptr.add(p1), 1); }
                        }
                        if f.param_indices.len() > 2 {
                            let p2 = f.param_indices[2];
                            if p2 < num_locals { std::ptr::copy_nonoverlapping(locals.add(arg2_slot as usize), target_ptr.add(p2), 1); }
                        }
                        let func_ptr: extern "C" fn(i64) -> i64 = std::mem::transmute(jit_entry);
                        let result_i64 = func_ptr(target_ptr as i64);
                        if vm.current_exception.is_some() {
                            vm.locals_arena.truncate(arena_offset);
                            return 0;
                        }
                        std::ptr::copy_nonoverlapping(target_ptr.add(result_i64 as usize), locals.add(result_slot as usize), 1);
                        vm.locals_arena.truncate(arena_offset);
                        return 1;
                    }
                }
            }
        }
        let func_val2 = (*locals.add(func_slot as usize)).clone();
        let mut args: Vec<Value> = Vec::with_capacity(3);
        args.push((*locals.add(arg0_slot as usize)).clone());
        args.push((*locals.add(arg1_slot as usize)).clone());
        args.push((*locals.add(arg2_slot as usize)).clone());
        let vm = &mut *(vm_ptr as *mut crate::vm::VM);
        let last = vm.modules.len().saturating_sub(1);
        let module_ptr = vm.modules.as_mut_ptr().add(last);
        match func_val2.call(&args, &[], vm, &mut *module_ptr) {
            Ok(val) => {
                *locals.add(result_slot as usize) = val;
                1
            }
            Err(e) => {
                vm.current_exception = Some(e.into_value());
                0
            }
        }
    }
}
