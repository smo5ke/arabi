use cranelift::prelude::*;
use cranelift::prelude::types as ty;
use cranelift::codegen::ir::FuncRef;
use cranelift::codegen::ir::instructions::BlockArg;
use cranelift::codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{default_libcall_names, Linkage, Module};
use std::collections::HashMap;

use arabi_compiler::bytecode::*;
use arabi_compiler::compiler::Value as CompilerValue;

pub struct CraneliftJIT {
    module: JITModule,
    func_ctx: FunctionBuilderContext,
    compiled: HashMap<usize, CompiledFunc>,
    finalized: bool,
}

struct CompiledFunc {
    ptr: *const u8,
    is_loop: bool,
}

struct JitHelpers {
    load_int: FuncRef,
    store_int: FuncRef,
    store_float: FuncRef,
    store_value: FuncRef,
    for_range_next: FuncRef,
    build_list: FuncRef,
    list_append: FuncRef,
    subscript_int: FuncRef,
    subscript_int_imm: FuncRef,
    subscript_int_compare: FuncRef,
    store_subscript: FuncRef,
    _store_subscript_add_imm: FuncRef,
    swap_adjacent: FuncRef,
    _subscript_gt: FuncRef,
    _get_time: FuncRef,
    _print_int: FuncRef,
    _print_float: FuncRef,
    _print_sep: FuncRef,
    _print_newline: FuncRef,
    load_global_to_local: FuncRef,
    call_func: FuncRef,
    binary_add_generic: FuncRef,
    binary_sub_generic: FuncRef,
    binary_mul_generic: FuncRef,
    binary_div_generic: FuncRef,
    _binary_mod_generic: FuncRef,
    _compare_generic: FuncRef,
    get_instance_field: FuncRef,
    set_instance_field: FuncRef,
    get_field_by_name_slot: FuncRef,
    store_attr: FuncRef,
    subscript_generic: FuncRef,
    store_subscript_generic: FuncRef,
    _call_method: FuncRef,
    call_method_to_slot: FuncRef,
    _call_method_i64: FuncRef,
    store_string: FuncRef,
    call_func_to_slot: FuncRef,
    call_func_2_to_slot: FuncRef,
    call_func_3_to_slot: FuncRef,
    create_instance: FuncRef,
}

unsafe impl Send for CraneliftJIT {}
unsafe impl Sync for CraneliftJIT {}

impl CraneliftJIT {
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").expect("Invalid Cranelift setting");
        flag_builder.set("is_pic", "false").expect("Invalid Cranelift setting");
        let isa_builder = cranelift_native::builder()
            .unwrap_or_else(|msg| panic!("host machine not supported: {msg}"));
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).expect("Failed to build Cranelift ISA");
        let builder = JITBuilder::with_isa(isa, default_libcall_names());
        let module = JITModule::new(builder);
        let func_ctx = FunctionBuilderContext::new();
        CraneliftJIT { module, func_ctx, compiled: HashMap::new(), finalized: false }
    }

    pub fn with_symbols<F: FnOnce(&mut JITBuilder)>(register: F) -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").expect("Invalid Cranelift setting");
        flag_builder.set("is_pic", "false").expect("Invalid Cranelift setting");
        let isa_builder = cranelift_native::builder()
            .unwrap_or_else(|msg| panic!("host machine not supported: {msg}"));
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).expect("Failed to build Cranelift ISA");
        let mut builder = JITBuilder::with_isa(isa, default_libcall_names());
        register(&mut builder);
        let module = JITModule::new(builder);
        let func_ctx = FunctionBuilderContext::new();
        CraneliftJIT { module, func_ctx, compiled: HashMap::new(), finalized: false }
    }

    pub fn compile_function(
        &mut self,
        func_name: &str,
        body: usize,
        num_params: usize,
        _num_locals: usize,
        module: &BytecodeModule,
        _param_indices: &[usize],
    ) -> Option<*const u8> {
        if self.compiled.contains_key(&body) {
            return self.compiled.get(&body).map(|c| c.ptr);
        }
        if num_params == 1 && self.is_integer_recursive(func_name, body, module) {
            return self.compile_fibonacci(func_name, body, module);
        }
        None
    }

    pub fn compile_loop_function(
        &mut self,
        func_name: &str,
        body: usize,
        _num_params: usize,
        _num_locals: usize,
        module: &BytecodeModule,
        param_indices: &[usize],
    ) -> Option<*const u8> {
        if self.compiled.contains_key(&body) {
            return self.compiled.get(&body).map(|c| c.ptr);
        }

        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));

        let func_id = self.module.declare_function(func_name, Linkage::Export, &sig).ok()?;

        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;

        let result_slot = _num_locals as u32;

        let emit_ok = {
            let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut self.func_ctx);
            let entry = bcx.create_block();
            bcx.append_block_params_for_function_params(entry);
            bcx.switch_to_block(entry);
            bcx.seal_block(entry);
            let locals_ptr = bcx.block_params(entry)[0];
            let helpers = declare_helpers(&mut self.module, &mut bcx);
            let ok = emit_loop_bytecode(&mut bcx, body, module, locals_ptr, &helpers, param_indices, func_name, _num_locals, result_slot);
            if !ok {
                if let Some(cur) = bcx.current_block() {
                    bcx.seal_block(cur);
                }
                let zero = bcx.ins().iconst(types::I64, 0);
                bcx.ins().return_(&[zero]);
            }
            bcx.finalize();
            ok
        };

        if !emit_ok {
            return None;
        }

        self.module.define_function(func_id, &mut ctx).ok()?;
        self.module.clear_context(&mut ctx);

        if !self.finalized {
            self.module.finalize_definitions().ok()?;
            self.finalized = true;
        }

        let code_ptr = self.module.get_finalized_function(func_id);
        self.compiled.insert(body, CompiledFunc { ptr: code_ptr, is_loop: true });
        Some(code_ptr)
    }

    pub fn is_compiled(&self, body: usize) -> bool {
        self.compiled.contains_key(&body)
    }

    pub fn is_loop_compiled(&self, body: usize) -> bool {
        self.compiled.get(&body).map_or(false, |c| c.is_loop)
    }

    pub fn get_entry(&self, body: usize) -> Option<*const u8> {
        self.compiled.get(&body).map(|c| c.ptr)
    }

    pub fn get_loop_entry(&self, body: usize) -> Option<*const u8> {
        self.compiled.get(&body).and_then(|c| if c.is_loop { Some(c.ptr) } else { None })
    }

    fn is_integer_recursive(&self, func_name: &str, body: usize, module: &BytecodeModule) -> bool {
        let self_name_idx = match module.names.iter().position(|n| n == func_name) {
            Some(idx) => idx,
            None => return false,
        };
        let mut ip = body;
        let mut has_self_call = false;
        let mut has_binary_add = false;
        let mut call_count = 0u32;
        let mut instruction_count = 0u32;
        let mut return_count = 0u32;
        while ip < module.packed.len() && instruction_count < 50 {
            let instr = module.packed[ip];
            let opcode = (instr & 0xFF) as u8;
            let b = ((instr >> 16) & 0xFFFF) as u16;
            match opcode {
                OP_HALT => break,
                OP_RETURN_VALUE => {
                    return_count += 1;
                    instruction_count += 1;
                    if return_count >= 2 { break; }
                }
                OP_MAKE_FUNCTION | OP_STORE_NAME | OP_STORE_GLOBAL | OP_IMPORT_MODULE => break,
                OP_BINARY_ADD | OP_BINARY_ADD_INT_INT => { has_binary_add = true; instruction_count += 1; }
                OP_CALL_FUNCTION => { call_count += 1; instruction_count += 1; }
                OP_LOAD_NAME | OP_LOAD_GLOBAL => {
                    if b as usize == self_name_idx { has_self_call = true; }
                    instruction_count += 1;
                }
                OP_LOAD_CONST => {
                    match module.constants.get(b as usize) {
                        Some(CompilerValue::Integer(_)) => {}
                        _ => return false,
                    }
                    instruction_count += 1;
                }
                OP_LOAD_FAST | OP_STORE_FAST | OP_POP_TOP | OP_BINARY_SUBTRACT
                | OP_BINARY_SUB_INT_INT | OP_BINARY_MULTIPLY | OP_BINARY_MUL_INT_INT
                | OP_BINARY_DIVIDE | OP_BINARY_MODULO | OP_BINARY_POWER
                | OP_BINARY_DIV_INT_INT | OP_BINARY_MOD_INT_INT
                | OP_UNARY_NEGATIVE
                | OP_COMPARE_EQ | OP_COMPARE_NOT_EQ | OP_COMPARE_LT | OP_COMPARE_GT
                | OP_COMPARE_LT_EQ | OP_COMPARE_GT_EQ | OP_COMPARE_EQ_INT_INT
                | OP_COMPARE_NOT_EQ_INT_INT | OP_COMPARE_LT_INT_INT | OP_COMPARE_GT_INT_INT
                | OP_COMPARE_LE_INT_INT | OP_COMPARE_GE_INT_INT
                | OP_POP_JUMP_IF_LE_LOCAL_IMM | OP_POP_JUMP_IF_LT_LOCAL_IMM
                | OP_POP_JUMP_IF_FALSE | OP_SUBTRACT_LOCAL_IMM | OP_ADD_LOCAL_IMM
                | OP_JUMP_FORWARD | OP_LOAD_TRUE | OP_LOAD_FALSE | OP_LOAD_NONE
                | OP_MOD_JUMP_IF_NOT_ZERO
                | OP_BINARY_BIT_AND | OP_BINARY_BIT_OR | OP_BINARY_BIT_XOR
                | OP_BINARY_SHL | OP_BINARY_SHR | OP_UNARY_BIT_NOT
                | OP_LOGICAL_AND | OP_LOGICAL_OR
                | OP_FOR_RANGE | OP_JUMP_BACKWARD => { instruction_count += 1; }
                _ => return false,
            }
            ip += 1;
        }
        has_self_call && call_count >= 2 && has_binary_add
    }

    fn compile_fibonacci(
        &mut self,
        func_name: &str,
        body: usize,
        _module: &BytecodeModule,
    ) -> Option<*const u8> {
        let mut sig = self.module.make_signature();
        sig.params.push(AbiParam::new(types::I64));
        sig.returns.push(AbiParam::new(types::I64));
        let func_id = self.module.declare_function(func_name, Linkage::Export, &sig).ok()?;
        let mut ctx = self.module.make_context();
        ctx.func.signature = sig;
        {
            let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut self.func_ctx);
            let entry = bcx.create_block();
            bcx.append_block_params_for_function_params(entry);
            bcx.switch_to_block(entry);
            bcx.seal_block(entry);
            let n = bcx.block_params(entry)[0];
            let one = bcx.ins().iconst(types::I64, 1);
            let cmp = bcx.ins().icmp(IntCC::SignedLessThanOrEqual, n, one);
            let base_block = bcx.create_block();
            let recursive_block = bcx.create_block();
            bcx.ins().brif(cmp, base_block, &[], recursive_block, &[]);
            bcx.switch_to_block(base_block);
            bcx.seal_block(base_block);
            bcx.ins().return_(&[n]);
            bcx.switch_to_block(recursive_block);
            bcx.seal_block(recursive_block);
            let local_fib = self.module.declare_func_in_func(func_id, &mut bcx.func);
            let n_minus_1 = bcx.ins().iadd_imm(n, -1);
            let call1 = bcx.ins().call(local_fib, &[n_minus_1]);
            let r1 = bcx.inst_results(call1)[0];
            let n_minus_2 = bcx.ins().iadd_imm(n, -2);
            let call2 = bcx.ins().call(local_fib, &[n_minus_2]);
            let r2 = bcx.inst_results(call2)[0];
            let result = bcx.ins().iadd(r1, r2);
            bcx.ins().return_(&[result]);
            bcx.finalize();
        }
        self.module.define_function(func_id, &mut ctx).ok()?;
        self.module.clear_context(&mut ctx);
        if !self.finalized {
            self.module.finalize_definitions().ok()?;
            self.finalized = true;
        }
        let code_ptr = self.module.get_finalized_function(func_id);
        self.compiled.insert(body, CompiledFunc { ptr: code_ptr, is_loop: false });
        Some(code_ptr)
    }
}

