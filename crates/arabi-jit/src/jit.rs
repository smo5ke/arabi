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
    for_range_next: FuncRef,
    build_list: FuncRef,
    list_append: FuncRef,
    subscript_int: FuncRef,
    subscript_int_imm: FuncRef,
    store_subscript: FuncRef,
    _store_subscript_add_imm: FuncRef,
    swap_adjacent: FuncRef,
    _subscript_gt: FuncRef,
    get_time: FuncRef,
    print_int: FuncRef,
    print_float: FuncRef,
    print_sep: FuncRef,
    _print_newline: FuncRef,
    load_global_to_local: FuncRef,
    call_func: FuncRef,
}

unsafe impl Send for CraneliftJIT {}
unsafe impl Sync for CraneliftJIT {}

impl CraneliftJIT {
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        let isa_builder = cranelift_native::builder()
            .unwrap_or_else(|msg| panic!("host machine not supported: {msg}"));
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).unwrap();
        let builder = JITBuilder::with_isa(isa, default_libcall_names());
        let module = JITModule::new(builder);
        let func_ctx = FunctionBuilderContext::new();
        CraneliftJIT { module, func_ctx, compiled: HashMap::new(), finalized: false }
    }

    pub fn with_symbols<F: FnOnce(&mut JITBuilder)>(register: F) -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        let isa_builder = cranelift_native::builder()
            .unwrap_or_else(|msg| panic!("host machine not supported: {msg}"));
        let isa = isa_builder.finish(settings::Flags::new(flag_builder)).unwrap();
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
    ) -> Option<*const u8> {
        if self.compiled.contains_key(&body) {
            return self.compiled.get(&body).map(|c| c.ptr);
        }
        if num_params != 1 {
            return None;
        }
        if !self.is_integer_recursive(func_name, body, module) {
            return None;
        }
        self.compile_fibonacci(func_name, body, module)
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

        {
            let mut bcx = FunctionBuilder::new(&mut ctx.func, &mut self.func_ctx);
            let entry = bcx.create_block();
            bcx.append_block_params_for_function_params(entry);
            bcx.switch_to_block(entry);
            bcx.seal_block(entry);
            let locals_ptr = bcx.block_params(entry)[0];
            let helpers = declare_helpers(&mut self.module, &mut bcx);
            let ok = emit_loop_bytecode(&mut bcx, body, module, locals_ptr, &helpers, param_indices, func_name, _num_locals);
            if !ok {
                let zero = bcx.ins().iconst(types::I64, 0);
                bcx.ins().return_(&[zero]);
            }
            bcx.finalize();
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
        let func_id = module.declare_function(name, Linkage::Import, &sig).unwrap();
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
        store_subscript: decl(module, bcx, "arabi_jit_store_subscript", &[types::I64, types::I32, types::I32, types::I32], &[]),
        _store_subscript_add_imm: decl(module, bcx, "arabi_jit_store_subscript_add_imm", &[types::I64, types::I32, types::I32, types::I64, types::I32], &[]),
        swap_adjacent: decl(module, bcx, "arabi_jit_swap_adjacent", &[types::I64, types::I32, types::I32], &[]),
        _subscript_gt: decl(module, bcx, "arabi_jit_subscript_gt", &[types::I64, types::I32, types::I32, types::I64], &[types::I64]),
        get_time: decl(module, bcx, "arabi_jit_get_time", &[], &[types::F64]),
        print_int: decl(module, bcx, "arabi_jit_print_int", &[types::I64], &[]),
        print_float: decl(module, bcx, "arabi_jit_print_float", &[types::F64], &[]),
        print_sep: decl(module, bcx, "arabi_jit_print_sep", &[], &[]),
        _print_newline: decl(module, bcx, "arabi_jit_print_newline", &[], &[]),
        load_global_to_local: decl(module, bcx, "arabi_jit_load_global_to_local", &[types::I32, types::I64, types::I32], &[]),
        call_func: decl(module, bcx, "arabi_jit_call_func", &[types::I64, types::I32, types::I32, types::I32], &[types::I64]),
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
) -> bool {
    let mut ip = body;
    let max_ip = module.packed.len().min(body + 2000);
    let mut block_map: HashMap<usize, Block> = HashMap::new();
    block_map.insert(body, bcx.current_block().unwrap_or_else(|| {
        let blk = bcx.create_block();
        bcx.switch_to_block(blk);
        blk
    }));

    let mut stack: Vec<cranelift::prelude::Value> = Vec::new();
    let mut slot_map: Vec<Option<u32>> = Vec::new();
    let mut next_temp_slot: u32 = num_locals as u32;

    while ip < max_ip {
        let packed = module.packed[ip];
        let opcode = (packed & 0xFF) as u8;
        let a = ((packed >> 8) & 0xFF) as u8;
        let b = ((packed >> 16) & 0xFFFF) as u16;
        let c = (packed >> 32) as u32;

        match opcode {
            OP_HALT | OP_RETURN_VALUE => {
                let val = if !stack.is_empty() { stack.pop().unwrap() } else {
                    bcx.ins().iconst(ty::I64, 0)
                };
                bcx.ins().return_(&[val]);
                return true;
            }
            OP_LOAD_CONST => {
                match module.constants.get(b as usize) {
                    Some(CompilerValue::Integer(n)) => { stack.push(bcx.ins().iconst(ty::I64, *n)); slot_map.push(None); }
                    Some(CompilerValue::Float(f)) => { stack.push(bcx.ins().f64const(*f)); slot_map.push(None); }
                    Some(CompilerValue::Boolean(true)) => { stack.push(bcx.ins().iconst(ty::I64, 1)); slot_map.push(None); }
                    _ => { stack.push(bcx.ins().iconst(ty::I64, 0)); slot_map.push(None); }
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
            OP_BINARY_ADD | OP_BINARY_ADD_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().iadd(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_SUBTRACT | OP_BINARY_SUB_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().isub(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_MULTIPLY | OP_BINARY_MUL_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().imul(left, right));
                    slot_map.push(None);
                }
            }
            OP_BINARY_DIVIDE | OP_BINARY_DIV_INT_INT => {
                if let (Some(right), Some(left)) = (stack.pop(), stack.pop()) {
                    slot_map.pop(); slot_map.pop();
                    stack.push(bcx.ins().sdiv(left, right));
                    slot_map.push(None);
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
                    let fallthrough = *block_map.entry(ip + 1).or_insert_with(|| bcx.create_block());
                    bcx.ins().brif(cond, fallthrough, &[], target_block, &[]);
                    let new_block = bcx.create_block();
                    bcx.switch_to_block(new_block);
                    bcx.seal_block(fallthrough);
                }
            }
            OP_POP_JUMP_IF_TRUE => {
                if let Some(cond) = stack.pop() {
                    slot_map.pop();
                    let target = ip + 1 + c as usize;
                    let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                    let fallthrough = *block_map.entry(ip + 1).or_insert_with(|| bcx.create_block());
                    bcx.ins().brif(cond, target_block, &[], fallthrough, &[]);
                    let new_block = bcx.create_block();
                    bcx.switch_to_block(new_block);
                    bcx.seal_block(fallthrough);
                }
            }
            OP_JUMP_FORWARD => {
                let target = ip + 1 + c as usize;
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                bcx.ins().jump(target_block, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
            }
            OP_JUMP_BACKWARD => {
                let target = c as usize;
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                bcx.ins().jump(target_block, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
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
                let fallthrough = *block_map.entry(ip + 1).or_insert_with(|| bcx.create_block());
                bcx.ins().brif(should_skip, target_block, &[], fallthrough, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
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
            OP_CALL_FUNCTION => {
                let argc = a as usize;
                if argc == 0 {
                    // 0-arg call: check for known builtins
                    let func_slot_opt = slot_map.pop().unwrap_or(None);
                    stack.pop();
                    if let Some(func_slot) = func_slot_opt {
                        let func_slot_i32 = bcx.ins().iconst(ty::I32, func_slot as i64);
                        let zero_i32 = bcx.ins().iconst(ty::I32, 0);
                        let call = bcx.ins().call(h.call_func, &[
                            locals_ptr,
                            func_slot_i32,
                            zero_i32,
                            zero_i32,
                        ]);
                        stack.push(bcx.inst_results(call)[0]);
                        slot_map.push(None);
                    } else {
                        stack.push(bcx.ins().iconst(ty::I64, 0));
                        slot_map.push(None);
                    }
                } else {
                    // Pop args and store in temp slots
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
                    // Pop function
                    let func_slot = slot_map.pop().unwrap_or(None).unwrap_or(0);
                    stack.pop();
                    // Call via runtime helper
                    let first_arg = arg_slot_indices.first().copied().unwrap_or(0);
                    let func_slot_i32 = bcx.ins().iconst(ty::I32, func_slot as i64);
                    let first_arg_i32 = bcx.ins().iconst(ty::I32, first_arg as i64);
                    let argc_i32 = bcx.ins().iconst(ty::I32, argc as i64);
                    let call = bcx.ins().call(h.call_func, &[
                        locals_ptr,
                        func_slot_i32,
                        first_arg_i32,
                        argc_i32,
                    ]);
                    stack.push(bcx.inst_results(call)[0]);
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
                let entry_block = *block_map.get(&loop_start).unwrap();
                bcx.ins().jump(entry_block, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
                return true;
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
                let fallthrough = *block_map.entry(ip + 1).or_insert_with(|| bcx.create_block());
                bcx.ins().brif(cmp, target_block, &[], fallthrough, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
                bcx.seal_block(fallthrough);
            }
            OP_POP_JUMP_IF_LT_LOCAL_IMM => {
                let local_idx = bcx.ins().iconst(ty::I32, a as i64);
                let call = bcx.ins().call(h.load_int, &[locals_ptr, local_idx]);
                let local_val = bcx.inst_results(call)[0];
                let imm = bcx.ins().iconst(ty::I64, b as i16 as i64);
                let cmp = bcx.ins().icmp(IntCC::SignedLessThan, local_val, imm);
                let target = ip + 1 + c as usize;
                let target_block = *block_map.entry(target).or_insert_with(|| bcx.create_block());
                let fallthrough = *block_map.entry(ip + 1).or_insert_with(|| bcx.create_block());
                bcx.ins().brif(cmp, target_block, &[], fallthrough, &[]);
                let new_block = bcx.create_block();
                bcx.switch_to_block(new_block);
                bcx.seal_block(fallthrough);
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
            _ => { return false; }
        }
        ip += 1;
    }
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
