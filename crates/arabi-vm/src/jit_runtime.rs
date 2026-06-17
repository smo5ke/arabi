use crate::frame::{Value, SharedList};
use std::cell::Cell;
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
            list.borrow_mut().push(val_val.clone());
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
                let borrow = list.borrow();
                let idx = (*key).max(0) as usize;
                if idx < borrow.len() {
                    match &borrow[idx] {
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
                let borrow = list.borrow();
                let idx = (*key + imm).max(0) as usize;
                if idx < borrow.len() {
                    match &borrow[idx] {
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
            let mut borrow = list.borrow_mut();
            let idx = (*key).max(0) as usize;
            if idx < borrow.len() {
                borrow[idx] = val_val.clone();
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
            let mut borrow = list.borrow_mut();
            let idx = (*key).max(0) as usize;
            if idx < borrow.len() {
                borrow[idx] = Value::Integer(val);
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
            let mut borrow = list.borrow_mut();
            let idx = (*key + imm).max(0) as usize;
            if idx < borrow.len() {
                borrow[idx] = val_val.clone();
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
            let mut borrow = list.borrow_mut();
            let idx = (*key).max(0) as usize;
            if idx + 1 < borrow.len() {
                borrow.swap(idx, idx + 1);
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

/// Load a global value by name_idx into a locals slot.
/// Called from JIT code when OP_LOAD_GLOBAL/OP_LOAD_NAME is encountered.
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

// Symbols registered by arabi-jit using the raw function pointers below
pub const JIT_SYMBOLS: &[(&str, extern "C" fn())] = &[];