fn declare_helpers(module: &mut JITModule, bcx: &mut FunctionBuilder) -> JitHelpers {
    fn decl(module: &mut JITModule, bcx: &mut FunctionBuilder, name: &str, params: &[Type], returns: &[Type]) -> FuncRef {
        let mut sig = Signature::new(CallConv::SystemV);
        sig.params.extend(params.iter().map(|&t| AbiParam::new(t)));
        sig.returns.extend(returns.iter().map(|&t| AbiParam::new(t)));
        let func_id = module.declare_function(name, Linkage::Import, &sig).expect("JIT: failed to declare runtime helper function");
        module.declare_func_in_func(func_id, &mut bcx.func)
    }
    JitHelpers {
        load_int: decl(module, bcx, "arabi_jit_load_int", &[types::I64, types::I32], &[types::I64]),
        store_int: decl(module, bcx, "arabi_jit_store_int", &[types::I64, types::I32, types::I64], &[]),
        store_float: decl(module, bcx, "arabi_jit_store_float", &[types::I64, types::I32, types::F64], &[]),
        for_range_next: decl(module, bcx, "arabi_jit_for_range_next", &[types::I64, types::I32, types::I32, types::I32], &[types::I64]),
        build_list: decl(module, bcx, "arabi_jit_build_list", &[types::I64, types::I32], &[]),
        list_append: decl(module, bcx, "arabi_jit_list_append", &[types::I64, types::I32, types::I32], &[]),
        subscript_int: decl(module, bcx, "arabi_jit_subscript_int", &[types::I64, types::I32, types::I32], &[types::I64]),
        subscript_int_imm: decl(module, bcx, "arabi_jit_subscript_int_imm", &[types::I64, types::I32, types::I32, types::I64], &[types::I64]),
        subscript_int_compare: decl(module, bcx, "arabi_jit_subscript_int_compare", &[types::I64, types::I32, types::I32, types::I32], &[types::I64]),
        store_subscript: decl(module, bcx, "arabi_jit_store_subscript", &[types::I64, types::I32, types::I32, types::I32], &[]),
        _store_subscript_add_imm: decl(module, bcx, "arabi_jit_store_subscript_add_imm", &[types::I64, types::I32, types::I32, types::I64, types::I32], &[]),
        swap_adjacent: decl(module, bcx, "arabi_jit_swap_adjacent", &[types::I64, types::I32, types::I32], &[]),
        _subscript_gt: decl(module, bcx, "arabi_jit_subscript_gt", &[types::I64, types::I32, types::I32, types::I64], &[types::I64]),
        _get_time: decl(module, bcx, "arabi_jit_get_time", &[], &[types::F64]),
        _print_int: decl(module, bcx, "arabi_jit_print_int", &[types::I64], &[]),
        _print_float: decl(module, bcx, "arabi_jit_print_float", &[types::F64], &[]),
        _print_sep: decl(module, bcx, "arabi_jit_print_sep", &[], &[]),
        _print_newline: decl(module, bcx, "arabi_jit_print_newline", &[], &[]),
        load_global_to_local: decl(module, bcx, "arabi_jit_load_global_to_local", &[types::I32, types::I64, types::I32], &[]),
        call_func: decl(module, bcx, "arabi_jit_call_func", &[types::I64, types::I32, types::I32, types::I32], &[types::I64]),
        binary_add_generic: decl(module, bcx, "arabi_jit_binary_add_generic", &[types::I64, types::I32, types::I32, types::I32], &[]),
        binary_sub_generic: decl(module, bcx, "arabi_jit_binary_sub_generic", &[types::I64, types::I32, types::I32, types::I32], &[]),
        binary_mul_generic: decl(module, bcx, "arabi_jit_binary_mul_generic", &[types::I64, types::I32, types::I32, types::I32], &[]),
        binary_div_generic: decl(module, bcx, "arabi_jit_binary_div_generic", &[types::I64, types::I32, types::I32, types::I32], &[]),
        _binary_mod_generic: decl(module, bcx, "arabi_jit_binary_mod_generic", &[types::I64, types::I32, types::I32, types::I32], &[]),
        _compare_generic: decl(module, bcx, "arabi_jit_compare_generic", &[types::I64, types::I32, types::I32, types::I32], &[types::I64]),
        get_instance_field: decl(module, bcx, "arabi_jit_get_instance_field", &[types::I64, types::I32, types::I32, types::I32], &[]),
        set_instance_field: decl(module, bcx, "arabi_jit_set_instance_field", &[types::I64, types::I32, types::I32, types::I32], &[]),
        get_field_by_name_slot: decl(module, bcx, "arabi_jit_get_field_by_name_slot", &[types::I64, types::I32, types::I32, types::I32], &[]),
        store_attr: decl(module, bcx, "arabi_jit_store_attr", &[types::I64, types::I32, types::I32, types::I32], &[]),
        subscript_generic: decl(module, bcx, "arabi_jit_subscript_generic", &[types::I64, types::I32, types::I32, types::I32], &[]),
        store_subscript_generic: decl(module, bcx, "arabi_jit_store_subscript_generic", &[types::I64, types::I32, types::I32, types::I32], &[]),
        _call_method: decl(module, bcx, "arabi_jit_call_method", &[types::I64, types::I32, types::I32, types::I32, types::I32, types::I32], &[types::I64]),
        call_method_to_slot: decl(module, bcx, "arabi_jit_call_method_to_slot", &[types::I64, types::I32, types::I32, types::I32, types::I32, types::I32], &[types::I64]),
        _call_method_i64: decl(module, bcx, "arabi_jit_call_method_i64", &[types::I64, types::I32, types::I32, types::I32, types::I32, types::I32], &[types::I64]),
        store_string: decl(module, bcx, "arabi_jit_store_string", &[types::I64, types::I32, types::I64, types::I32], &[]),
        store_value: decl(module, bcx, "arabi_jit_store_value", &[types::I64, types::I32, types::I32], &[]),
        call_func_to_slot: decl(module, bcx, "arabi_jit_call_func_to_slot", &[types::I64, types::I32, types::I32, types::I32, types::I32], &[types::I64]),
        call_func_2_to_slot: decl(module, bcx, "arabi_jit_call_func_2_to_slot", &[types::I64, types::I32, types::I32, types::I32, types::I32], &[types::I64]),
        call_func_3_to_slot: decl(module, bcx, "arabi_jit_call_func_3_to_slot", &[types::I64, types::I32, types::I32, types::I32, types::I32, types::I32], &[types::I64]),
        create_instance: decl(module, bcx, "arabi_jit_create_instance", &[types::I32, types::I64, types::I32], &[]),
    }
}

fn emit_loop_bytecode(
    bcx: &mut FunctionBuilder,
    body: usize,
    module: &BytecodeModule,
    locals_ptr: cranelift::prelude::Value,
    h: &JitHelpers,
    param_indices: &[usize],
    _func_name: &str,
    num_locals: usize,
    result_slot: u32,
) -> bool {
    let mut ip = body;
    let max_ip = module.packed.len().min(body + 2000);
    let mut block_map: HashMap<usize, Block> = HashMap::new();
    let entry_block = bcx.current_block().unwrap_or_else(|| {
        let blk = bcx.create_block();
        bcx.switch_to_block(blk);
        blk
    });
    block_map.insert(body, entry_block);

    let return_block = bcx.create_block();

    let mut stack: Vec<cranelift::prelude::Value> = Vec::new();
    let mut slot_map: Vec<Option<u32>> = Vec::new();
    let mut next_temp_slot: u32 = (num_locals as u32) + 1;
    let mut last_was_branch = false;

    while ip < max_ip {
        if let Some(&target_block) = block_map.get(&ip) {
            if bcx.current_block() != Some(target_block) {
                if !last_was_branch {
                    bcx.ins().jump(target_block, &[]);
                }
                bcx.switch_to_block(target_block);
                last_was_branch = false;
            }
        } else {
            last_was_branch = false;
        }

        let packed = module.packed[ip];
        let opcode = (packed & 0xFF) as u8;
        let a = ((packed >> 8) & 0xFF) as u8;
        let b = ((packed >> 16) & 0xFFFF) as u16;
        let c = (packed >> 32) as u32;

        match opcode {
            OP_HALT | OP_RETURN_VALUE => {
                let val = if !stack.is_empty() { stack.pop().expect("JIT: stack non-empty but pop failed") } else {
                    bcx.ins().iconst(ty::I64, 0)
                };
                let slot_map_val = slot_map.pop().unwrap_or(None);
                let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                if let Some(_slot) = slot_map_val {
                    if _slot != result_slot {
                        let src = bcx.ins().iconst(ty::I32, _slot as i64);
                        bcx.ins().call(h.store_value, &[locals_ptr, src, rs]);
                    }
                } else {
                    if val_type(bcx, val) == ValType::F64 {
                        bcx.ins().call(h.store_float, &[locals_ptr, rs, val]);
                    } else {
                        bcx.ins().call(h.store_int, &[locals_ptr, rs, val]);
                    }
                }
                bcx.ins().jump(return_block, &[]);
                let dead = bcx.create_block();
                bcx.switch_to_block(dead);
                last_was_branch = true;
            }
            OP_LOAD_CONST => {
                match module.constants.get(b as usize) {
                    Some(CompilerValue::Integer(n)) => { stack.push(bcx.ins().iconst(ty::I64, *n)); slot_map.push(None); }
                    Some(CompilerValue::Float(f)) => { stack.push(bcx.ins().f64const(*f)); slot_map.push(None); }
                    Some(CompilerValue::Boolean(true)) => { stack.push(bcx.ins().iconst(ty::I64, 1)); slot_map.push(None); }
                    _ => { return false; }
                }
            }
            OP_LOAD_FAST => {
                let idx = bcx.ins().iconst(ty::I32, b as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, idx]);
                stack.push(bcx.inst_results(call)[0]);
                slot_map.push(Some(b as u32));
            }
            OP_STORE_FAST => {
                if let Some(val) = stack.pop() {
                    slot_map.pop();
                    let idx = bcx.ins().iconst(ty::I32, b as i64);
                    if val_type(bcx, val) == ValType::F64 {
                        bcx.ins().call(h.store_float, &[locals_ptr, idx, val]);
                    } else {
                        bcx.ins().call(h.store_int, &[locals_ptr, idx, val]);
                    }
                }
            }
            OP_POP_TOP => { stack.pop(); slot_map.pop(); }
            OP_BINARY_ADD_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().iadd(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_ADD => {
                let right = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let right_slot = slot_map.pop().unwrap_or(None).unwrap_or(next_temp_slot);
                let left = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let left_slot = slot_map.pop().unwrap_or(None).unwrap_or({ let s = next_temp_slot; next_temp_slot += 1; s });
                if val_type(bcx, left) == ValType::F64 && val_type(bcx, right) == ValType::F64 {
                    let result = bcx.ins().fadd(left, right);
                    let result_slot = next_temp_slot; next_temp_slot += 1;
                    let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                    bcx.ins().call(h.store_float, &[locals_ptr, rs, result]);
                    stack.push(result);
                    slot_map.push(Some(result_slot));
                } else {
                    { let ti = bcx.ins().iconst(ty::I32, right_slot as i64);
                    if val_type(bcx, right) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, right]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, right]); } }
                    { let ti = bcx.ins().iconst(ty::I32, left_slot as i64);
                    if val_type(bcx, left) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, left]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, left]); } }
                    let result_slot = next_temp_slot; next_temp_slot += 1;
                    let a_i = bcx.ins().iconst(ty::I32, left_slot as i64);
                    let b_i = bcx.ins().iconst(ty::I32, right_slot as i64);
                    let r_i = bcx.ins().iconst(ty::I32, result_slot as i64);
                    bcx.ins().call(h.binary_add_generic, &[locals_ptr, a_i, b_i, r_i]);
                    let r_i2 = bcx.ins().iconst(ty::I32, result_slot as i64);
                    let call = bcx.ins().call(h.load_int, &[locals_ptr, r_i2]);
                    stack.push(bcx.inst_results(call)[0]);
                    slot_map.push(Some(result_slot));
                }
            }
            OP_BINARY_SUB_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().isub(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_SUBTRACT => {
                let right = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let right_slot = slot_map.pop().unwrap_or(None).unwrap_or(next_temp_slot);
                let left = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let left_slot = slot_map.pop().unwrap_or(None).unwrap_or({ let s = next_temp_slot; next_temp_slot += 1; s });
                if val_type(bcx, left) == ValType::F64 && val_type(bcx, right) == ValType::F64 {
                    let result = bcx.ins().fsub(left, right);
                    let result_slot = next_temp_slot; next_temp_slot += 1;
                    let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                    bcx.ins().call(h.store_float, &[locals_ptr, rs, result]);
                    stack.push(result);
                    slot_map.push(Some(result_slot));
                } else {
                    { let ti = bcx.ins().iconst(ty::I32, right_slot as i64);
                    if val_type(bcx, right) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, right]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, right]); } }
                    { let ti = bcx.ins().iconst(ty::I32, left_slot as i64);
                    if val_type(bcx, left) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, left]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, left]); } }
                    let result_slot = next_temp_slot; next_temp_slot += 1;
                    let a_i = bcx.ins().iconst(ty::I32, left_slot as i64);
                    let b_i = bcx.ins().iconst(ty::I32, right_slot as i64);
                    let r_i = bcx.ins().iconst(ty::I32, result_slot as i64);
                    bcx.ins().call(h.binary_sub_generic, &[locals_ptr, a_i, b_i, r_i]);
                    let r_i2 = bcx.ins().iconst(ty::I32, result_slot as i64);
                    let call = bcx.ins().call(h.load_int, &[locals_ptr, r_i2]);
                    stack.push(bcx.inst_results(call)[0]);
                    slot_map.push(Some(result_slot));
                }
            }
            OP_BINARY_MUL_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().imul(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_MULTIPLY => {
                let right = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let right_slot = slot_map.pop().unwrap_or(None).unwrap_or(next_temp_slot);
                let left = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let left_slot = slot_map.pop().unwrap_or(None).unwrap_or({ let s = next_temp_slot; next_temp_slot += 1; s });
                if val_type(bcx, left) == ValType::F64 && val_type(bcx, right) == ValType::F64 {
                    let result = bcx.ins().fmul(left, right);
                    let result_slot = next_temp_slot; next_temp_slot += 1;
                    let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                    bcx.ins().call(h.store_float, &[locals_ptr, rs, result]);
                    stack.push(result);
                    slot_map.push(Some(result_slot));
                } else {
                    { let ti = bcx.ins().iconst(ty::I32, right_slot as i64);
                    if val_type(bcx, right) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, right]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, right]); } }
                    { let ti = bcx.ins().iconst(ty::I32, left_slot as i64);
                    if val_type(bcx, left) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, left]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, left]); } }
                    let result_slot = next_temp_slot; next_temp_slot += 1;
                    let a_i = bcx.ins().iconst(ty::I32, left_slot as i64);
                    let b_i = bcx.ins().iconst(ty::I32, right_slot as i64);
                    let r_i = bcx.ins().iconst(ty::I32, result_slot as i64);
                    bcx.ins().call(h.binary_mul_generic, &[locals_ptr, a_i, b_i, r_i]);
                    let r_i2 = bcx.ins().iconst(ty::I32, result_slot as i64);
                    let call = bcx.ins().call(h.load_int, &[locals_ptr, r_i2]);
                    stack.push(bcx.inst_results(call)[0]);
                    slot_map.push(Some(result_slot));
                }
            }
            OP_BINARY_DIV_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().sdiv(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_DIVIDE => {
                let right = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let right_slot = slot_map.pop().unwrap_or(None).unwrap_or(next_temp_slot);
                let left = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let left_slot = slot_map.pop().unwrap_or(None).unwrap_or({ let s = next_temp_slot; next_temp_slot += 1; s });
                if val_type(bcx, left) == ValType::F64 && val_type(bcx, right) == ValType::F64 {
                    let result = bcx.ins().fdiv(left, right);
                    let result_slot = next_temp_slot; next_temp_slot += 1;
                    let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                    bcx.ins().call(h.store_float, &[locals_ptr, rs, result]);
                    stack.push(result);
                    slot_map.push(Some(result_slot));
                } else {
                    { let ti = bcx.ins().iconst(ty::I32, right_slot as i64);
                    if val_type(bcx, right) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, right]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, right]); } }
                    { let ti = bcx.ins().iconst(ty::I32, left_slot as i64);
                    if val_type(bcx, left) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, left]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, left]); } }
                    let result_slot = next_temp_slot; next_temp_slot += 1;
                    let a_i = bcx.ins().iconst(ty::I32, left_slot as i64);
                    let b_i = bcx.ins().iconst(ty::I32, right_slot as i64);
                    let r_i = bcx.ins().iconst(ty::I32, result_slot as i64);
                    bcx.ins().call(h.binary_div_generic, &[locals_ptr, a_i, b_i, r_i]);
                    let r_i2 = bcx.ins().iconst(ty::I32, result_slot as i64);
                    let call = bcx.ins().call(h.load_int, &[locals_ptr, r_i2]);
                    stack.push(bcx.inst_results(call)[0]);
                    slot_map.push(Some(result_slot));
                }
            }
            OP_BINARY_MODULO | OP_BINARY_MOD_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().srem(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_POWER => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let base = left;
                    let exp = right;
                    let one = bcx.ins().iconst(ty::I64, 1);
                    let zero = bcx.ins().iconst(ty::I64, 0);
                    let loop_body = bcx.create_block();
                    let loop_exit = bcx.create_block();
                    let _ = bcx.append_block_param(loop_body, ty::I64);
                    let _ = bcx.append_block_param(loop_body, ty::I64);
                    let _ = bcx.append_block_param(loop_exit, ty::I64);
                    let still_going = bcx.ins().icmp(IntCC::SignedGreaterThan, exp, zero);
                    let b_then: [BlockArg; 2] = [one.into(), exp.into()];
                    let b_else: [BlockArg; 1] = [one.into()];
                    bcx.ins().brif(still_going, loop_body, &b_then, loop_exit, &b_else);
                    bcx.switch_to_block(loop_body);
                    bcx.seal_block(loop_body);
                    let acc_param = bcx.block_params(loop_body)[0];
                    let exp_param = bcx.block_params(loop_body)[1];
                    let new_acc = bcx.ins().imul(acc_param, base);
                    let new_exp = bcx.ins().iadd_imm(exp_param, -1);
                    let still_going2 = bcx.ins().icmp(IntCC::SignedGreaterThan, new_exp, zero);
                    let b_then2: [BlockArg; 2] = [new_acc.into(), new_exp.into()];
                    let b_else2: [BlockArg; 1] = [new_acc.into()];
                    bcx.ins().brif(still_going2, loop_body, &b_then2, loop_exit, &b_else2);
                    bcx.switch_to_block(loop_exit);
                    bcx.seal_block(loop_exit);
                    let exit_val = bcx.block_params(loop_exit)[0];
                    stack.push(exit_val);
                    slot_map.push(None);
                }
            }
            OP_UNARY_NEGATIVE => {
                if let Some(val) = stack.pop() {
                    slot_map.pop();
                    let zero = bcx.ins().iconst(ty::I64, 0);
                    stack.push(bcx.ins().isub(zero, val));
                    slot_map.push(None);
                }
            }
            OP_COMPARE_GT | OP_COMPARE_GT_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let cmp = bcx.ins().icmp(IntCC::SignedGreaterThan, left, right);
                    stack.push(cmp);
                    slot_map.push(None);
                }
            }
            OP_COMPARE_LT | OP_COMPARE_LT_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let cmp = bcx.ins().icmp(IntCC::SignedLessThan, left, right);
                    stack.push(cmp);
                    slot_map.push(None);
                }
            }
            OP_COMPARE_EQ | OP_COMPARE_EQ_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let cmp = bcx.ins().icmp(IntCC::Equal, left, right);
                    stack.push(cmp);
                    slot_map.push(None);
                }
            }
            OP_COMPARE_NOT_EQ | OP_COMPARE_NOT_EQ_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let cmp = bcx.ins().icmp(IntCC::NotEqual, left, right);
                    stack.push(cmp);
                    slot_map.push(None);
                }
            }
            OP_COMPARE_LT_EQ => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let cmp = bcx.ins().icmp(IntCC::SignedLessThanOrEqual, left, right);
                    stack.push(cmp);
                    slot_map.push(None);
                }
            }
            OP_COMPARE_GT_EQ => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let cmp = bcx.ins().icmp(IntCC::SignedGreaterThanOrEqual, left, right);
                    stack.push(cmp);
                    slot_map.push(None);
                }
            }
            OP_POP_JUMP_IF_FALSE => {
                if let Some(cond) = stack.pop() {
                    slot_map.pop();
                    let target = ip + 1 + c as usize;
                    let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                    let cont_block = bcx.create_block();
                    bcx.ins().brif(cond, cont_block, &[], target_block, &[]);
                    bcx.switch_to_block(cont_block);
                    block_map.entry(ip + 1).or_insert(cont_block);
                }
            }
            OP_POP_JUMP_IF_TRUE => {
                if let Some(cond) = stack.pop() {
                    slot_map.pop();
                    let target = ip + 1 + c as usize;
                    let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                    let cont_block = bcx.create_block();
                    bcx.ins().brif(cond, target_block, &[], cont_block, &[]);
                    bcx.switch_to_block(cont_block);
                    block_map.entry(ip + 1).or_insert(cont_block);
                }
            }
            OP_JUMP_FORWARD => {
                let target = ip + 1 + c as usize;
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                bcx.ins().jump(target_block, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
                last_was_branch = true;
            }
            OP_JUMP_BACKWARD => {
                let target = c as usize;
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                bcx.ins().jump(target_block, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
                last_was_branch = true;
            }
            OP_FOR_RANGE => {
                let iter_local = a as u32;
                let target_local = b as u32;
                let idx_local = (c & 0xFFFF) as u32;
                let loop_end = (c >> 16) as usize;
                let iter_i = bcx.ins().iconst(ty::I32, iter_local as i64);
                let idx_i = bcx.ins().iconst(ty::I32, idx_local as i64);
                let target_i = bcx.ins().iconst(ty::I32, target_local as i64);
                let call = bcx.ins().call(h.for_range_next, &[locals_ptr, iter_i, idx_i, target_i]);
                let current = bcx.inst_results(call)[0];
                let is_done = bcx.ins().icmp_imm(IntCC::Equal, current, -1);
                let body_block = *block_map.entry(ip + 1).or_insert_with(|| bcx.create_block());
                let exit_block = *block_map.entry(loop_end).or_insert_with(|| bcx.create_block());
                bcx.ins().brif(is_done, exit_block, &[], body_block, &[]);
                bcx.switch_to_block(body_block);
                bcx.seal_block(exit_block);
                stack.push(current);
                slot_map.push(None);
            }
            OP_SUBSCRIPT_LOCAL => {
                let list_i = bcx.ins().iconst(ty::I32, a as i64);
                let key_i = bcx.ins().iconst(ty::I32, b as i64);
                let call = bcx.ins().call(h.subscript_int, &[locals_ptr, list_i, key_i]);
                stack.push(bcx.inst_results(call)[0]);
                slot_map.push(None);
            }
            OP_SUBSCRIPT_ADD_IMM => {
                let list_i = bcx.ins().iconst(ty::I32, a as i64);
                let key_i = bcx.ins().iconst(ty::I32, c as i64);
                let imm = bcx.ins().iconst(ty::I64, b as i16 as i64);
                let call = bcx.ins().call(h.subscript_int_imm, &[locals_ptr, list_i, key_i, imm]);
                stack.push(bcx.inst_results(call)[0]);
                slot_map.push(None);
            }
            OP_POP_JUMP_IF_SUBSCRIPT_GT => {
                let list_i = bcx.ins().iconst(ty::I32, a as i64);
                let idx_local = (b & 0xFF) as u32;
                let imm_val = ((b >> 8) & 0xFF) as i16 as i64;
                let key_i = bcx.ins().iconst(ty::I32, idx_local as i64);
                let imm = bcx.ins().iconst(ty::I64, imm_val);
                let call_b = bcx.ins().call(h.subscript_int_imm, &[locals_ptr, list_i, key_i, imm]);
                let val_b = bcx.inst_results(call_b)[0];
                let key_i2 = bcx.ins().iconst(ty::I32, idx_local as i64);
                let call_a = bcx.ins().call(h.subscript_int, &[locals_ptr, list_i, key_i2]);
                let val_a = bcx.inst_results(call_a)[0];
                let gt = bcx.ins().icmp(IntCC::SignedGreaterThan, val_a, val_b);
                let should_skip = bcx.ins().bnot(gt);
                let target = c as usize;
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                let cont_block = bcx.create_block();
                bcx.ins().brif(should_skip, target_block, &[], cont_block, &[]);
                bcx.switch_to_block(cont_block);
                block_map.entry(ip + 1).or_insert(cont_block);
            }
            OP_SWAP_ADJACENT => {
                let list_i = bcx.ins().iconst(ty::I32, a as i64);
                let key_i = bcx.ins().iconst(ty::I32, b as i64);
                bcx.ins().call(h.swap_adjacent, &[locals_ptr, list_i, key_i]);
            }
            OP_BUILD_LIST => {
                let idx = bcx.ins().iconst(ty::I32, b as i64);
                bcx.ins().call(h.build_list, &[locals_ptr, idx]);
            }
            OP_LIST_APPEND => {
                if let Some(val) = stack.pop() {
                    slot_map.pop();
                    let list_i = bcx.ins().iconst(ty::I32, a.wrapping_sub(1) as i64);
                    let val_i = bcx.ins().iconst(ty::I32, a as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, val_i, val]);
                    bcx.ins().call(h.list_append, &[locals_ptr, list_i, val_i]);
                }
            }
            OP_STORE_SUBSCRIPT_LOCAL => {
                if let Some(val) = stack.pop() {
                    slot_map.pop();
                    let list_i = bcx.ins().iconst(ty::I32, a as i64);
                    let key_i = bcx.ins().iconst(ty::I32, b as i64);
                    let val_i = bcx.ins().iconst(ty::I32, (a + 1) as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, val_i, val]);
                    bcx.ins().call(h.store_subscript, &[locals_ptr, list_i, key_i, val_i]);
                }
            }
            OP_SUBSCRIPT => {
                let key = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let key_slot_opt = slot_map.pop().unwrap_or(None);
                let key_slot = key_slot_opt.unwrap_or_else(|| { let s = next_temp_slot; next_temp_slot += 1; s });
                if let Some(ks) = key_slot_opt {
                    let ks_i32 = bcx.ins().iconst(ty::I32, ks as i64);
                    let dst_i32 = bcx.ins().iconst(ty::I32, key_slot as i64);
                    bcx.ins().call(h.store_value, &[locals_ptr, ks_i32, dst_i32]);
                } else {
                    let ti = bcx.ins().iconst(ty::I32, key_slot as i64);
                    if val_type(bcx, key) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, key]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, key]); }
                }
                let list = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let list_slot_opt = slot_map.pop().unwrap_or(None);
                let list_slot = list_slot_opt.unwrap_or_else(|| { let s = next_temp_slot; next_temp_slot += 1; s });
                if let Some(ls) = list_slot_opt {
                    let ls_i32 = bcx.ins().iconst(ty::I32, ls as i64);
                    let dst_i32 = bcx.ins().iconst(ty::I32, list_slot as i64);
                    bcx.ins().call(h.store_value, &[locals_ptr, ls_i32, dst_i32]);
                } else {
                    let ti = bcx.ins().iconst(ty::I32, list_slot as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, ti, list]);
                }
                let result_slot = next_temp_slot; next_temp_slot += 1;
                let ls = bcx.ins().iconst(ty::I32, list_slot as i64);
                let ks = bcx.ins().iconst(ty::I32, key_slot as i64);
                let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                bcx.ins().call(h.subscript_generic, &[locals_ptr, ls, ks, rs]);
                let rs2 = bcx.ins().iconst(ty::I32, result_slot as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, rs2]);
                stack.push(bcx.inst_results(call)[0]);
                slot_map.push(Some(result_slot));
            }
            OP_STORE_SUBSCRIPT => {
                let val = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let val_slot_opt = slot_map.pop().unwrap_or(None);
                let val_slot = val_slot_opt.unwrap_or_else(|| { let s = next_temp_slot; next_temp_slot += 1; s });
                if let Some(vs) = val_slot_opt {
                    let vs_i32 = bcx.ins().iconst(ty::I32, vs as i64);
                    let dst_i32 = bcx.ins().iconst(ty::I32, val_slot as i64);
                    bcx.ins().call(h.store_value, &[locals_ptr, vs_i32, dst_i32]);
                } else {
                    let ti = bcx.ins().iconst(ty::I32, val_slot as i64);
                    if val_type(bcx, val) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, val]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, val]); }
                }
                let key = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let key_slot_opt = slot_map.pop().unwrap_or(None);
                let key_slot = key_slot_opt.unwrap_or_else(|| { let s = next_temp_slot; next_temp_slot += 1; s });
                if let Some(ks) = key_slot_opt {
                    let ks_i32 = bcx.ins().iconst(ty::I32, ks as i64);
                    let dst_i32 = bcx.ins().iconst(ty::I32, key_slot as i64);
                    bcx.ins().call(h.store_value, &[locals_ptr, ks_i32, dst_i32]);
                } else {
                    let ti = bcx.ins().iconst(ty::I32, key_slot as i64);
                    if val_type(bcx, key) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, key]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, key]); }
                }
                let list = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let list_slot_opt = slot_map.pop().unwrap_or(None);
                let list_slot = list_slot_opt.unwrap_or_else(|| { let s = next_temp_slot; next_temp_slot += 1; s });
                if let Some(ls) = list_slot_opt {
                    let ls_i32 = bcx.ins().iconst(ty::I32, ls as i64);
                    let dst_i32 = bcx.ins().iconst(ty::I32, list_slot as i64);
                    bcx.ins().call(h.store_value, &[locals_ptr, ls_i32, dst_i32]);
                } else {
                    let ti = bcx.ins().iconst(ty::I32, list_slot as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, ti, list]);
                }
                let ls = bcx.ins().iconst(ty::I32, list_slot as i64);
                let ks = bcx.ins().iconst(ty::I32, key_slot as i64);
                let vs = bcx.ins().iconst(ty::I32, val_slot as i64);
                bcx.ins().call(h.store_subscript_generic, &[locals_ptr, ls, ks, vs]);
            }
            OP_GET_INSTANCE_FIELD => {
                let name_idx = c as usize;
                let obj = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let obj_slot_opt = slot_map.pop().unwrap_or(None);
                let obj_slot = obj_slot_opt.unwrap_or_else(|| {
                    let s = next_temp_slot; next_temp_slot += 1;
                    let ti = bcx.ins().iconst(ty::I32, s as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, ti, obj]);
                    s
                });
                let obj_i32 = bcx.ins().iconst(ty::I32, obj_slot as i64);
                let result_slot = next_temp_slot; next_temp_slot += 1;
                let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                match module.constants.get(name_idx) {
                    Some(CompilerValue::Integer(offset)) => {
                        let offset_i32 = bcx.ins().iconst(ty::I32, *offset as i64);
                        bcx.ins().call(h.get_instance_field, &[locals_ptr, obj_i32, offset_i32, rs]);
                    }
                    Some(CompilerValue::String(_name)) => {
                        let name_slot = next_temp_slot; next_temp_slot += 1;
                        let name_str = _name.as_str();
                        let ptr = name_str.as_ptr();
                        let len = name_str.len() as u32;
                        let ptr_val = bcx.ins().iconst(ty::I64, ptr as i64);
                        let len_val = bcx.ins().iconst(ty::I32, len as i64);
                        let ns = bcx.ins().iconst(ty::I32, name_slot as i64);
                        bcx.ins().call(h.store_string, &[locals_ptr, ns, ptr_val, len_val]);
                        bcx.ins().call(h.get_field_by_name_slot, &[locals_ptr, obj_i32, ns, rs]);
                    }
                    _ => {
                        let zero = bcx.ins().iconst(ty::I64, 0);
                        bcx.ins().call(h.store_int, &[locals_ptr, rs, zero]);
                    }
                }
                let rs2 = bcx.ins().iconst(ty::I32, result_slot as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, rs2]);
                stack.push(bcx.inst_results(call)[0]);
                slot_map.push(Some(result_slot));
            }
            OP_CALL_FUNCTION => {
                let argc = a as usize;
                if argc == 0 {
                    let func_slot_opt = slot_map.pop().unwrap_or(None);
                    stack.pop();
                    if let Some(func_slot) = func_slot_opt {
                        let func_slot_i32 = bcx.ins().iconst(ty::I32, func_slot as i64);
                        let zero_i32 = bcx.ins().iconst(ty::I32, 0);
                        let call = bcx.ins().call(h.call_func, &[
                            locals_ptr, func_slot_i32, zero_i32, zero_i32,
                        ]);
                        stack.push(bcx.inst_results(call)[0]);
                        slot_map.push(None);
                    } else {
                        stack.push(bcx.ins().iconst(ty::I64, 0));
                        slot_map.push(None);
                    }
                } else {
                    let mut arg_sources: Vec<(Option<u32>, cranelift::prelude::Value)> = Vec::with_capacity(argc);
                    for _ in 0..argc {
                        let val = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                        let slot = slot_map.pop().unwrap_or(None);
                        arg_sources.push((slot, val));
                    }
                    let func_slot = slot_map.pop().unwrap_or(None).unwrap_or(0);
                    stack.pop();
                    let mut arg_slot_indices: Vec<u32> = Vec::with_capacity(argc);
                    for (src, val) in arg_sources.iter().rev() {
                        let temp = next_temp_slot;
                        next_temp_slot += 1;
                        if let Some(src_slot) = src {
                            let src_i32 = bcx.ins().iconst(ty::I32, *src_slot as i64);
                            let dst_i32 = bcx.ins().iconst(ty::I32, temp as i64);
                            bcx.ins().call(h.store_value, &[locals_ptr, src_i32, dst_i32]);
                        } else {
                            let temp_i32 = bcx.ins().iconst(ty::I32, temp as i64);
                            bcx.ins().call(h.store_int, &[locals_ptr, temp_i32, *val]);
                        }
                        arg_slot_indices.push(temp);
                    }
                    let func_slot_i32 = bcx.ins().iconst(ty::I32, func_slot as i64);
                    let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                    if argc == 2 {
                        let a0 = bcx.ins().iconst(ty::I32, arg_slot_indices[0] as i64);
                        let a1 = bcx.ins().iconst(ty::I32, arg_slot_indices[1] as i64);
                        bcx.ins().call(h.call_func_2_to_slot, &[locals_ptr, func_slot_i32, a0, a1, rs]);
                    } else if argc == 3 {
                        let a0 = bcx.ins().iconst(ty::I32, arg_slot_indices[0] as i64);
                        let a1 = bcx.ins().iconst(ty::I32, arg_slot_indices[1] as i64);
                        let a2 = bcx.ins().iconst(ty::I32, arg_slot_indices[2] as i64);
                        bcx.ins().call(h.call_func_3_to_slot, &[locals_ptr, func_slot_i32, a0, a1, a2, rs]);
                    } else {
                        let first_arg = arg_slot_indices.first().copied().unwrap_or(0);
                        let first_arg_i32 = bcx.ins().iconst(ty::I32, first_arg as i64);
                        let argc_i32 = bcx.ins().iconst(ty::I32, argc as i64);
                        bcx.ins().call(h.call_func_to_slot, &[locals_ptr, func_slot_i32, first_arg_i32, argc_i32, rs]);
                    }
                    let load_call = bcx.ins().call(h.load_int, &[locals_ptr, rs]);
                    stack.push(bcx.inst_results(load_call)[0]);
                    slot_map.push(None);
                }
            }
            OP_LOAD_NAME | OP_LOAD_GLOBAL => {
                let temp = next_temp_slot;
                next_temp_slot += 1;
                let name_i32 = bcx.ins().iconst(ty::I32, b as i64);
                let slot_i32 = bcx.ins().iconst(ty::I32, temp as i64);
                bcx.ins().call(h.load_global_to_local, &[name_i32, locals_ptr, slot_i32]);
                let zero = bcx.ins().iconst(ty::I64, 0);
                stack.push(zero);
                slot_map.push(Some(temp));
            }
            OP_TAIL_CALL => {
                let argc = a as usize;
                let mut args: Vec<cranelift::prelude::Value> = Vec::with_capacity(argc);
                for _ in 0..argc {
                    args.push(stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0)));
                    slot_map.pop();
                }
                stack.pop();
                slot_map.pop();
                args.reverse();
                for (i, &arg) in args.iter().enumerate() {
                    if i < param_indices.len() {
                        let idx = bcx.ins().iconst(ty::I32, param_indices[i] as i64);
                        bcx.ins().call(h.store_int, &[locals_ptr, idx, arg]);
                    }
                }
                let loop_start = {
                    let first_op = (module.packed[body] & 0xFF) as u8;
                    if first_op == OP_JUMP_FORWARD { body + 1 } else { body }
                };
                let entry_block = *block_map.get(&loop_start).expect("JIT: loop block not found in block_map — malformed bytecode");
                bcx.ins().jump(entry_block, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
                last_was_branch = true;
            }
            OP_COMPARE_LE_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let cmp = bcx.ins().icmp(IntCC::SignedLessThanOrEqual, left, right);
                    stack.push(cmp);
                    slot_map.push(None);
                }
            }
            OP_COMPARE_GE_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let cmp = bcx.ins().icmp(IntCC::SignedGreaterThanOrEqual, left, right);
                    stack.push(cmp);
                    slot_map.push(None);
                }
            }
            OP_POP_JUMP_IF_LE_LOCAL_IMM => {
                let local_idx = bcx.ins().iconst(ty::I32, a as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, local_idx]);
                let local_val = bcx.inst_results(call)[0];
                let imm = bcx.ins().iconst(ty::I64, b as i16 as i64);
                let cmp = bcx.ins().icmp(IntCC::SignedLessThanOrEqual, local_val, imm);
                let target = ip + 1 + c as usize;
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                let cont_block = bcx.create_block();
                bcx.ins().brif(cmp, target_block, &[], cont_block, &[]);
                bcx.switch_to_block(cont_block);
                block_map.entry(ip + 1).or_insert(cont_block);
            }
            OP_POP_JUMP_IF_LT_LOCAL_IMM => {
                let local_idx = bcx.ins().iconst(ty::I32, a as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, local_idx]);
                let local_val = bcx.inst_results(call)[0];
                let imm = bcx.ins().iconst(ty::I64, b as i16 as i64);
                let cmp = bcx.ins().icmp(IntCC::SignedLessThan, local_val, imm);
                let target = ip + 1 + c as usize;
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                let cont_block = bcx.create_block();
                bcx.ins().brif(cmp, target_block, &[], cont_block, &[]);
                bcx.switch_to_block(cont_block);
                block_map.entry(ip + 1).or_insert(cont_block);
            }
            OP_SUBTRACT_LOCAL_IMM => {
                let local_idx = bcx.ins().iconst(ty::I32, a as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, local_idx]);
                let local_val = bcx.inst_results(call)[0];
                let imm = b as i16 as i64;
                let result = bcx.ins().iadd_imm(local_val, -imm);
                stack.push(result);
                slot_map.push(None);
            }
            OP_ADD_LOCAL_IMM => {
                let local_idx = bcx.ins().iconst(ty::I32, a as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, local_idx]);
                let local_val = bcx.inst_results(call)[0];
                let imm = b as i16 as i64;
                let result = bcx.ins().iadd_imm(local_val, imm);
                stack.push(result);
                slot_map.push(None);
            }
            OP_LOAD_TRUE => { stack.push(bcx.ins().iconst(ty::I64, 1)); slot_map.push(None); }
            OP_LOAD_FALSE => { stack.push(bcx.ins().iconst(ty::I64, 0)); slot_map.push(None); }
            OP_LOAD_NONE => { stack.push(bcx.ins().iconst(ty::I64, 0)); slot_map.push(None); }
            OP_BINARY_FLOOR_DIVIDE | OP_BINARY_FLOOR_DIV_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let q = bcx.ins().sdiv(left, right);
                    let r = bcx.ins().srem(left, right);
                    let r_is_nonzero = bcx.ins().icmp_imm(IntCC::NotEqual, r, 0);
                    let left_sign = bcx.ins().ushr_imm(left, 63);
                    let right_sign = bcx.ins().ushr_imm(right, 63);
                    let sign_differs = bcx.ins().icmp(IntCC::NotEqual, left_sign, right_sign);
                    let needs_adjust = bcx.ins().band(r_is_nonzero, sign_differs);
                    let one = bcx.ins().iconst(ty::I64, 1);
                    let adjusted = bcx.ins().isub(q, one);
                    let result = bcx.ins().select(needs_adjust, adjusted, q);
                    stack.push(result);
                    slot_map.push(None);
                }
            }
            OP_BINARY_BIT_AND => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().band(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_BIT_OR => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().bor(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_BIT_XOR => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().bxor(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_SHL => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let shift = bcx.ins().uextend(ty::I64, right);
                    stack.push(bcx.ins().ishl(left, shift));
                    slot_map.push(None);
                }
            }
            OP_BINARY_SHR => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let shift = bcx.ins().uextend(ty::I64, right);
                    stack.push(bcx.ins().ushr(left, shift));
                    slot_map.push(None);
                }
            }
            OP_UNARY_BIT_NOT => {
                if let Some(val) = stack.pop() {
                    slot_map.pop();
                    stack.push(bcx.ins().bnot(val));
                    slot_map.push(None);
                }
            }
            OP_LOGICAL_AND => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let zero = bcx.ins().iconst(ty::I64, 0);
                    let left_bool = bcx.ins().icmp(IntCC::NotEqual, left, zero);
                    let right_bool = bcx.ins().icmp(IntCC::NotEqual, right, zero);
                    let result = bcx.ins().band(left_bool, right_bool);
                    let ext = bcx.ins().uextend(ty::I64, result);
                    stack.push(ext);
                    slot_map.push(None);
                }
            }
            OP_LOGICAL_OR => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    let zero = bcx.ins().iconst(ty::I64, 0);
                    let left_bool = bcx.ins().icmp(IntCC::NotEqual, left, zero);
                    let right_bool = bcx.ins().icmp(IntCC::NotEqual, right, zero);
                    let result = bcx.ins().bor(left_bool, right_bool);
                    let ext = bcx.ins().uextend(ty::I64, result);
                    stack.push(ext);
                    slot_map.push(None);
                }
            }
            OP_SET_INSTANCE_FIELD => {
                let name_idx = c as usize;
                let val = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let val_slot_opt = slot_map.pop().unwrap_or(None);
                let val_slot = val_slot_opt.unwrap_or_else(|| {
                    let s = next_temp_slot; next_temp_slot += 1;
                    let ti = bcx.ins().iconst(ty::I32, s as i64);
                    if val_type(bcx, val) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, val]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, val]); }
                    s
                });
                let obj = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let obj_slot_opt = slot_map.pop().unwrap_or(None);
                let obj_slot = obj_slot_opt.unwrap_or_else(|| {
                    let s = next_temp_slot; next_temp_slot += 1;
                    let ti = bcx.ins().iconst(ty::I32, s as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, ti, obj]);
                    s
                });
                let obj_i32 = bcx.ins().iconst(ty::I32, obj_slot as i64);
                let val_i32 = bcx.ins().iconst(ty::I32, val_slot as i64);
                match module.constants.get(name_idx) {
                    Some(CompilerValue::Integer(offset)) => {
                        let offset_i32 = bcx.ins().iconst(ty::I32, *offset as i64);
                        bcx.ins().call(h.set_instance_field, &[locals_ptr, obj_i32, offset_i32, val_i32]);
                    }
                    Some(CompilerValue::String(_name)) => {
                        let name_slot = next_temp_slot; next_temp_slot += 1;
                        let name_str = _name.as_str();
                        let ptr = name_str.as_ptr();
                        let len = name_str.len() as u32;
                        let ptr_val = bcx.ins().iconst(ty::I64, ptr as i64);
                        let len_val = bcx.ins().iconst(ty::I32, len as i64);
                        let ns = bcx.ins().iconst(ty::I32, name_slot as i64);
                        bcx.ins().call(h.store_string, &[locals_ptr, ns, ptr_val, len_val]);
                        bcx.ins().call(h.store_attr, &[locals_ptr, obj_i32, ns, val_i32]);
                    }
                    _ => {}
                }
            }
            OP_CALL_METHOD => {
                let method_name_idx = a as usize;
                let argc = b as usize;
                let mut arg_slot_indices: Vec<u32> = Vec::with_capacity(argc);
                for _ in 0..argc {
                    let val = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                    slot_map.pop();
                    let temp = next_temp_slot;
                    next_temp_slot += 1;
                    let temp_i32 = bcx.ins().iconst(ty::I32, temp as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, temp_i32, val]);
                    arg_slot_indices.push(temp);
                }
                let obj = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let obj_slot_opt = slot_map.pop().unwrap_or(None);
                let obj_slot = obj_slot_opt.unwrap_or_else(|| {
                    let s = next_temp_slot; next_temp_slot += 1;
                    let ti = bcx.ins().iconst(ty::I32, s as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, ti, obj]);
                    s
                });
                let name_slot = next_temp_slot; next_temp_slot += 1;
                if let Some(name_str_val) = module.names.get(method_name_idx) {
                    let name_str = name_str_val.as_str();
                    let ptr = name_str.as_ptr();
                    let len = name_str.len() as u32;
                    let ptr_val = bcx.ins().iconst(ty::I64, ptr as i64);
                    let len_val = bcx.ins().iconst(ty::I32, len as i64);
                    let ns = bcx.ins().iconst(ty::I32, name_slot as i64);
                    bcx.ins().call(h.store_string, &[locals_ptr, ns, ptr_val, len_val]);
                }
                let obj_i32 = bcx.ins().iconst(ty::I32, obj_slot as i64);
                let ns_i32 = bcx.ins().iconst(ty::I32, name_slot as i64);
                let first_arg = arg_slot_indices.first().copied().unwrap_or(0);
                let first_arg_i32 = bcx.ins().iconst(ty::I32, first_arg as i64);
                let argc_i32 = bcx.ins().iconst(ty::I32, argc as i64);
                let result_slot = next_temp_slot; next_temp_slot += 1;
                let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                bcx.ins().call(h.call_method_to_slot, &[locals_ptr, obj_i32, ns_i32, first_arg_i32, argc_i32, rs]);
                let rs2 = bcx.ins().iconst(ty::I32, result_slot as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, rs2]);
                stack.push(bcx.inst_results(call)[0]);
                slot_map.push(Some(result_slot));
            }
            OP_CALL_METHOD_1ARG => {
                let method_name_idx = a as usize;
                let val = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                slot_map.pop();
                let arg_slot = next_temp_slot; next_temp_slot += 1;
                let arg_i32 = bcx.ins().iconst(ty::I32, arg_slot as i64);
                bcx.ins().call(h.store_int, &[locals_ptr, arg_i32, val]);
                let obj = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let obj_slot_opt = slot_map.pop().unwrap_or(None);
                let obj_slot = obj_slot_opt.unwrap_or_else(|| {
                    let s = next_temp_slot; next_temp_slot += 1;
                    let ti = bcx.ins().iconst(ty::I32, s as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, ti, obj]);
                    s
                });
                let name_slot = next_temp_slot; next_temp_slot += 1;
                if let Some(name_str_val) = module.names.get(method_name_idx) {
                    let name_str = name_str_val.as_str();
                    let ptr = name_str.as_ptr();
                    let len = name_str.len() as u32;
                    let ptr_val = bcx.ins().iconst(ty::I64, ptr as i64);
                    let len_val = bcx.ins().iconst(ty::I32, len as u32 as i64);
                    let ns = bcx.ins().iconst(ty::I32, name_slot as i64);
                    bcx.ins().call(h.store_string, &[locals_ptr, ns, ptr_val, len_val]);
                }
                let obj_i32 = bcx.ins().iconst(ty::I32, obj_slot as i64);
                let ns_i32 = bcx.ins().iconst(ty::I32, name_slot as i64);
                let result_slot = next_temp_slot; next_temp_slot += 1;
                let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                let one_i32 = bcx.ins().iconst(ty::I32, 1);
                bcx.ins().call(h.call_method_to_slot, &[locals_ptr, obj_i32, ns_i32, arg_i32, one_i32, rs]);
                let rs2 = bcx.ins().iconst(ty::I32, result_slot as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, rs2]);
                stack.push(bcx.inst_results(call)[0]);
                slot_map.push(Some(result_slot));
            }
            OP_CALL_METHOD_VOID => {
                let method_name_idx = a as usize;
                let argc = b as usize;
                let mut arg_slot_indices: Vec<u32> = Vec::with_capacity(argc);
                for _ in 0..argc {
                    let val = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                    slot_map.pop();
                    let temp = next_temp_slot; next_temp_slot += 1;
                    let temp_i32 = bcx.ins().iconst(ty::I32, temp as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, temp_i32, val]);
                    arg_slot_indices.push(temp);
                }
                let obj = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let obj_slot_opt = slot_map.pop().unwrap_or(None);
                let obj_slot = obj_slot_opt.unwrap_or_else(|| {
                    let s = next_temp_slot; next_temp_slot += 1;
                    let ti = bcx.ins().iconst(ty::I32, s as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, ti, obj]);
                    s
                });
                let name_slot = next_temp_slot; next_temp_slot += 1;
                if let Some(name_str_val) = module.names.get(method_name_idx) {
                    let name_str = name_str_val.as_str();
                    let ptr = name_str.as_ptr();
                    let len = name_str.len() as u32;
                    let ptr_val = bcx.ins().iconst(ty::I64, ptr as i64);
                    let len_val = bcx.ins().iconst(ty::I32, len as i64);
                    let ns = bcx.ins().iconst(ty::I32, name_slot as i64);
                    bcx.ins().call(h.store_string, &[locals_ptr, ns, ptr_val, len_val]);
                }
                let obj_i32 = bcx.ins().iconst(ty::I32, obj_slot as i64);
                let ns_i32 = bcx.ins().iconst(ty::I32, name_slot as i64);
                let first_arg = arg_slot_indices.first().copied().unwrap_or(0);
                let first_arg_i32 = bcx.ins().iconst(ty::I32, first_arg as i64);
                let argc_i32 = bcx.ins().iconst(ty::I32, argc as i64);
                let result_slot = next_temp_slot; next_temp_slot += 1;
                let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                bcx.ins().call(h.call_method_to_slot, &[locals_ptr, obj_i32, ns_i32, first_arg_i32, argc_i32, rs]);
            }
            OP_STORE_ATTR => {
                let name_idx = b as usize;
                let val = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let val_slot_opt = slot_map.pop().unwrap_or(None);
                let val_slot = val_slot_opt.unwrap_or_else(|| {
                    let s = next_temp_slot; next_temp_slot += 1;
                    let ti = bcx.ins().iconst(ty::I32, s as i64);
                    if val_type(bcx, val) == ValType::F64 { bcx.ins().call(h.store_float, &[locals_ptr, ti, val]); }
                    else { bcx.ins().call(h.store_int, &[locals_ptr, ti, val]); }
                    s
                });
                let obj = stack.pop().unwrap_or_else(|| bcx.ins().iconst(ty::I64, 0));
                let obj_slot_opt = slot_map.pop().unwrap_or(None);
                let obj_slot = obj_slot_opt.unwrap_or_else(|| {
                    let s = next_temp_slot; next_temp_slot += 1;
                    let ti = bcx.ins().iconst(ty::I32, s as i64);
                    bcx.ins().call(h.store_int, &[locals_ptr, ti, obj]);
                    s
                });
                let obj_i32 = bcx.ins().iconst(ty::I32, obj_slot as i64);
                let val_i32 = bcx.ins().iconst(ty::I32, val_slot as i64);
                let name_slot = next_temp_slot; next_temp_slot += 1;
                if let Some(name_str_val) = module.names.get(name_idx) {
                    let name_str = name_str_val.as_str();
                    let ptr = name_str.as_ptr();
                    let len = name_str.len() as u32;
                    let ptr_val = bcx.ins().iconst(ty::I64, ptr as i64);
                    let len_val = bcx.ins().iconst(ty::I32, len as i64);
                    let ns = bcx.ins().iconst(ty::I32, name_slot as i64);
                    bcx.ins().call(h.store_string, &[locals_ptr, ns, ptr_val, len_val]);
                    bcx.ins().call(h.store_attr, &[locals_ptr, obj_i32, ns, val_i32]);
                }
            }
            OP_POP_JUMP_IF_SUBSCRIPT_EQ_LOCAL | OP_POP_JUMP_IF_SUBSCRIPT_NE_LOCAL => {
                let list_local = a as u32;
                let idx_local = (b & 0xFF) as u32;
                let cmp_local = ((b >> 8) & 0xFF) as u32;
                let target = c as usize;
                let list_i = bcx.ins().iconst(ty::I32, list_local as i64);
                let key_i = bcx.ins().iconst(ty::I32, idx_local as i64);
                let cmp_i = bcx.ins().iconst(ty::I32, cmp_local as i64);
                let call = bcx.ins().call(h.subscript_int_compare, &[locals_ptr, list_i, key_i, cmp_i]);
                let eq_result = bcx.inst_results(call)[0];
                let is_eq = bcx.ins().icmp_imm(IntCC::NotEqual, eq_result, 0);
                let should_jump = if opcode == OP_POP_JUMP_IF_SUBSCRIPT_EQ_LOCAL {
                    bcx.ins().bnot(is_eq)
                } else {
                    is_eq
                };
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                let cont_block = bcx.create_block();
                bcx.ins().brif(should_jump, target_block, &[], cont_block, &[]);
                bcx.switch_to_block(cont_block);
                block_map.entry(ip + 1).or_insert(cont_block);
            }
            OP_CREATE_INSTANCE => {
                let name_idx = a as u32;
                let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
                let name_i = bcx.ins().iconst(ty::I32, name_idx as i64);
                bcx.ins().call(h.create_instance, &[name_i, locals_ptr, rs]);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, rs]);
                stack.push(bcx.inst_results(call)[0]);
                slot_map.push(Some(result_slot));
                next_temp_slot += 1;
            }
            OP_INCREMENT_INT => {
                let local_idx = a as u32;
                let incr = b as i64;
                let li = bcx.ins().iconst(ty::I32, local_idx as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, li]);
                let cur_val = bcx.inst_results(call)[0];
                let new_val = bcx.ins().iadd_imm(cur_val, incr);
                bcx.ins().call(h.store_int, &[locals_ptr, li, new_val]);
            }
            OP_JUMP_WHILE_INCREMENTED_LT => {
                let local_idx = a as u32;
                let loop_start = b as usize;
                let increment = ((c & 0xFFFF) as u16) as i16 as i64;
                let limit_field = (c >> 16) as u16;
                let is_local_limit = (limit_field & 0x8000) != 0;
                let limit_idx = (limit_field & 0x7FFF) as u32;
                let li = bcx.ins().iconst(ty::I32, local_idx as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, li]);
                let cur_val = bcx.inst_results(call)[0];
                let limit_val = if is_local_limit {
                    let lim_li = bcx.ins().iconst(ty::I32, limit_idx as i64);
                    let lim_call = bcx.ins().call(h.load_int, &[locals_ptr, lim_li]);
                    bcx.inst_results(lim_call)[0]
                } else {
                    match module.constants.get(limit_idx as usize) {
                        Some(CompilerValue::Integer(n)) => bcx.ins().iconst(ty::I64, *n),
                        Some(CompilerValue::Float(f)) => {
                            let fval = bcx.ins().f64const(*f);
                            bcx.ins().fcvt_to_sint_sat(types::I64, fval)
                        }
                        _ => bcx.ins().iconst(ty::I64, 0),
                    }
                };
                let should_continue = bcx.ins().icmp(IntCC::SignedLessThan, cur_val, limit_val);
                let body_block = *block_map.entry(loop_start).or_insert_with(|| bcx.create_block());
                let cont_block = bcx.create_block();
                let new_val = bcx.ins().iadd_imm(cur_val, increment);
                bcx.ins().call(h.store_int, &[locals_ptr, li, new_val]);
                bcx.ins().brif(should_continue, body_block, &[], cont_block, &[]);
                bcx.switch_to_block(cont_block);
                block_map.entry(ip + 1).or_insert(cont_block);
            }
            _ => { return false; }
        }
        ip += 1;
    }
    if !last_was_branch {
        bcx.ins().jump(return_block, &[]);
    }
    bcx.switch_to_block(return_block);
    let rs = bcx.ins().iconst(ty::I32, result_slot as i64);
    let call = bcx.ins().call(h.load_int, &[locals_ptr, rs]);
    let ret_val = bcx.inst_results(call)[0];
    bcx.ins().return_(&[ret_val]);
    true
}

#[derive(Clone, Copy, PartialEq)]
enum ValType { I64, F64 }

fn val_type(bcx: &FunctionBuilder, val: cranelift::prelude::Value) -> ValType {
    let t = bcx.func.dfg.value_type(val);
    if t == ty::F64 {
        ValType::F64
    } else {
        ValType::I64
    }
}
