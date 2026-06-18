use arabi_parser::ast::*;
use arabi_core::error::{ArabiError, Result};
use crate::bytecode::*;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,
}

#[derive(Debug, Clone)]
enum FPart {
    Literal(String),
    Expr(String, Option<String>),
}

struct LoopContext {
    break_jumps: Vec<usize>,
    continue_jumps: Vec<usize>,
    continue_target: usize,
}

pub struct Compiler {
    constants: Vec<Value>,
    names: Vec<String>,
    name_index: std::collections::HashMap<String, usize>,
    instructions: Vec<Instruction>,
    local_names: Vec<String>,
    local_map: std::collections::HashMap<String, usize>,
    num_locals: usize,
    loop_stack: Vec<LoopContext>,
    string_pool: std::collections::HashMap<String, usize>,
    global_names: HashSet<String>,
    loop_depth: usize,
    // Closure support: stack of enclosing scope local maps
    enclosing_local_maps: Vec<std::collections::HashMap<String, usize>>,
    // Names declared as nonlocal in current function
    nonlocal_names: HashSet<String>,
    // Free variables collected during compilation: (name, outer_local_idx, inner_local_idx)
    free_vars: Vec<(String, usize, usize)>,
    // Per-function pending capture stack: (name, original_outer_idx, parent_outer_idx)
    // Each nesting level has its own Vec. Pushed in compile_function_def, popped at end.
    pending_captures_stack: Vec<Vec<(String, usize, usize)>>,
    self_local: Option<usize>,
    current_line: usize,
    // Class field layouts for compile-time field offset resolution
    // Maps class name → (field_name, offset) pairs
    class_field_map: std::collections::HashMap<String, Vec<(String, usize)>>,
    // Current class being compiled (for self.field offset resolution)
    current_class: Option<String>,
    // True when compiling inside __تهيئة__ — don't use compile-time offsets for writes
    in_constructor: bool,
    // Current function name (for TCO: detect self-recursive tail calls)
    current_function_name: Option<String>,
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            constants: Vec::new(),
            names: Vec::new(),
            name_index: std::collections::HashMap::new(),
            instructions: Vec::new(),
            local_names: Vec::new(),
            local_map: std::collections::HashMap::new(),
            num_locals: 0,
            loop_stack: Vec::new(),
            string_pool: std::collections::HashMap::new(),
            global_names: HashSet::new(),
            loop_depth: 0,
            enclosing_local_maps: Vec::new(),
            nonlocal_names: HashSet::new(),
            free_vars: Vec::new(),
            pending_captures_stack: Vec::new(),
            self_local: None,
            current_line: 1,
            class_field_map: std::collections::HashMap::new(),
            current_class: None,
            in_constructor: false,
            current_function_name: None,
        }
    }

    fn get_or_create_local(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.local_map.get(name) {
            idx
        } else {
            let idx = self.num_locals;
            self.local_map.insert(name.to_string(), idx);
            self.local_names.push(name.to_string());
            self.num_locals += 1;
            idx
        }
    }

    /// Find a variable in enclosing scopes. Returns (outer_local_idx, inner_local_idx, free_var_index).
    fn find_enclosing_var(&mut self, name: &str) -> Option<(usize, usize, usize)> {
        let num_enclosing = self.enclosing_local_maps.len();
        if num_enclosing == 0 {
            return None;
        }
        // Search enclosing scopes from innermost to outermost
        for (scope_idx, enclosing_map) in self.enclosing_local_maps.iter().enumerate().rev() {
            if let Some(&outer_idx) = enclosing_map.get(name) {
                let immediate_parent = num_enclosing - 1;
                if scope_idx < immediate_parent {
                    // Found in grandparent scope — propagate through intermediate scopes
                    // Use __closure_ prefix to match what the parent function actually has in its local map
                    let closure_key = format!("__closure_{}", name);
                    for intermediate in (scope_idx..immediate_parent).rev() {
                        let target = intermediate + 1;
                        if !self.enclosing_local_maps[target].contains_key(&closure_key) {
                            let idx = self.enclosing_local_maps[target].len();
                            self.enclosing_local_maps[target].insert(closure_key.clone(), idx);
                        }
                    }
                    // Now find the __closure_ key in the immediate parent's map
                    let parent_outer_idx = *self.enclosing_local_maps[immediate_parent].get(&closure_key)?;
                    // Record pending capture for the intermediate function (parent level)
                    let parent_depth = self.pending_captures_stack.len().saturating_sub(2);
                    if parent_depth < self.pending_captures_stack.len() {
                        self.pending_captures_stack[parent_depth].push((name.to_string(), outer_idx, parent_outer_idx));
                    }
                    // Check if already captured
                    if let Some(pos) = self.free_vars.iter().position(|(n, _, _)| n == name) {
                        let inner_idx = self.free_vars[pos].2;
                        return Some((parent_outer_idx, inner_idx, pos));
                    }
                    let inner_idx = self.get_or_create_local(&format!("__closure_{}", name));
                    let free_idx = self.free_vars.len();
                    self.free_vars.push((name.to_string(), parent_outer_idx, inner_idx));
                    return Some((parent_outer_idx, inner_idx, free_idx));
                }
                // Found in immediate parent or module scope
                if let Some(pos) = self.free_vars.iter().position(|(n, _, _)| n == name) {
                    let inner_idx = self.free_vars[pos].2;
                    return Some((outer_idx, inner_idx, pos));
                }
                let inner_idx = self.get_or_create_local(&format!("__closure_{}", name));
                let free_idx = self.free_vars.len();
                self.free_vars.push((name.to_string(), outer_idx, inner_idx));
                return Some((outer_idx, inner_idx, free_idx));
            }
        }
        None
    }

    pub fn compile(&mut self, program: &Program) -> Result<BytecodeModule> {
        // Pre-scan: register all module-level function and class names as globals
        for stmt in &program.stmts {
            match stmt {
                Stmt::FunctionDef { name, .. } => {
                    self.global_names.insert(name.clone());
                }
                Stmt::ClassDef { name, .. } => {
                    self.global_names.insert(name.clone());
                }
                _ => {}
            }
        }
        for (i, stmt) in program.stmts.iter().enumerate() {
            self.current_line = program.stmt_lines.get(i).copied().unwrap_or(1);
            self.compile_stmt(stmt)?;
        }
        self.emit(Opcode::Halt, 0);

        self.peephole_optimize();

        let mut module = BytecodeModule {
            instructions: self.instructions.clone(),
            constants: self.constants.clone(),
            names: self.names.clone(),
            num_locals: self.num_locals,
            local_names: self.local_names.clone(),
            local_name_map: self.local_names.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect(),
            global_names: self.global_names.iter().cloned().collect(),
            packed: Vec::new(),
            lines: Vec::new(),
        };
        module.pack_all();
        Ok(module)
    }

    fn peephole_optimize(&mut self) {
        let len = self.instructions.len();
        if len == 0 { return; }

        // Pass 1: Dead store elimination — Remove redundant StoreFast to same local
        let mut i = 0;
        while i + 1 < len {
            if let (Opcode::StoreFast(a), Opcode::StoreFast(b)) = (&self.instructions[i].opcode, &self.instructions[i + 1].opcode) {
                if a == b {
                    self.instructions[i].opcode = Opcode::Nop;
                }
            }
            i += 1;
        }

        // Pass 2: Load followed by PopTop is dead code (for non-effectful loads)
        let len = self.instructions.len();
        for i in 0..len {
            if matches!(self.instructions[i].opcode, Opcode::PopTop) && i > 0 {
                match &self.instructions[i - 1].opcode {
                    Opcode::LoadConst(_) | Opcode::LoadFast(_) | Opcode::LoadTrue | Opcode::LoadFalse | Opcode::LoadNone => {
                        self.instructions[i - 1].opcode = Opcode::Nop;
                        self.instructions[i].opcode = Opcode::Nop;
                    }
                    _ => {}
                }
            }
        }

        // Pass 3: PopTop after Print is not dead (has side effect), skip
        // Pass 4: Duplicate StoreFast with no intervening writes
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let Opcode::StoreFast(a) = &self.instructions[i].opcode {
                if matches!(&self.instructions[i + 1].opcode, Opcode::PopTop) {
                    if let Opcode::StoreFast(b) = &self.instructions[i + 2].opcode {
                        if a == b {
                            self.instructions[i].opcode = Opcode::Nop;
                            self.instructions[i + 1].opcode = Opcode::Nop;
                        }
                    }
                }
            }
            i += 1;
        }

        // Pass 5: Fuse LoadFast + LoadConst(int) + BinarySubtract → SubtractLocalImm
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let (Opcode::LoadFast(local), Opcode::LoadConst(cidx), Opcode::BinarySubtract) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode)
            {
                if let Value::Integer(imm) = self.constants[*cidx] {
                    let local_idx = *local;
                    self.instructions[i].opcode = Opcode::SubtractLocalImm(local_idx, imm);
                    self.instructions[i + 1].opcode = Opcode::Nop;
                    self.instructions[i + 2].opcode = Opcode::Nop;
                    i += 3;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 6: Fuse LoadFast + LoadConst(int) + BinaryAdd → AddLocalImm
        // MOVED: now runs after passes 10-13 to avoid Nops breaking subscript patterns
        // Will be placed after pass 13

        // Pass 7: Fuse LoadFast + LoadConst(int) + CompareLtEq + PopJumpIfFalse → PopJumpIfLeLocalImm
        let len = self.instructions.len();
        let mut i = 0;
        while i + 3 < len {
            if let (Opcode::LoadFast(local), Opcode::LoadConst(cidx), Opcode::CompareLtEq, Opcode::PopJumpIfFalse(target)) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode, &self.instructions[i + 3].opcode)
            {
                if let Value::Integer(imm) = self.constants[*cidx] {
                    let local_idx = *local;
                    // PopJumpIfFalse uses relative offset: ip = ip - 1 + offset
                    // At position i+3, ip becomes i+4, then ip = i+4-1+offset = i+3+offset
                    // Convert to absolute IP for our fused opcode
                    let abs_target = (i + 3) + *target;
                    self.instructions[i].opcode = Opcode::PopJumpIfLeLocalImm(local_idx, imm, abs_target);
                    self.instructions[i + 1].opcode = Opcode::Nop;
                    self.instructions[i + 2].opcode = Opcode::Nop;
                    self.instructions[i + 3].opcode = Opcode::Nop;
                    i += 4;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 8: Fuse LoadFast + LoadConst(int) + CompareLt + PopJumpIfFalse → PopJumpIfLtLocalImm
        let len = self.instructions.len();
        let mut i = 0;
        while i + 3 < len {
            if let (Opcode::LoadFast(local), Opcode::LoadConst(cidx), Opcode::CompareLt, Opcode::PopJumpIfFalse(target)) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode, &self.instructions[i + 3].opcode)
            {
                if let Value::Integer(imm) = self.constants[*cidx] {
                    let local_idx = *local;
                    let abs_target = (i + 3) + *target;
                    self.instructions[i].opcode = Opcode::PopJumpIfLtLocalImm(local_idx, imm, abs_target);
                    self.instructions[i + 1].opcode = Opcode::Nop;
                    self.instructions[i + 2].opcode = Opcode::Nop;
                    self.instructions[i + 3].opcode = Opcode::Nop;
                    i += 4;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 9: Fuse LoadFast + LoadFast + BinaryModulo + LoadConst(0) + CompareEq + PopJumpIfFalse
        // → ModJumpIfNotZero(local_a, local_b, target)
        let len = self.instructions.len();
        let mut i = 0;
        while i + 5 < len {
            if let (Opcode::LoadFast(a), Opcode::LoadFast(b), Opcode::BinaryModulo, Opcode::LoadConst(cidx), Opcode::CompareEq, Opcode::PopJumpIfFalse(target)) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode, &self.instructions[i + 3].opcode, &self.instructions[i + 4].opcode, &self.instructions[i + 5].opcode)
            {
                if let Value::Integer(0) = self.constants[*cidx] {
                    let local_a = *a;
                    let local_b = *b;
                    let abs_target = (i + 5) + *target;
                    self.instructions[i].opcode = Opcode::ModJumpIfNotZero(local_a, local_b, abs_target);
                    self.instructions[i + 1].opcode = Opcode::Nop;
                    self.instructions[i + 2].opcode = Opcode::Nop;
                    self.instructions[i + 3].opcode = Opcode::Nop;
                    self.instructions[i + 4].opcode = Opcode::Nop;
                    self.instructions[i + 5].opcode = Opcode::Nop;
                    i += 6;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 10: Fuse LoadFast + LoadFast + Subscript → SubscriptLocal
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let (Opcode::LoadFast(a), Opcode::LoadFast(b), Opcode::Subscript) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode)
            {
                let la = *a;
                let lb = *b;
                self.instructions[i].opcode = Opcode::SubscriptLocal(la, lb);
                self.instructions[i + 1].opcode = Opcode::Nop;
                self.instructions[i + 2].opcode = Opcode::Nop;
                i += 3;
                continue;
            }
            i += 1;
        }

        // Pass 11: Fuse SubscriptLocal + LoadFast + Subscript → SubscriptLocal2D
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let (Opcode::SubscriptLocal(a, b), Opcode::LoadFast(c), Opcode::Subscript) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode)
            {
                let la = *a;
                let lb = *b;
                let lc = *c;
                self.instructions[i].opcode = Opcode::SubscriptLocal2D(la, lb, lc);
                self.instructions[i + 1].opcode = Opcode::Nop;
                self.instructions[i + 2].opcode = Opcode::Nop;
                i += 3;
                continue;
            }
            i += 1;
        }

        // Pass 12: Fuse LoadFast + LoadFast + StoreSubscript → StoreSubscriptLocal
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let (Opcode::LoadFast(a), Opcode::LoadFast(b), Opcode::StoreSubscript) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode)
            {
                let la = *a;
                let lb = *b;
                self.instructions[i].opcode = Opcode::StoreSubscriptLocal(la, lb);
                self.instructions[i + 1].opcode = Opcode::Nop;
                self.instructions[i + 2].opcode = Opcode::Nop;
                i += 3;
                continue;
            }
            i += 1;
        }

        // Pass 12b: Fuse SubscriptLocal + [Nops] + LoadFast + Subscript → SubscriptLocal2D
        // After pass 10, M[i][k] becomes: SubscriptLocal(A,B), Nop, Nop, LoadFast(C), Subscript
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let Opcode::SubscriptLocal(a, b) = &self.instructions[i].opcode {
                let la = *a;
                let lb = *b;
                let mut non_nops = Vec::new();
                let mut j = i + 1;
                while j < len && non_nops.len() < 2 {
                    if !matches!(self.instructions[j].opcode, Opcode::Nop) {
                        non_nops.push(j);
                    }
                    j += 1;
                }
                if non_nops.len() < 2 { i += 1; continue; }
                if let (Opcode::LoadFast(c), Opcode::Subscript) =
                    (&self.instructions[non_nops[0]].opcode, &self.instructions[non_nops[1]].opcode)
                {
                    let lc = *c;
                    self.instructions[i].opcode = Opcode::SubscriptLocal2D(la, lb, lc);
                    for k in (i + 1)..=non_nops[1] {
                        self.instructions[k].opcode = Opcode::Nop;
                    }
                    i = non_nops[1] + 1;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 13: Fuse SubscriptLocal + [Nops] + LoadFast + StoreSubscript → StoreSubscriptLocal2D
        // After pass 10, the pattern is: SubscriptLocal(A,B), Nop, Nop, LoadFast(C), StoreSubscript
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let Opcode::SubscriptLocal(a, b) = &self.instructions[i].opcode {
                let la = *a;
                let lb = *b;
                let mut non_nops = Vec::new();
                let mut j = i + 1;
                while j < len && non_nops.len() < 2 {
                    if !matches!(self.instructions[j].opcode, Opcode::Nop) {
                        non_nops.push(j);
                    }
                    j += 1;
                }
                if non_nops.len() < 2 { i += 1; continue; }
                if let (Opcode::LoadFast(c), Opcode::StoreSubscript) =
                    (&self.instructions[non_nops[0]].opcode, &self.instructions[non_nops[1]].opcode)
                {
                    let lc = *c;
                    self.instructions[i].opcode = Opcode::StoreSubscriptLocal2D(la, lb, lc);
                    for k in (i + 1)..=non_nops[1] {
                        self.instructions[k].opcode = Opcode::Nop;
                    }
                    i = non_nops[1] + 1;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 6 (moved here from before pass 7): Fuse LoadFast + LoadConst(int) + BinaryAdd → AddLocalImm
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let (Opcode::LoadFast(local), Opcode::LoadConst(cidx), Opcode::BinaryAdd) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode)
            {
                if let Value::Integer(imm) = self.constants[*cidx] {
                    let local_idx = *local;
                    self.instructions[i].opcode = Opcode::AddLocalImm(local_idx, imm);
                    self.instructions[i + 1].opcode = Opcode::Nop;
                    self.instructions[i + 2].opcode = Opcode::Nop;
                    i += 3;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 14: Fuse LoadFast + AddLocalImm + Subscript → SubscriptAddImm
        // NOTE: With pass 6 moved after pass 13, AddLocalImm now has 2 Nops before Subscript.
        // Use NOP-skip to bridge the 2-Nop gap.
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let (Opcode::LoadFast(list_local), Opcode::AddLocalImm(j_local, imm)) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode)
            {
                let li = *list_local;
                let lj = *j_local;
                let imm_val = *imm;
                let mut sub_pos = i + 2;
                while sub_pos < len && sub_pos <= i + 4 && matches!(self.instructions[sub_pos].opcode, Opcode::Nop) {
                    sub_pos += 1;
                }
                if sub_pos < len && matches!(self.instructions[sub_pos].opcode, Opcode::Subscript) {
                    self.instructions[i].opcode = Opcode::SubscriptAddImm(li, lj, imm_val);
                    for k in (i + 1)..=sub_pos {
                        self.instructions[k].opcode = Opcode::Nop;
                    }
                    i = sub_pos + 1;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 15: Fuse LoadFast + AddLocalImm + StoreSubscript → StoreSubscriptAddImm
        // NOTE: Same NOP-skip as pass 14 for the 2-Nop gap from pass 6.
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let (Opcode::LoadFast(list_local), Opcode::AddLocalImm(j_local, imm)) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode)
            {
                let li = *list_local;
                let lj = *j_local;
                let imm_val = *imm;
                let mut sub_pos = i + 2;
                while sub_pos < len && sub_pos <= i + 4 && matches!(self.instructions[sub_pos].opcode, Opcode::Nop) {
                    sub_pos += 1;
                }
                if sub_pos < len && matches!(self.instructions[sub_pos].opcode, Opcode::StoreSubscript) {
                    self.instructions[i].opcode = Opcode::StoreSubscriptAddImm(li, lj, imm_val);
                    for k in (i + 1)..=sub_pos {
                        self.instructions[k].opcode = Opcode::Nop;
                    }
                    i = sub_pos + 1;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 16: Fuse SubscriptLocal2D + LoadFast + BinaryAdd + StoreSubscriptLocal2D → AddToSubscript2D
        // After passes 12b and 13, the pattern is:
        //   SubscriptLocal2D(A,B,C), LoadFast(D), BinaryAdd, StoreSubscriptLocal2D(A,B,C)
        // → LoadFast(D), AddToSubscript2D(A,B,C,D)
        let len = self.instructions.len();
        let mut i = 0;
        while i + 3 < len {
            if let (Opcode::SubscriptLocal2D(a1, b1, c1),
                     Opcode::LoadFast(d),
                     Opcode::BinaryAdd,
                     Opcode::StoreSubscriptLocal2D(a2, b2, c2)) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode,
                 &self.instructions[i + 2].opcode, &self.instructions[i + 3].opcode)
            {
                if a1 == a2 && b1 == b2 && c1 == c2 {
                    let la = *a1;
                    let lb = *b1;
                    let lc = *c1;
                    let ld = *d;
                    self.instructions[i].opcode = Opcode::LoadFast(ld);
                    self.instructions[i + 1].opcode = Opcode::AddToSubscript2D(la, lb, lc, ld);
                    self.instructions[i + 2].opcode = Opcode::Nop;
                    self.instructions[i + 3].opcode = Opcode::Nop;
                    i += 4;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 17: Fuse swap pattern → SwapAdjacent (skip Nops between instructions)
        // Actual 10-instruction pattern after passes 10-16:
        //   SubscriptLocal(A,B), StoreFast(T), LoadFast(A), LoadFast(B), SubscriptAddImm(A,B,1),
        //   StoreSubscript, LoadFast(A), AddLocalImm(B,1), LoadFast(T), StoreSubscript
        // → SwapAdjacent(A, B)
        let len = self.instructions.len();
        let mut i = 0;
        while i < len {
            if !matches!(self.instructions[i].opcode, Opcode::SubscriptLocal(_, _)) {
                i += 1;
                continue;
            }
            let mut positions = Vec::new();
            let mut j = i;
            while j < len && positions.len() < 10 {
                if !matches!(self.instructions[j].opcode, Opcode::Nop) {
                    positions.push(j);
                }
                j += 1;
            }
            if positions.len() < 10 { i += 1; continue; }

            if let (Opcode::SubscriptLocal(a, b),
                     Opcode::StoreFast(t),
                     Opcode::LoadFast(al1),
                     Opcode::LoadFast(bl1),
                     Opcode::SubscriptAddImm(a2, b2, imm),
                     Opcode::StoreSubscript,
                     Opcode::LoadFast(al2),
                     Opcode::AddLocalImm(bl2, imm2),
                     Opcode::LoadFast(t2),
                     Opcode::StoreSubscript) =
                (&self.instructions[positions[0]].opcode, &self.instructions[positions[1]].opcode,
                 &self.instructions[positions[2]].opcode, &self.instructions[positions[3]].opcode,
                 &self.instructions[positions[4]].opcode, &self.instructions[positions[5]].opcode,
                 &self.instructions[positions[6]].opcode, &self.instructions[positions[7]].opcode,
                 &self.instructions[positions[8]].opcode, &self.instructions[positions[9]].opcode)
            {
                if a == a2 && al1 == a && al2 == a && bl1 == b && bl2 == b && b == b2
                    && t == t2 && *imm == 1 && *imm2 == 1
                {
                    let la = *a;
                    let lb = *b;
                    for k in i..=positions[9] {
                        self.instructions[k].opcode = Opcode::Nop;
                    }
                    self.instructions[i].opcode = Opcode::SwapAdjacent(la, lb);
                    i = positions[9] + 1;
                    continue;
                }
            }
            i += 1;
        }

        // Pass 18: Fuse SubscriptLocal + LoadFast + StoreSubscript → StoreSubscriptLocal2D
        // Pattern: SubscriptLocal(A, B) + LoadFast(C) + StoreSubscript
        let len = self.instructions.len();
        let mut i = 0;
        while i + 2 < len {
            if let (Opcode::SubscriptLocal(a, b), Opcode::LoadFast(c), Opcode::StoreSubscript) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode)
            {
                let la = *a;
                let lb = *b;
                let lc = *c;
                self.instructions[i].opcode = Opcode::StoreSubscriptLocal2D(la, lb, lc);
                self.instructions[i + 1].opcode = Opcode::Nop;
                self.instructions[i + 2].opcode = Opcode::Nop;
                i += 3;
                continue;
            }
            i += 1;
        }

        // Pass 19: Fuse tight AddToSubscript2D pattern (with NOP-skip)
        // Pattern: SubscriptLocal2D(A,B,C) + LoadFast(D) + BinaryAdd + StoreSubscriptLocal2D(A,B,C)
        // → LoadFast(D) + AddToSubscript2D(A,B,C,D)
        let len = self.instructions.len();
        let mut i = 0;
        let mut pass19_count = 0usize;
        while i < len {
            if !matches!(self.instructions[i].opcode, Opcode::SubscriptLocal2D(_, _, _)) {
                i += 1;
                continue;
            }
            let mut positions = Vec::new();
            let mut j = i;
            while j < len && positions.len() < 4 {
                if !matches!(self.instructions[j].opcode, Opcode::Nop) {
                    positions.push(j);
                }
                j += 1;
            }
            if positions.len() < 4 { i += 1; continue; }

            if let (Opcode::SubscriptLocal2D(a, b, c),
                     Opcode::LoadFast(d),
                     Opcode::BinaryAdd,
                     Opcode::StoreSubscriptLocal2D(a2, b2, c2)) =
                (&self.instructions[positions[0]].opcode, &self.instructions[positions[1]].opcode,
                 &self.instructions[positions[2]].opcode, &self.instructions[positions[3]].opcode)
            {
                if a == a2 && b == b2 && c == c2 {
                    let la = *a;
                    let lb = *b;
                    let lc = *c;
                    let ld = *d;
                    for k in i..=positions[3] {
                        self.instructions[k].opcode = Opcode::Nop;
                    }
                    self.instructions[i].opcode = Opcode::LoadFast(ld);
                    self.instructions[i + 1].opcode = Opcode::AddToSubscript2D(la, lb, lc, ld);
                    pass19_count += 1;
                    i = positions[3] + 1;
                    continue;
                }
            }
            i += 1;
        }
        if pass19_count > 0 {
            eprintln!("[peephole] Pass 19: fused {} AddToSubscript2D", pass19_count);
        }

        // Pass 20: Fuse SubscriptLocal + [Nops] + SubscriptAddImm + CompareGt + PopJumpIfFalse (NOP-skip)
        // → PopJumpIfSubscriptGt(list, idx, imm, target)
        // PopJumpIfFalse uses ip = ip - 1 + c (relative), so absolute target = pos + c
        let len = self.instructions.len();
        let mut i = 0;
        while i + 4 < len {
            if !matches!(self.instructions[i].opcode, Opcode::SubscriptLocal(_, _)) {
                i += 1;
                continue;
            }
            if let Opcode::SubscriptLocal(a_local, b_local) = &self.instructions[i].opcode {
                let la = *a_local;
                let lb = *b_local;
                let mut non_nops = Vec::new();
                let mut j = i + 1;
                while j < len && non_nops.len() < 3 {
                    if !matches!(self.instructions[j].opcode, Opcode::Nop) {
                        non_nops.push(j);
                    }
                    j += 1;
                }
                if non_nops.len() < 3 { i += 1; continue; }
                if let (Opcode::SubscriptAddImm(a2, b2, imm),
                         Opcode::CompareGt,
                         Opcode::PopJumpIfFalse(target)) =
                    (&self.instructions[non_nops[0]].opcode,
                     &self.instructions[non_nops[1]].opcode,
                     &self.instructions[non_nops[2]].opcode)
                {
                    if la == *a2 && lb == *b2 {
                        let abs_target = non_nops[2] + *target;
                        self.instructions[i].opcode = Opcode::PopJumpIfSubscriptGt(la, lb, *imm, abs_target);
                        for k in i..=non_nops[2] {
                            self.instructions[k].opcode = Opcode::Nop;
                        }
                        i = non_nops[2] + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }

        // Pass 21a: Fuse pattern with NOP-skip: LoadFast(src) + LoadConst(mod) + BinaryModulo + LoadConst(0) + CompareEq + PopJumpIfFalse(skip) + [nops] + AddLocalImm(counter, 1) + [nops] + StoreFast(counter)
        // → ModAddIfZero(src, mod_const_idx, counter) — reduces many instructions to 1
        let len = self.instructions.len();
        let mut i = 0;
        let mut pass21_count = 0usize;
        while i + 5 < len {
            if let (Opcode::LoadFast(src), Opcode::LoadConst(mod_cidx), Opcode::BinaryModulo,
                     Opcode::LoadConst(zero_cidx), Opcode::CompareEq, Opcode::PopJumpIfFalse(_skip)) =
                (&self.instructions[i].opcode, &self.instructions[i + 1].opcode, &self.instructions[i + 2].opcode,
                 &self.instructions[i + 3].opcode, &self.instructions[i + 4].opcode, &self.instructions[i + 5].opcode)
            {
                if let Value::Integer(0) = &self.constants[*zero_cidx] {
                    // Find next non-Nop after PopJumpIfFalse
                    let mut found_add = false;
                    let mut add_imm = 0i64;
                    let mut add_counter = 0usize;
                    let mut found_store = false;
                    let mut store_counter = 0usize;
                    let mut j = i + 6;
                    while j < len && (!found_add || !found_store) {
                        if matches!(self.instructions[j].opcode, Opcode::Nop) {
                            j += 1;
                            continue;
                        }
                        if !found_add {
                            if let Opcode::AddLocalImm(counter, imm) = &self.instructions[j].opcode {
                                add_imm = *imm;
                                add_counter = *counter;
                                found_add = true;
                                j += 1;
                                continue;
                            } else {
                                break;
                            }
                        }
                        if !found_store {
                            if let Opcode::StoreFast(counter) = &self.instructions[j].opcode {
                                store_counter = *counter;
                                found_store = true;
                                break;
                            } else {
                                break;
                            }
                        }
                    }
                    if found_add && found_store && add_counter == store_counter && add_imm == 1 {
                        let la = *src;
                        let lb = *mod_cidx;
                        let lc = add_counter;
                        let end = j + 1; // include StoreFast
                        for k in i..end {
                            self.instructions[k].opcode = Opcode::Nop;
                        }
                        self.instructions[i].opcode = Opcode::ModAddIfZero(la, lb, lc);
                        pass21_count += 1;
                        i = end;
                        continue;
                    }
                }
            }
            i += 1;
        }
        if pass21_count > 0 {
            eprintln!("[peephole] Pass 21: fused {} ModAddIfZero (full pattern)", pass21_count);
        }

        // Pass 21b: Fuse ModJumpIfNotZero + LOAD_FAST(counter) + LOAD_CONST(1) + BINARY_ADD + STORE_FAST(counter)
        // → ModAddIfZero(source, modulus_local, counter) — 5 instructions → 1
        let len = self.instructions.len();
        let mut i = 0;
        while i < len {
            if !matches!(self.instructions[i].opcode, Opcode::ModJumpIfNotZero(_, _, _)) {
                i += 1;
                continue;
            }
            let mut positions = Vec::new();
            let mut j = i + 1;
            while j < len && positions.len() < 4 {
                if !matches!(self.instructions[j].opcode, Opcode::Nop) {
                    positions.push(j);
                }
                j += 1;
            }
            if positions.len() < 4 { i += 1; continue; }

            if let (Opcode::ModJumpIfNotZero(src, modulus, _skip_target),
                     Opcode::LoadFast(counter),
                     Opcode::LoadConst(cidx),
                     Opcode::BinaryAdd,
                     Opcode::StoreFast(counter2)) =
                (&self.instructions[i].opcode,
                 &self.instructions[positions[0]].opcode,
                 &self.instructions[positions[1]].opcode,
                 &self.instructions[positions[2]].opcode,
                 &self.instructions[positions[3]].opcode)
            {
                if counter == counter2 {
                    if let Value::Integer(1) = self.constants[*cidx] {
                        let la = *src;
                        let lb = *modulus;
                        let lc = *counter;
                        for k in i..=positions[3] {
                            self.instructions[k].opcode = Opcode::Nop;
                        }
                        self.instructions[i].opcode = Opcode::ModAddIfZero(la, lb, lc);
                        i = positions[3] + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }

        // Pass 23: Float fusion — fuse float multiply-add patterns
        let pass23_count;
        {
            let len = self.instructions.len();
            let mut count = 0;

            // Pass 23a: NegFloorDivSqrAddImm (8-instruction pattern)
            let mut i = 0;
            while i + 7 < len {
                let op0 = self.instructions[i].opcode.clone();
                let op1 = self.instructions[i+1].opcode.clone();
                let op2 = self.instructions[i+2].opcode.clone();
                let op3 = self.instructions[i+3].opcode.clone();
                let op4 = self.instructions[i+4].opcode.clone();
                let op5 = self.instructions[i+5].opcode.clone();
                let op6 = self.instructions[i+6].opcode.clone();
                let op7 = self.instructions[i+7].opcode.clone();
                if let (
                    Opcode::LoadConst(neg1_idx), Opcode::LoadFast(b), Opcode::LoadFast(b2),
                    Opcode::BinaryMultiply, Opcode::LoadConst(eps_idx), Opcode::BinaryAdd,
                    Opcode::BinaryFloorDivide, Opcode::StoreFast(a),
                ) = (&op0, &op1, &op2, &op3, &op4, &op5, &op6, &op7) {
                    if b == b2 {
                        if let (Value::Float(neg1), Value::Float(eps)) = (&self.constants[*neg1_idx], &self.constants[*eps_idx]) {
                            if (*neg1 - (-1.0_f64)).abs() < 1e-10 && *eps > 0.0 {
                                self.instructions[i].opcode = Opcode::Nop;
                                self.instructions[i+1].opcode = Opcode::Nop;
                                self.instructions[i+2].opcode = Opcode::Nop;
                                self.instructions[i+3].opcode = Opcode::Nop;
                                self.instructions[i+4].opcode = Opcode::Nop;
                                self.instructions[i+5].opcode = Opcode::Nop;
                                self.instructions[i+6].opcode = Opcode::Nop;
                                self.instructions[i+7].opcode = Opcode::NegFloorDivSqrAddImm(*a, *b, *eps_idx);
                                count += 1;
                                i += 8;
                                continue;
                            }
                        }
                    }
                }
                i += 1;
            }

            // Pass 23b: FloatAddMulLocal and FloatAddMulImm (6-instruction patterns)
            let len = self.instructions.len();
            let mut i = 0;
            while i + 5 < len {
                let op0 = self.instructions[i].opcode.clone();
                let op1 = self.instructions[i+1].opcode.clone();
                let op2 = self.instructions[i+2].opcode.clone();
                let op3 = self.instructions[i+3].opcode.clone();
                let op4 = self.instructions[i+4].opcode.clone();
                let op5 = self.instructions[i+5].opcode.clone();
                if let (
                    Opcode::LoadFast(a), Opcode::LoadFast(b), Opcode::LoadFast(c),
                    Opcode::BinaryMultiply, Opcode::BinaryAdd, Opcode::StoreFast(a2),
                ) = (&op0, &op1, &op2, &op3, &op4, &op5) {
                    if a == a2 && b != a && c != a {
                        self.instructions[i].opcode = Opcode::Nop;
                        self.instructions[i+1].opcode = Opcode::Nop;
                        self.instructions[i+2].opcode = Opcode::Nop;
                        self.instructions[i+3].opcode = Opcode::Nop;
                        self.instructions[i+4].opcode = Opcode::Nop;
                        self.instructions[i+5].opcode = Opcode::FloatAddMulLocal(*a, *b, *c);
                        count += 1;
                        i += 6;
                        continue;
                    }
                }
                if let (
                    Opcode::LoadFast(a), Opcode::LoadFast(b), Opcode::LoadConst(c),
                    Opcode::BinaryMultiply, Opcode::BinaryAdd, Opcode::StoreFast(a2),
                ) = (&op0, &op1, &op2, &op3, &op4, &op5) {
                    if a == a2 {
                        self.instructions[i].opcode = Opcode::Nop;
                        self.instructions[i+1].opcode = Opcode::Nop;
                        self.instructions[i+2].opcode = Opcode::Nop;
                        self.instructions[i+3].opcode = Opcode::Nop;
                        self.instructions[i+4].opcode = Opcode::Nop;
                        self.instructions[i+5].opcode = Opcode::FloatAddMulImm(*a, *b, *c);
                        count += 1;
                        i += 6;
                        continue;
                    }
                }
                i += 1;
            }
            pass23_count = count;
        }
        if pass23_count > 0 {
            eprintln!("[peephole] Pass 23: fused {} float multiply-add patterns", pass23_count);
        }

        // Pass 24: PopJumpIfLtLocal — fuse LoadFast + LoadFast + CompareLt + PopJumpIfFalse
        let pass24_count;
        {
            let len = self.instructions.len();
            let mut count = 0;
            let mut i = 0;
            while i + 3 < len {
                let op0 = self.instructions[i].opcode.clone();
                let op1 = self.instructions[i+1].opcode.clone();
                let op2 = self.instructions[i+2].opcode.clone();
                let op3 = self.instructions[i+3].opcode.clone();
                if let (Opcode::LoadFast(a), Opcode::LoadFast(b), Opcode::CompareLt, Opcode::PopJumpIfFalse(target)) =
                    (&op0, &op1, &op2, &op3) {
                    let abs_target = (i + 3) + target;
                    self.instructions[i].opcode = Opcode::Nop;
                    self.instructions[i+1].opcode = Opcode::Nop;
                    self.instructions[i+2].opcode = Opcode::Nop;
                    self.instructions[i+3].opcode = Opcode::PopJumpIfLtLocal(*a, *b, abs_target);
                    count += 1;
                    i += 4;
                    continue;
                }
                i += 1;
            }
            pass24_count = count;
        }
        if pass24_count > 0 {
            eprintln!("[peephole] Pass 24: fused {} PopJumpIfLtLocal", pass24_count);
        }

        // Pass 25: FloatSqrSubAddImm — fuse LoadFast(b)+LoadFast(b)+BinaryMultiply+LoadFast(c)+LoadFast(c)+BinaryMultiply+BinarySubtract+LoadFast(d)+BinaryAdd+StoreFast(a) → FloatSqrSubAddImm(a,b,c,d)
        let pass25_count;
        {
            let len = self.instructions.len();
            let mut count = 0;
            let mut i = 0;
            while i + 9 < len {
                let ops: Vec<_> = (0..10).map(|j| self.instructions[i+j].opcode.clone()).collect();
                if let (
                    Opcode::LoadFast(b1), Opcode::LoadFast(b2), Opcode::BinaryMultiply,
                    Opcode::LoadFast(c1), Opcode::LoadFast(c2), Opcode::BinaryMultiply,
                    Opcode::BinarySubtract, Opcode::LoadFast(d), Opcode::BinaryAdd, Opcode::StoreFast(a),
                ) = (&ops[0], &ops[1], &ops[2], &ops[3], &ops[4], &ops[5], &ops[6], &ops[7], &ops[8], &ops[9]) {
                    if b1 == b2 && c1 == c2 && b1 != c1 {
                        for j in 0..9 { self.instructions[i+j].opcode = Opcode::Nop; }
                        self.instructions[i+9].opcode = Opcode::FloatSqrSubAddImm(*a, *b1, *c1, *d);
                        count += 1;
                        i += 10;
                        continue;
                    }
                }
                i += 1;
            }
            pass25_count = count;
        }
        if pass25_count > 0 {
            eprintln!("[peephole] Pass 25: fused {} FloatSqrSubAddImm", pass25_count);
        }

        // Pass 26: FloatMulMulAddImm — fuse LoadConst(2.0)+LoadFast(b)+BinaryMultiply+LoadFast(c)+BinaryMultiply+LoadFast(d)+BinaryAdd+StoreFast(a) → FloatMulMulAddImm(a,b,c,d)
        let pass26_count;
        {
            let len = self.instructions.len();
            let mut count = 0;
            let mut i = 0;
            while i + 7 < len {
                let ops: Vec<_> = (0..8).map(|j| self.instructions[i+j].opcode.clone()).collect();
                if let (
                    Opcode::LoadConst(two_idx), Opcode::LoadFast(b), Opcode::BinaryMultiply,
                    Opcode::LoadFast(c), Opcode::BinaryMultiply, Opcode::LoadFast(d),
                    Opcode::BinaryAdd, Opcode::StoreFast(a),
                ) = (&ops[0], &ops[1], &ops[2], &ops[3], &ops[4], &ops[5], &ops[6], &ops[7]) {
                    if let Value::Float(two) = &self.constants[*two_idx] {
                        if (*two - 2.0_f64).abs() < 1e-10 {
                            for j in 0..8 { self.instructions[i+j].opcode = Opcode::Nop; }
                            self.instructions[i+7].opcode = Opcode::FloatMulMulAddImm(*a, *b, *c, *d);
                            count += 1;
                            i += 8;
                            continue;
                        }
                    }
                }
                i += 1;
            }
            pass26_count = count;
        }
        if pass26_count > 0 {
            eprintln!("[peephole] Pass 26: fused {} FloatMulMulAddImm", pass26_count);
        }

        // Pass 27: PopJumpIfNotSqrAddSqrGtImm — fuse LoadFast(a)+LoadFast(a)+BinaryMultiply+LoadFast(b)+LoadFast(b)+BinaryMultiply+BinaryAdd+LoadConst(c)+CompareGt+PopJumpIfFalse → PopJumpIfNotSqrAddSqrGtImm(a,b,c_idx,target)
        let pass27_count;
        {
            let len = self.instructions.len();
            let mut count = 0;
            let mut i = 0;
            while i + 9 < len {
                let ops: Vec<_> = (0..10).map(|j| self.instructions[i+j].opcode.clone()).collect();
                if let (
                    Opcode::LoadFast(a1), Opcode::LoadFast(a2), Opcode::BinaryMultiply,
                    Opcode::LoadFast(b1), Opcode::LoadFast(b2), Opcode::BinaryMultiply,
                    Opcode::BinaryAdd, Opcode::LoadConst(c_idx), Opcode::CompareGt, Opcode::PopJumpIfFalse(target),
                ) = (&ops[0], &ops[1], &ops[2], &ops[3], &ops[4], &ops[5], &ops[6], &ops[7], &ops[8], &ops[9]) {
                    if a1 == a2 && b1 == b2 && a1 != b1 {
                        let abs_target = (i + 9) + target;
                        for j in 0..10 { self.instructions[i+j].opcode = Opcode::Nop; }
                        self.instructions[i+9].opcode = Opcode::PopJumpIfNotSqrAddSqrGtImm(*a1, *b1, *c_idx, abs_target);
                        count += 1;
                        i += 10;
                        continue;
                    }
                }
                i += 1;
            }
            pass27_count = count;
        }
        if pass27_count > 0 {
            eprintln!("[peephole] Pass 27: fused {} PopJumpIfNotSqrAddSqrGtImm", pass27_count);
        }

        // Pass 28: Fuse SubscriptLocal(A,B) + LoadFast(C) + SubscriptLocal2D(A,B,C) + LoadFast(D) + BinaryAdd + StoreSubscript
        //     → AddToSubscript2D(A, B, C, D)
        // This handles the pattern: result[i][j] = result[i][j] + value (matrix multiply inner loop)
        // Supports NOPs between instructions from earlier passes.
        let pass28_count;
        {
            let len = self.instructions.len();
            let mut count = 0;
            let mut i = 0;
            while i < len {
                if !matches!(self.instructions[i].opcode, Opcode::SubscriptLocal(_, _)) {
                    i += 1;
                    continue;
                }
                // Collect the next 5 non-Nop positions
                let mut non_nop_positions = Vec::with_capacity(5);
                let mut j = i + 1;
                while j < len && non_nop_positions.len() < 5 {
                    if !matches!(self.instructions[j].opcode, Opcode::Nop) {
                        non_nop_positions.push(j);
                    }
                    j += 1;
                }
                if non_nop_positions.len() < 5 {
                    i += 1;
                    continue;
                }
                let p0 = i;
                let p1 = non_nop_positions[0];
                let p2 = non_nop_positions[1];
                let p3 = non_nop_positions[2];
                let p4 = non_nop_positions[3];
                let p5 = non_nop_positions[4];

                if let (
                    Opcode::SubscriptLocal(a1, b1),
                    Opcode::LoadFast(c_val),
                    Opcode::SubscriptLocal2D(a2, b2, c2),
                    Opcode::LoadFast(d),
                    Opcode::BinaryAdd,
                    Opcode::StoreSubscript,
                ) = (&self.instructions[p0].opcode, &self.instructions[p1].opcode, &self.instructions[p2].opcode, &self.instructions[p3].opcode, &self.instructions[p4].opcode, &self.instructions[p5].opcode)
                {
                    if a1 == a2 && b1 == b2 && c_val == c2 {
                        let la = *a1;
                        let lb = *b1;
                        let lc = *c_val;
                        let ld = *d;
                        for k in p0..=p5 {
                            self.instructions[k].opcode = Opcode::Nop;
                        }
                        self.instructions[p5].opcode = Opcode::AddToSubscript2D(la, lb, lc, ld);
                        count += 1;
                        i = p5 + 1;
                        continue;
                    }
                }
                i += 1;
            }
            pass28_count = count;
        }
        if pass28_count > 0 {
            eprintln!("[peephole] Pass 28: fused {} AddToSubscript2D (matrix inner loop)", pass28_count);
        }

        // Pass 29: Fuse CallMethod("اضف", 1) + PopTop → ListAppendLocal(list)
        // Scan BACKWARD from CallMethod to find the correct LoadFast(list).
        // Pattern: LoadFast(list) + <arg_expr> + CallMethod("اضف", 1) + PopTop
        //   → <arg_expr> + ListAppendLocal(list_local)
        // The arg expression has net forward stack effect +1. When backward depth reaches 0,
        // we're at the boundary between list and arg. The list is the LoadFast before that.
        let pass29_count;
        {
            let len = self.instructions.len();
            let mut count = 0;
            let mut i = 0;
            while i + 1 < len {
                if let Opcode::CallMethod(name_idx, 1) = self.instructions[i].opcode {
                    if name_idx < self.names.len() && self.names[name_idx] == "اضف" {
                        if matches!(self.instructions[i + 1].opcode, Opcode::PopTop) {
                            let cm_pos = i;
                            let mut depth: i32 = 1;
                            let mut found = false;
                            for back in (0..cm_pos).rev() {
                                let eff = match &self.instructions[back].opcode {
                                    Opcode::LoadFast(_) | Opcode::LoadConst(_) |
                                    Opcode::LoadTrue | Opcode::LoadFalse | Opcode::LoadNone |
                                    Opcode::LoadName(_) | Opcode::LoadGlobal(_) |
                                    Opcode::SubscriptLocal(_, _) |
                                    Opcode::BuildList(_) | Opcode::BuildDict(_) => 1,
                                    Opcode::BinaryAdd | Opcode::BinarySubtract |
                                    Opcode::BinaryMultiply | Opcode::BinaryDivide |
                                    Opcode::BinaryModulo | Opcode::BinaryFloorDivide |
                                    Opcode::BinaryPower |
                                    Opcode::CompareLt | Opcode::CompareGt |
                                    Opcode::CompareLtEq | Opcode::CompareGtEq |
                                    Opcode::CompareEq | Opcode::CompareNotEq |
                                    Opcode::LogicalAnd | Opcode::LogicalOr |
                                    Opcode::Subscript => -1,
                                    Opcode::CallFunction(n) => -(*n as i32),
                                    Opcode::CallMethod(_, n) => -(*n as i32),
                                    Opcode::Nop => 0,
                                    _ => 0,
                                };
                                depth -= eff;
                                if depth == 0 {
                                    // We've consumed the arg expression.
                                    // The list is the LoadFast BEFORE this point (back - 1).
                                    let mut list_pos = back;
                                    loop {
                                        if list_pos == 0 { break; }
                                        list_pos -= 1;
                                        match &self.instructions[list_pos].opcode {
                                            Opcode::Nop => continue,
                                            Opcode::LoadFast(l) => {
                                                let ll = *l;
                                                self.instructions[list_pos].opcode = Opcode::Nop;
                                                self.instructions[cm_pos].opcode = Opcode::ListAppendLocal(ll);
                                                self.instructions[cm_pos + 1].opcode = Opcode::Nop;
                                                count += 1;
                                                found = true;
                                                break;
                                            }
                                            _ => break,
                                        }
                                    }
                                    break;
                                }
                            }
                            if found {
                                i = cm_pos + 2;
                                continue;
                            }
                        }
                    }
                }
                i += 1;
            }
            pass29_count = count;
        }
        if pass29_count > 0 {
            eprintln!("[peephole] Pass 29: fused {} ListAppendLocal (list append)", pass29_count);
        }

        // Pass 30: Fuse LoadFast(dest) + SubscriptLocal(list, idx) + BinaryAdd + StoreFast(dest)
        //     → SubscriptLocal(list, idx) + AddLocalFromStack(dest)
        // This handles the pattern: sum = sum + list[i] (List Ops sum loop)
        // Also handles: LoadFast(dest) + Subscript + BinaryAdd + StoreFast(dest)
        let pass30_count;
        {
            let len = self.instructions.len();
            let mut count = 0;
            let mut i = 0;
            while i + 4 < len {
                if let (
                    Opcode::LoadFast(dest),
                    _,
                    Opcode::BinaryAdd,
                    Opcode::StoreFast(dest2),
                ) = (&self.instructions[i].opcode, &self.instructions[i+1].opcode, &self.instructions[i+2].opcode, &self.instructions[i+3].opcode)
                {
                    if dest == dest2 {
                        // Check if position i+1 is a subscript-like instruction
                        if matches!(&self.instructions[i+1].opcode, Opcode::SubscriptLocal(_, _) | Opcode::Subscript) {
                            let ld = *dest;
                            self.instructions[i].opcode = Opcode::Nop;     // Remove LoadFast(dest)
                            self.instructions[i+2].opcode = Opcode::Nop;   // Remove BinaryAdd
                            self.instructions[i+3].opcode = Opcode::AddLocalFromStack(ld);  // Replace StoreFast(dest)
                            count += 1;
                            i += 4;
                            continue;
                        }
                    }
                }
                i += 1;
            }
            pass30_count = count;
        }
        if pass30_count > 0 {
            eprintln!("[peephole] Pass 30: fused {} AddLocalSubscript (list sum)", pass30_count);
        }

        // Pass 31: IntInt comparison specialization — safe because comparisons only produce booleans
        let mut pass31_count = 0u32;
        {
            let len = self.instructions.len();
            let mut i = 0;
            while i + 2 < len {
                if let (Opcode::LoadFast(l1), Opcode::LoadFast(l2), ref op) = (
                    &self.instructions[i].opcode,
                    &self.instructions[i + 1].opcode,
                    &self.instructions[i + 2].opcode,
                ) {
                    if l1 != l2 {
                        let specialized = match op {
                            Opcode::CompareLt => Some(Opcode::CompareLtIntInt),
                            Opcode::CompareGt => Some(Opcode::CompareGtIntInt),
                            Opcode::CompareEq => Some(Opcode::CompareEqIntInt),
                            Opcode::CompareLtEq => Some(Opcode::CompareLeIntInt),
                            Opcode::CompareGtEq => Some(Opcode::CompareGeIntInt),
                            Opcode::CompareNotEq => Some(Opcode::CompareNotEqIntInt),
                            _ => None,
                        };
                        if let Some(spec_op) = specialized {
                            self.instructions[i].opcode = spec_op;
                            self.instructions[i + 1].opcode = Opcode::Nop;
                            self.instructions[i + 2].opcode = Opcode::Nop;
                            pass31_count += 1;
                            i += 3;
                            continue;
                        }
                    }
                }
                i += 1;
            }
        }
        if pass31_count > 0 {
            eprintln!("[peephole] Pass 31: specialized {} comparison ops to IntInt", pass31_count);
        }

        // Pass 32: Algebraic simplification — x - A + A → x (and x + A - A → x)
        let mut pass32_count = 0u32;
        {
            let len = self.instructions.len();
            let mut i = 0;
            while i + 3 < len {
                if let (Opcode::LoadConst(c1), ref op1, Opcode::LoadConst(c2), ref op2) = (
                    &self.instructions[i].opcode,
                    &self.instructions[i + 1].opcode,
                    &self.instructions[i + 2].opcode,
                    &self.instructions[i + 3].opcode,
                ) {
                    let same_const = match (&self.constants[*c1], &self.constants[*c2]) {
                        (Value::Integer(a), Value::Integer(b)) => a == b,
                        (Value::Float(a), Value::Float(b)) => a == b,
                        (Value::String(a), Value::String(b)) => a == b,
                        (Value::Boolean(a), Value::Boolean(b)) => a == b,
                        _ => false,
                    };
                    if same_const {
                        match (op1, op2) {
                            (Opcode::BinarySubtract, Opcode::BinaryAdd) |
                            (Opcode::BinaryAdd, Opcode::BinarySubtract) => {
                                self.instructions[i].opcode = Opcode::Nop;
                                self.instructions[i + 1].opcode = Opcode::Nop;
                                self.instructions[i + 2].opcode = Opcode::Nop;
                                self.instructions[i + 3].opcode = Opcode::Nop;
                                pass32_count += 1;
                                i += 4;
                                continue;
                            }
                            _ => {}
                        }
                    }
                }
                // Pattern: LoadConst(0) + BinaryAdd → NOP (identity)
                if let (Opcode::LoadConst(cidx), Opcode::BinaryAdd) = (
                    &self.instructions[i].opcode,
                    &self.instructions[i + 1].opcode,
                ) {
                    if matches!(&self.constants[*cidx], Value::Integer(0)) {
                        self.instructions[i].opcode = Opcode::Nop;
                        self.instructions[i + 1].opcode = Opcode::Nop;
                        pass32_count += 1;
                        i += 2;
                        continue;
                    }
                }
                // Pattern: LoadConst(1) + BinaryMultiply → NOP (identity)
                if let (Opcode::LoadConst(cidx), Opcode::BinaryMultiply) = (
                    &self.instructions[i].opcode,
                    &self.instructions[i + 1].opcode,
                ) {
                    if matches!(&self.constants[*cidx], Value::Integer(1)) {
                        self.instructions[i].opcode = Opcode::Nop;
                        self.instructions[i + 1].opcode = Opcode::Nop;
                        pass32_count += 1;
                        i += 2;
                        continue;
                    }
                }
                i += 1;
            }
        }
        if pass32_count > 0 {
            eprintln!("[peephole] Pass 32: algebraic simplifications: {}", pass32_count);
        }

        // Pass 33: Disabled — CallMethodVoid handler needs full dispatch chain

        // Pass 22: Compact — remove all Nop instructions and adjust jump targets
        let old_len = self.instructions.len();
        if old_len == 0 { return; }

        // Build mapping: old_ip → new_ip (skipping Nops)
        let mut old_to_new: Vec<usize> = vec![0; old_len];
        let mut new_ip = 0usize;
        for i in 0..old_len {
            if matches!(self.instructions[i].opcode, Opcode::Nop) {
                old_to_new[i] = usize::MAX; // deleted
            } else {
                old_to_new[i] = new_ip;
                new_ip += 1;
            }
        }

        // Collect non-Nop instructions and adjust jump targets
        let old_instructions: Vec<Instruction> = self.instructions.drain(..).collect();
        for (old_pos, instr) in old_instructions.iter().enumerate() {
            if matches!(instr.opcode, Opcode::Nop) { continue; }
            let mut new_instr = instr.clone();
            // Handle RELATIVE offset opcodes (ip = ip - 1 + offset)
            match &mut new_instr.opcode {
                Opcode::JumpForward(offset)
                | Opcode::PopJumpIfFalse(offset)
                | Opcode::PopJumpIfTrue(offset)
                | Opcode::ForIter(offset) => {
                    let old_abs_target = old_pos + *offset;
                    if old_abs_target < old_len {
                        let new_abs_target = old_to_new[old_abs_target];
                        let new_pos = old_to_new[old_pos];
                        if new_abs_target == usize::MAX {
                            let mut next = old_abs_target + 1;
                            while next < old_len && old_to_new[next] == usize::MAX {
                                next += 1;
                            }
                            if next < old_len {
                                *offset = old_to_new[next].saturating_sub(new_pos);
                            } else {
                                *offset = 0;
                            }
                        } else {
                            *offset = new_abs_target.saturating_sub(new_pos);
                        }
                    }
                }
                // JumpBackward uses ABSOLUTE target (VM: ip = c)
                Opcode::JumpBackward(target) => {
                    if *target < old_len {
                        let mapped = old_to_new[*target];
                        if mapped == usize::MAX {
                            // Target was Nopped — scan forward to next non-Nop
                            let mut next = *target + 1;
                            while next < old_len && old_to_new[next] == usize::MAX {
                                next += 1;
                            }
                            if next < old_len {
                                *target = old_to_new[next];
                            }
                        } else {
                            *target = mapped;
                        }
                    }
                }
                // Handle ABSOLUTE target opcodes (target = absolute IP)
                Opcode::SetupExcept(target)
                | Opcode::SetupFinally(target)
                | Opcode::SetupLoop(target) => {
                    if *target < old_len {
                        *target = old_to_new[*target];
                    }
                }
                Opcode::PopJumpIfLeLocalImm(_, _, target)
                | Opcode::PopJumpIfLtLocalImm(_, _, target) => {
                    if *target < old_len {
                        *target = old_to_new[*target];
                    }
                }
                Opcode::ModJumpIfNotZero(_, _, target) => {
                    if *target < old_len {
                        *target = old_to_new[*target];
                    }
                }
                Opcode::PopJumpIfSubscriptGt(_, _, _, target) => {
                    if *target < old_len {
                        *target = old_to_new[*target];
                    }
                }
                Opcode::PopJumpIfLtLocal(_, _, target) => {
                    if *target < old_len {
                        *target = old_to_new[*target];
                    }
                }
                Opcode::PopJumpIfNotSqrAddSqrGtImm(_, _, _, target) => {
                    if *target < old_len {
                        *target = old_to_new[*target];
                    }
                }
                // JumpWhileIncrementedLt(packed_bitfield):
                // bits 0-15: local_idx, bits 16-31: loop_start, bits 32-47: increment, bits 48-63: limit
                Opcode::JumpWhileIncrementedLt(packed_val) => {
                    let loop_start = (*packed_val >> 16) & 0xFFFF;
                    if loop_start < old_len {
                        let new_loop_start = old_to_new[loop_start];
                        *packed_val = (*packed_val & !(0xFFFF << 16)) | (new_loop_start << 16);
                    }
                }
                // ForRange(0) — target is in self.operand, not opcode field:
                // operand bits 48-63: loop_end
                Opcode::ForRange(_) => {
                    let old_operand = new_instr.operand;
                    let loop_end = (old_operand >> 48) & 0xFFFF;
                    if loop_end < old_len {
                        let mut mapped = old_to_new[loop_end];
                        if mapped == usize::MAX {
                            let mut next = loop_end + 1;
                            while next < old_len && old_to_new[next] == usize::MAX {
                                next += 1;
                            }
                            if next < old_len {
                                mapped = old_to_new[next];
                            } else {
                                mapped = 0;
                            }
                        }
                        new_instr.operand = (old_operand & !(0xFFFF << 48)) | (mapped << 48);
                    }
                }
                // MakeFunction(func_name_idx, params_count, func_info_idx)
                // func_info constant string starts with "body_start,..."
                Opcode::MakeFunction(_, _, func_info_idx) => {
                    if let Some(Value::String(info)) = self.constants.get_mut(*func_info_idx) {
                        let parts: Vec<&str> = info.split(',').collect();
                        if let Some(Ok(old_body_start)) = parts.first().map(|s| s.parse::<usize>()) {
                            if old_body_start < old_len {
                                let new_body_start = old_to_new[old_body_start];
                                let new_info = format!("{},{}", new_body_start, parts[1..].join(","));
                                *info = new_info;
                            }
                        }
                    }
                }
                _ => {}
            }
            self.instructions.push(new_instr);
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.emit(Opcode::PopTop, 0);
            }
            Stmt::Assign { target, value } => {
                match target {
                    Expr::Identifier(name) => {
                        self.compile_expr(value)?;
                        if self.global_names.contains(name) {
                            let name_idx = self.intern_name(name);
                            self.emit(Opcode::StoreName(name_idx), 0);
                        } else if self.nonlocal_names.contains(name) {
                            // nonlocal: store to enclosing scope
                            if let Some((_, inner_idx, _)) = self.find_enclosing_var(name) {
                                self.emit(Opcode::StoreFast(inner_idx), 0);
                            } else {
                                // Fallback: treat as global
                                let name_idx = self.intern_name(name);
                                self.emit(Opcode::StoreName(name_idx), 0);
                            }
                        } else {
                            // Check if this variable is captured by an enclosing scope
                            if let Some((_, inner_idx, _)) = self.find_enclosing_var(name) {
                                self.emit(Opcode::StoreFast(inner_idx), 0);
                            } else {
                                let idx = self.get_or_create_local(name);
                                self.emit(Opcode::StoreFast(idx), 0);
                            }
                        }
                    }
                    Expr::Attribute { object, name } => {
                        if let Expr::Identifier(obj_name) = &**object {
                            if let Some(sl) = self.self_local {
                                if let Some(&local_idx) = self.local_map.get(obj_name.as_str()) {
                                    if local_idx == sl {
                                        self.emit(Opcode::LoadFast(local_idx), 0);
                                        self.compile_expr(value)?;
                                        // In constructor: use StoreAttr so VM field detection can parse names
                                        // In methods: use compile-time offset via SetInstanceField
                                        if self.in_constructor {
                                            let attr_name_idx = self.intern_name(name);
                                            self.emit(Opcode::StoreAttr(attr_name_idx), 0);
                                        } else {
                                            let idx = if let Some(ref class_name) = self.current_class {
                                                if let Some(fields) = self.class_field_map.get(class_name) {
                                                    if let Some((_, offset)) = fields.iter().find(|(n, _)| n == name) {
                                                        self.add_constant(Value::Integer(*offset as i64))
                                                    } else {
                                                        self.add_constant(Value::String(name.clone()))
                                                    }
                                                } else {
                                                    self.add_constant(Value::String(name.clone()))
                                                }
                                            } else {
                                                self.add_constant(Value::String(name.clone()))
                                            };
                                            self.emit(Opcode::SetInstanceField(idx), 0);
                                        }
                                        return Ok(());
                                    }
                                }
                            }
                        }
                        self.compile_expr(object)?;
                        self.compile_expr(value)?;
                        let attr_name_idx = self.intern_name(name);
                        self.emit(Opcode::StoreAttr(attr_name_idx), 0);
                    }
                    Expr::Index { object, index } => {
                        self.compile_expr(object)?;
                        self.compile_expr(index)?;
                        self.compile_expr(value)?;
                        self.emit(Opcode::StoreSubscript, 0);
                    }
                    Expr::Tuple(items) => {
                        self.compile_expr(value)?;
                        let temp_idx = self.get_or_create_local("__temp_multi");
                        self.emit(Opcode::StoreFast(temp_idx), 0);
                        for (i, item) in items.iter().enumerate() {
                            if let Expr::Identifier(name) = item {
                                self.emit(Opcode::LoadFast(temp_idx), 0);
                                let idx = self.add_constant(Value::Integer(i as i64));
                                self.emit(Opcode::LoadConst(idx), 0);
                                self.emit(Opcode::Subscript, 0);
                                let tidx = self.get_or_create_local(name);
                                self.emit(Opcode::StoreFast(tidx), 0);
                            } else {
                                return Err(ArabiError::CompileError {
                                    message: "هدف اسناد غير صالح".to_string(),
                                    span: arabi_core::span::Span::single(arabi_core::span::Position::start()),
                                });
                            }
                        }
                        self.emit(Opcode::LoadFast(temp_idx), 0);
                        self.emit(Opcode::PopTop, 0);
                    }
                    _ => return Err(ArabiError::CompileError {
                        message: "هدف اسناد غير صالح".to_string(),
                        span: arabi_core::span::Span::single(arabi_core::span::Position::start()),
                    }),
                }
            }
            Stmt::MultiAssign { targets, value } => {
                self.compile_expr(value)?;

                let has_star = targets.iter().any(|(_, star)| *star);

                if !has_star {
                    let temp_idx = self.get_or_create_local("__temp_multi");
                    self.emit(Opcode::StoreFast(temp_idx), 0);
                    for (i, (target, _)) in targets.iter().enumerate() {
                        self.emit(Opcode::LoadFast(temp_idx), 0);
                        let idx = self.add_constant(Value::Integer(i as i64));
                        self.emit(Opcode::LoadConst(idx), 0);
                        self.emit(Opcode::Subscript, 0);
                        let tidx = self.get_or_create_local(target);
                        self.emit(Opcode::StoreFast(tidx), 0);
                    }
                    self.emit(Opcode::LoadFast(temp_idx), 0);
                    self.emit(Opcode::PopTop, 0);
                } else {
                    let star_pos = targets.iter().position(|(_, s)| *s).expect("Star target expected but not found in multi-assign");
                    let before_count = star_pos;
                    let after_count = targets.len() - 1 - star_pos;

                    // Store RHS value
                    let temp_idx = self.get_or_create_local("__star_temp");
                    self.emit(Opcode::StoreFast(temp_idx), 0);

                    // Get length: temp.طول
                    self.emit(Opcode::LoadFast(temp_idx), 0);
                    let attr_idx = self.add_constant(Value::String("طول".to_string()));
                    self.emit(Opcode::LoadConst(attr_idx), 0);
                    self.emit(Opcode::GetAttribute, 0);
                    let len_idx = self.get_or_create_local("__star_len");
                    self.emit(Opcode::StoreFast(len_idx), 0);

                    // Assign before-star targets: temp[i]
                    for (i, target) in targets.iter().enumerate().take(before_count) {
                        self.emit(Opcode::LoadFast(temp_idx), 0);
                        let idx = self.add_constant(Value::Integer(i as i64));
                        self.emit(Opcode::LoadConst(idx), 0);
                        self.emit(Opcode::Subscript, 0);
                        let tidx = self.get_or_create_local(&target.0);
                        self.emit(Opcode::StoreFast(tidx), 0);
                    }

                    // Star target: temp[before_count : len-after_count]
                    self.emit(Opcode::LoadFast(temp_idx), 0);
                    let s = self.add_constant(Value::Integer(before_count as i64));
                    self.emit(Opcode::LoadConst(s), 0);
                    if after_count > 0 {
                        self.emit(Opcode::LoadFast(len_idx), 0);
                        let ac = self.add_constant(Value::Integer(after_count as i64));
                        self.emit(Opcode::LoadConst(ac), 0);
                        self.emit(Opcode::BinarySubtract, 0);
                    } else {
                        self.emit(Opcode::LoadNone, 0);
                    }
                    self.emit(Opcode::LoadNone, 0); // step
                    self.emit(Opcode::BuildSlice, 0);
                    self.emit(Opcode::Subscript, 0);
                    let tidx = self.get_or_create_local(&targets[star_pos].0);
                    self.emit(Opcode::StoreFast(tidx), 0);

                    // Assign after-star targets: temp[len - after_count + i]
                    for i in 0..after_count {
                        self.emit(Opcode::LoadFast(temp_idx), 0);
                        self.emit(Opcode::LoadFast(len_idx), 0);
                        let ac = self.add_constant(Value::Integer((after_count - i) as i64));
                        self.emit(Opcode::LoadConst(ac), 0);
                        self.emit(Opcode::BinarySubtract, 0);
                        self.emit(Opcode::Subscript, 0);
                        let tidx = self.get_or_create_local(&targets[star_pos + 1 + i].0);
                        self.emit(Opcode::StoreFast(tidx), 0);
                    }

                    // Pop temp
                    self.emit(Opcode::LoadFast(temp_idx), 0);
                    self.emit(Opcode::PopTop, 0);
                }
            }
            Stmt::AugAssign { target, op, value } => {
                match target {
                    Expr::Identifier(name) => {
                        // Check if this variable is captured by an enclosing scope first
                        if let Some((_, inner_idx, _)) = self.find_enclosing_var(name) {
                            if *op == AugOp::Add {
                                if let Expr::Integer(n) = value {
                                    self.emit_with_operand(Opcode::IncrementInt(inner_idx), *n as usize, 0);
                                    return Ok(());
                                }
                            }
                            self.emit(Opcode::LoadFast(inner_idx), 0);
                            self.compile_expr(value)?;
                            self.compile_aug_op(op)?;
                            self.emit(Opcode::StoreFast(inner_idx), 0);
                        } else                         if let Some(&idx) = self.local_map.get(name) {
                            if *op == AugOp::Add {
                                if let Expr::Integer(n) = value {
                                    self.emit_with_operand(Opcode::IncrementInt(idx), *n as usize, 0);
                                    return Ok(());
                                }
                                if let Expr::String(s) = value {
                                    let const_idx = self.add_constant(Value::String(s.clone()));
                                    self.emit_with_operand(Opcode::InplaceAddStrConst(idx), const_idx, 0);
                                    return Ok(());
                                }
                            }
                            self.emit(Opcode::LoadFast(idx), 0);
                            self.compile_expr(value)?;
                            self.compile_aug_op(op)?;
                            self.emit(Opcode::StoreFast(idx), 0);
                        } else if self.nonlocal_names.contains(name) {
                            let name_idx = self.intern_name(name);
                            self.emit(Opcode::LoadName(name_idx), 0);
                            self.compile_expr(value)?;
                            self.compile_aug_op(op)?;
                            let name_idx2 = self.intern_name(name);
                            self.emit(Opcode::StoreName(name_idx2), 0);
                        } else {
                            let name_idx = self.intern_name(name);
                            self.emit(Opcode::LoadName(name_idx), 0);
                            self.compile_expr(value)?;
                            self.compile_aug_op(op)?;
                            let name_idx2 = self.intern_name(name);
                            self.emit(Opcode::StoreName(name_idx2), 0);
                        }
                    }
                    _ => return Err(ArabiError::CompileError {
                        message: "هدف اسناد مركب غير صالح".to_string(),
                        span: arabi_core::span::Span::single(arabi_core::span::Position::start()),
                    }),
                }
            }
            Stmt::Return(value) => {
                if let Some(expr) = value {
                    // TCO: detect tail call — return f(args) where f is current function
                    if let Expr::Call { function, args, kwargs, unpack_args, unpack_kwargs } = &*expr {
                        if kwargs.is_empty() && unpack_args.is_empty() && unpack_kwargs.is_empty() {
                            if let Expr::Identifier(name) = &**function {
                                if self.current_function_name.as_deref() == Some(name.as_str()) {
                                    // Push func onto stack first (same layout as normal call)
                                    self.compile_expr(function)?;
                                    for arg in args {
                                        self.compile_expr(arg)?;
                                    }
                                    self.emit(Opcode::TailCall(args.len()), 0);
                                    return Ok(());
                                }
                            }
                        }
                    }
                    self.compile_expr(expr)?;
                } else {
                    self.emit(Opcode::LoadNone, 0);
                }
                self.emit(Opcode::ReturnValue, 0);
            }
            Stmt::Decorator { decorators, definition } => {
                self.compile_stmt(definition)?;
                if let Stmt::FunctionDef { name, .. } = &**definition {
                    for decorator in decorators.iter().rev() {
                        self.compile_expr(decorator)?;
                        let name_idx = self.intern_name(name);
                        self.emit(Opcode::LoadName(name_idx), 0);
                        self.emit(Opcode::CallFunction(1), 0);
                        let idx = self.get_or_create_local(name);
                        self.emit(Opcode::StoreFast(idx), 0);
                    }
                } else if let Stmt::ClassDef { name, .. } = &**definition {
                    for decorator in decorators.iter().rev() {
                        self.compile_expr(decorator)?;
                        let name_idx = self.intern_name(name);
                        self.emit(Opcode::LoadName(name_idx), 0);
                        self.emit(Opcode::CallFunction(1), 0);
                        let idx = self.get_or_create_local(name);
                        self.emit(Opcode::StoreFast(idx), 0);
                    }
                }
            }
            Stmt::FunctionDef { name, params, body } => {
                self.compile_function_def(name, params, body)?;
            }
            Stmt::If { condition, body, elifs, else_body } => {
                self.compile_if(condition, body, elifs, else_body)?;
            }
            Stmt::While { condition, body, else_body } => {
                self.compile_while(condition, body, else_body.as_ref())?;
            }
            Stmt::For { target, iterable, body, else_body } => {
                self.compile_for(target, iterable, body, else_body.as_ref())?;
            }
            Stmt::Break => {
                if let Some(ctx) = self.loop_stack.last() {
                    let _ = ctx;
                    self.emit(Opcode::JumpForward(0), 0);
                    let jmp_idx = self.instructions.len() - 1;
                    if let Some(ctx_mut) = self.loop_stack.last_mut() {
                        ctx_mut.break_jumps.push(jmp_idx);
                    }
                }
            }
            Stmt::Continue => {
                if !self.loop_stack.is_empty() {
                    self.emit(Opcode::JumpBackward(0), 0);
                    let jmp_idx = self.instructions.len() - 1;
                    if let Some(ctx_mut) = self.loop_stack.last_mut() {
                        ctx_mut.continue_jumps.push(jmp_idx);
                    }
                }
            }
            Stmt::ClassDef { name, bases, body } => {
                self.compile_class_def(name, bases, body)?;
            }
            Stmt::Import { module, alias } => {
                let target_name = alias.as_ref().unwrap_or(module).clone();
                let idx = self.add_constant(Value::String(module.clone()));
                self.emit(Opcode::LoadConst(idx), 0);
                let mod_idx = self.intern_name(module);
                self.emit_with_operand(Opcode::ImportModule(mod_idx), 0, 0);
                let tidx = self.get_or_create_local(&target_name);
                self.emit(Opcode::StoreFast(tidx), 0);
            }
            Stmt::ImportFrom { module, names } => {
                let idx = self.add_constant(Value::String(module.clone()));
                self.emit(Opcode::LoadConst(idx), 0);
                let mod_idx = self.intern_name(module);
                self.emit_with_operand(Opcode::ImportModule(mod_idx), 0, 0);
                for (name, alias) in names.iter() {
                    let target = alias.as_ref().unwrap_or(name).clone();
                    self.emit(Opcode::DupTop, 0);
                    let nidx = self.add_constant(Value::String(name.clone()));
                    self.emit(Opcode::LoadConst(nidx), 0);
                    self.emit(Opcode::GetAttribute, 0);
                    let tidx = self.get_or_create_local(&target);
                    self.emit(Opcode::StoreFast(tidx), 0);
                }
                self.emit(Opcode::PopTop, 0);
            }
            Stmt::Try { body, excepts, else_body, finally_body } => {
                let exc_local = self.get_or_create_local("ح_استثناء");
                self.emit(Opcode::SetupExcept(0), 0);
                let setup_idx = self.instructions.len() - 1;
                for stmt in &body.stmts {
                    self.compile_stmt(stmt)?;
                }
                self.emit(Opcode::JumpForward(0), 0);
                let jump_success_idx = self.instructions.len() - 1;
                let handler_ip = self.instructions.len();
                if let Opcode::SetupExcept(ref mut target) = self.instructions[setup_idx].opcode {
                    *target = handler_ip;
                }
                let mut except_jump_indices = Vec::new();
                for exc in excepts.iter() {
                    self.emit(Opcode::StoreFast(exc_local), 0);
                    let mut type_check_jump = None;
                    if let Some(ref type_name) = exc.type_name {
                        self.emit(Opcode::LoadFast(exc_local), 0);
                        let const_idx = self.add_constant(Value::String(type_name.clone()));
                        self.emit(Opcode::CheckExceptionType(const_idx), 0);
                        self.emit(Opcode::PopJumpIfFalse(0), 0);
                        type_check_jump = Some(self.instructions.len() - 1);
                    }
                    if let Some(ref name) = exc.name {
                        let idx = self.get_or_create_local(name);
                        self.emit(Opcode::LoadFast(exc_local), 0);
                        self.emit(Opcode::StoreFast(idx), 0);
                    }
                    for stmt in &exc.body.stmts {
                        self.compile_stmt(stmt)?;
                    }
                    if type_check_jump.is_some() {
                        self.emit(Opcode::PopTop, 0);
                    }
                    self.emit(Opcode::JumpForward(0), 0);
                    except_jump_indices.push(self.instructions.len() - 1);
                    if let Some(jump_idx) = type_check_jump {
                        let target_ip = self.instructions.len();
                        if let Opcode::PopJumpIfFalse(ref mut target) = self.instructions[jump_idx].opcode {
                            *target = target_ip - jump_idx;
                        }
                    }
                }
                let else_or_finally_ip = self.instructions.len();
                if let Opcode::JumpForward(ref mut target) = self.instructions[jump_success_idx].opcode {
                    *target = else_or_finally_ip - jump_success_idx;
                }
                if let Some(ref else_b) = else_body {
                    for stmt in &else_b.stmts {
                        self.compile_stmt(stmt)?;
                    }
                }
                let finally_ip = self.instructions.len();
                for jump_idx in except_jump_indices {
                    if let Opcode::JumpForward(ref mut target) = self.instructions[jump_idx].opcode {
                        *target = finally_ip - jump_idx;
                    }
                }
                if let Some(ref finally_b) = finally_body {
                    for stmt in &finally_b.stmts {
                        self.compile_stmt(stmt)?;
                    }
                }
                self.emit(Opcode::EndExcept, 0);
            }
            Stmt::Raise(value) => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(Opcode::LoadNone, 0);
                }
                self.emit(Opcode::Raise, 0);
            }
            Stmt::Yield(value) => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(Opcode::LoadNone, 0);
                }
                self.emit(Opcode::YieldValue, 0);
            }
            Stmt::YieldFrom(expr) => {
                // Desugar yield_from(x) into:
                //   __yfi = <expr>
                //   __yfi_for_iter = __yfi (for iteration)
                //   while __yfi_idx < len:
                //       __yfv = __yfi[__yfi_idx]
                //       yield __yfv
                //       __yfi_idx += 1
                self.loop_depth += 1;
                let depth = self.loop_depth;
                let iter_idx = self.get_or_create_local(&format!("__yfi_{}", depth));
                let idx_idx = self.get_or_create_local(&format!("__yfi_idx_{}", depth));
                let len_idx = self.get_or_create_local(&format!("__yfi_len_{}", depth));
                let val_idx = self.get_or_create_local(&format!("__yfi_val_{}", depth));

                // Compile the iterable and materialize via ForIter
                self.compile_expr(expr)?;
                self.emit(Opcode::ForIter(0), 0);

                // Store the three values: len, idx=0, list
                self.emit(Opcode::StoreFast(len_idx), 0);
                self.emit(Opcode::StoreFast(idx_idx), 0);
                self.emit(Opcode::StoreFast(iter_idx), 0);

                // Loop start (condition check)
                let loop_start = self.instructions.len();

                // Check if idx < len
                self.emit(Opcode::LoadFast(idx_idx), 0);
                self.emit(Opcode::LoadFast(len_idx), 0);
                self.emit(Opcode::CompareLt, 0);

                // Jump to end if false
                let jump_to_end = self.instructions.len();
                self.emit(Opcode::PopJumpIfFalse(0), 0);

                // Load current item: __yfi[__yfi_idx]
                self.emit(Opcode::LoadFast(iter_idx), 0);
                self.emit(Opcode::LoadFast(idx_idx), 0);
                self.emit(Opcode::Subscript, 0);
                self.emit(Opcode::StoreFast(val_idx), 0);

                // yield __yfv
                self.emit(Opcode::LoadFast(val_idx), 0);
                self.emit(Opcode::YieldValue, 0);

                // Increment index
                self.emit_with_operand(Opcode::IncrementInt(idx_idx), 1, 0);

                // Jump back to loop start
                self.emit(Opcode::JumpBackward(loop_start), 0);

                // Patch jump_to_end
                let loop_end = self.instructions.len();
                if let Some(offset) = self.instructions[jump_to_end].opcode.as_jump_offset_mut() {
                    *offset = loop_end - jump_to_end;
                }

                self.loop_depth -= 1;
            }
            Stmt::With { context, target, body } => {
                self.compile_expr(context)?;
                let ctx_idx = self.get_or_create_local("__سياق");
                self.emit(Opcode::StoreFast(ctx_idx), 0);
                self.emit(Opcode::LoadFast(ctx_idx), 0);
                let enter_idx = self.intern_name("__ادخل__");
                self.emit(Opcode::CallMethod(enter_idx, 0), 0);
                if let Some(name) = target {
                    let idx = self.get_or_create_local(name);
                    self.emit(Opcode::StoreFast(idx), 0);
                } else {
                    self.emit(Opcode::PopTop, 0);
                }
                self.emit(Opcode::SetupFinally(0), 0);
                let setup_idx = self.instructions.len() - 1;
                for stmt in &body.stmts {
                    self.compile_stmt(stmt)?;
                }
                self.emit(Opcode::LoadFast(ctx_idx), 0);
                let exit_idx = self.intern_name("__اترك__");
                self.emit(Opcode::CallMethod(exit_idx, 0), 0);
                self.emit(Opcode::PopTop, 0);
                self.emit(Opcode::JumpForward(0), 0);
                let jump_end_idx = self.instructions.len() - 1;
                let handler_ip = self.instructions.len();
                if let Opcode::SetupFinally(ref mut target) = self.instructions[setup_idx].opcode {
                    *target = handler_ip;
                }
                self.emit(Opcode::PopTop, 0);
                self.emit(Opcode::LoadFast(ctx_idx), 0);
                let exit_idx2 = self.intern_name("__اترك__");
                self.emit(Opcode::CallMethod(exit_idx2, 0), 0);
                self.emit(Opcode::PopTop, 0);
                let end_ip = self.instructions.len();
                if let Opcode::JumpForward(ref mut target) = self.instructions[jump_end_idx].opcode {
                    *target = end_ip - jump_end_idx;
                }
            }
            Stmt::Match { value, cases, default } => {
                // Compile match/case as if/elif/else chain
                let match_value_idx = self.get_or_create_local("__طابق_قيمة");
                self.compile_expr(value)?;
                self.emit(Opcode::StoreFast(match_value_idx), 0);

                let mut jump_next_idxs = Vec::new();
                let mut case_start_idxs = Vec::new();
                let mut jump_end_idxs = Vec::new();

                for (_i, (pattern, guard, body)) in cases.iter().enumerate() {
                    // Record start of this case
                    case_start_idxs.push(self.instructions.len());

                    // Load match value
                    self.emit(Opcode::LoadFast(match_value_idx), 0);
                    // Load pattern
                    self.compile_expr(pattern)?;
                    // Compare
                    self.emit(Opcode::CompareEq, 0);
                    // Jump to next case if false
                    self.emit(Opcode::PopJumpIfFalse(0), 0);
                    jump_next_idxs.push(self.instructions.len() - 1);

                    // If there's a guard, compile and check it
                    if let Some(guard_expr) = guard {
                        self.emit(Opcode::LoadFast(match_value_idx), 0);
                        self.compile_expr(guard_expr)?;
                        self.emit(Opcode::PopJumpIfFalse(0), 0);
                        jump_next_idxs.push(self.instructions.len() - 1);
                    }

                    // Compile body
                    for stmt in &body.stmts {
                        self.compile_stmt(stmt)?;
                    }

                    // Jump to end
                    self.emit(Opcode::JumpForward(0), 0);
                    jump_end_idxs.push(self.instructions.len() - 1);
                }

                // Patch jump-next offsets
                // Each case may produce 1 or 2 jumps (pattern + optional guard)
                // All jumps from case i should target case_start_idxs[i+1]
                let default_or_end = self.instructions.len();
                let num_cases = cases.len();
                let mut jump_idx = 0;
                for case_i in 0..num_cases {
                    let target = if case_i + 1 < num_cases {
                        case_start_idxs[case_i + 1]
                    } else {
                        default_or_end
                    };
                    let has_guard = cases[case_i].1.is_some();
                    let num_jumps = if has_guard { 2 } else { 1 };
                    for _ in 0..num_jumps {
                        if let Some(&idx) = jump_next_idxs.get(jump_idx) {
                            if let Opcode::PopJumpIfFalse(ref mut offset) = self.instructions[idx].opcode {
                                *offset = target - idx;
                            }
                        }
                        jump_idx += 1;
                    }
                }

                // Default case
                if let Some(default_body) = default {
                    for stmt in &default_body.stmts {
                        self.compile_stmt(stmt)?;
                    }
                }

                // Patch all jump-to-end
                let end_ip = self.instructions.len();
                for idx in jump_end_idxs {
                    if let Opcode::JumpForward(ref mut offset) = self.instructions[idx].opcode {
                        *offset = end_ip - idx;
                    }
                }
            }
            Stmt::Delete(expr) => {
                if let Expr::Identifier(name) = expr {
                    let idx = self.get_or_create_local(name);
                    self.emit(Opcode::LoadNone, 0);
                    self.emit(Opcode::StoreFast(idx), 0);
                }
            }
            Stmt::Assert { condition, message } => {
                self.compile_expr(condition)?;
                let false_idx = self.add_constant(Value::Boolean(false));
                self.emit(Opcode::LoadConst(false_idx), 0);
                self.emit(Opcode::CompareEq, 0);
                let jump_to_err = self.instructions.len();
                self.emit(Opcode::PopJumpIfTrue(0), 0);
                let jump_to_end = self.instructions.len();
                self.emit(Opcode::JumpForward(0), 0);
                let err_start = self.instructions.len();
                if let Some(msg) = message {
                    self.compile_expr(msg)?;
                } else {
                    let err_idx = self.add_constant(Value::String("assertion failed".to_string()));
                    self.emit(Opcode::LoadConst(err_idx), 0);
                }
                self.emit(Opcode::Raise, 0);
                let end = self.instructions.len();
                if let Opcode::PopJumpIfTrue(ref mut target) = self.instructions[jump_to_err].opcode {
                    *target = err_start - jump_to_err;
                }
                if let Opcode::JumpForward(ref mut target) = self.instructions[jump_to_end].opcode {
                    *target = end - jump_to_end;
                }
            }
            Stmt::Global(names) => {
                for name in names {
                    self.global_names.insert(name.clone());
                    self.local_map.remove(name);
                }
            }
            Stmt::Nonlocal(names) => {
                for name in names {
                    self.nonlocal_names.insert(name.clone());
                    self.local_map.remove(name);
                }
            }
            Stmt::Pass => {}
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Integer(n) => {
                let idx = self.add_constant(Value::Integer(*n));
                self.emit(Opcode::LoadConst(idx), 0);
            }
            Expr::Float(f) => {
                let idx = self.add_constant(Value::Float(*f));
                self.emit(Opcode::LoadConst(idx), 0);
            }
            Expr::String(s) => {
                let idx = self.add_constant(Value::String(s.clone()));
                self.emit(Opcode::LoadConst(idx), 0);
            }
            Expr::Boolean(b) => {
                if *b {
                    self.emit(Opcode::LoadTrue, 0);
                } else {
                    self.emit(Opcode::LoadFalse, 0);
                }
            }
            Expr::Null => {
                self.emit(Opcode::LoadNone, 0);
            }
            Expr::Super => {
                self.emit(Opcode::LoadSuper, 0);
            }
            Expr::Identifier(name) => {
                if let Some(&idx) = self.local_map.get(name) {
                    self.emit(Opcode::LoadFast(idx), 0);
                } else if self.global_names.contains(name) {
                    let name_idx = self.intern_name(name);
                    self.emit(Opcode::LoadGlobal(name_idx), 0);
                } else {
                    if let Some((_, inner_idx, _)) = self.find_enclosing_var(name) {
                        self.emit(Opcode::LoadFast(inner_idx), 0);
                    } else {
                        let name_idx = self.intern_name(name);
                        self.emit(Opcode::LoadName(name_idx), 0);
                    }
                }
            }
            Expr::BinaryOp { left, op, right } => {
                match op {
                    BinOp::And => {
                        self.compile_expr(left)?;
                        self.emit(Opcode::DupTop, 0);
                        let jump_to_false = self.instructions.len();
                        self.emit(Opcode::PopJumpIfFalse(0), 0);
                        self.emit(Opcode::PopTop, 0);
                        self.compile_expr(right)?;
                        let jump_to_end = self.instructions.len();
                        self.emit(Opcode::JumpForward(0), 0);
                        let end_label = self.instructions.len();
                        if let Opcode::PopJumpIfFalse(ref mut target) = self.instructions[jump_to_false].opcode {
                            *target = end_label - jump_to_false;
                        }
                        if let Opcode::JumpForward(ref mut target) = self.instructions[jump_to_end].opcode {
                            *target = end_label - jump_to_end;
                        }
                    }
                    BinOp::Or => {
                        self.compile_expr(left)?;
                        self.emit(Opcode::DupTop, 0);
                        let jump_to_true = self.instructions.len();
                        self.emit(Opcode::PopJumpIfTrue(0), 0);
                        self.emit(Opcode::PopTop, 0);
                        self.compile_expr(right)?;
                        let jump_to_end = self.instructions.len();
                        self.emit(Opcode::JumpForward(0), 0);
                        let end_label = self.instructions.len();
                        if let Opcode::PopJumpIfTrue(ref mut target) = self.instructions[jump_to_true].opcode {
                            *target = end_label - jump_to_true;
                        }
                        if let Opcode::JumpForward(ref mut target) = self.instructions[jump_to_end].opcode {
                            *target = end_label - jump_to_end;
                        }
                    }
                    _ => {
                        if let Some(folded) = self.try_fold_constants(left, op, right) {
                            let idx = self.add_constant(folded);
                            self.emit(Opcode::LoadConst(idx), 0);
                        } else {
                            self.compile_expr(left)?;
                            self.compile_expr(right)?;
                            self.compile_bin_op(op)?;
                        }
                    }
                }
            }
            Expr::UnaryOp { op, operand } => {
                if let Some(folded) = self.try_fold_unary(op, operand) {
                    let idx = self.add_constant(folded);
                    self.emit(Opcode::LoadConst(idx), 0);
                } else {
                    self.compile_expr(operand)?;
                    match op {
                        UnaryOp::Neg => self.emit(Opcode::UnaryNegative, 0),
                        UnaryOp::Not => self.emit(Opcode::UnaryNot, 0),
                        UnaryOp::BitNot => self.emit(Opcode::UnaryBitNot, 0),
                    }
                }
            }
            Expr::Call { function, args, kwargs, unpack_args, unpack_kwargs } => {
                if let Expr::Attribute { object, name } = &**function {
                    self.compile_expr(object)?;
                    for arg in args {
                        self.compile_expr(arg)?;
                    }
                    let method_idx = self.intern_name(name);
                    self.emit(Opcode::CallMethod(method_idx, args.len()), 0);
                } else {
                    let has_unpack = !unpack_args.is_empty() || !unpack_kwargs.is_empty();
                    if has_unpack {
                        self.compile_expr(function)?;
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        for ua in unpack_args {
                            self.compile_expr(ua)?;
                        }
                        for uk in unpack_kwargs {
                            self.compile_expr(uk)?;
                        }
                        // For regular kwargs, build a dict on the stack
                        if !kwargs.is_empty() {
                            for (kw_name, val) in kwargs {
                                let idx = self.add_constant(Value::String(kw_name.clone()));
                                self.emit(Opcode::LoadConst(idx), 0);
                                self.compile_expr(val)?;
                            }
                            self.emit(Opcode::BuildDict(kwargs.len()), 0);
                        }
                        let total_kw_dicts = unpack_kwargs.len() + if kwargs.is_empty() { 0 } else { 1 };
                        self.emit(Opcode::CallFunctionUnpacked(args.len(), unpack_args.len(), total_kw_dicts), 0);
                    } else {
                        self.compile_expr(function)?;
                        for arg in args {
                            self.compile_expr(arg)?;
                        }
                        if kwargs.is_empty() {
                            self.emit(Opcode::CallFunction(args.len()), 0);
                        } else {
                            for (kw_name, val) in kwargs {
                                let idx = self.add_constant(Value::String(kw_name.clone()));
                                self.emit(Opcode::LoadConst(idx), 0);
                                self.compile_expr(val)?;
                            }
                            let kw_count = kwargs.len();
                            let idx = self.add_constant(Value::Integer(kw_count as i64));
                            self.emit(Opcode::LoadConst(idx), 0);
                            self.emit(Opcode::CallFunctionKw(args.len()), 0);
                        }
                    }
                }
            }
            Expr::List(items) => {
                for item in items {
                    self.compile_expr(item)?;
                }
                self.emit(Opcode::BuildList(items.len()), 0);
            }
            Expr::FString(s) => {
                let parts = self.parse_fstring_parts(s);
                if parts.len() == 1 {
                    match &parts[0] {
                        FPart::Literal(lit) => {
                            let idx = self.add_constant(Value::String(lit.clone()));
                            self.emit(Opcode::LoadConst(idx), 0);
                        }
                        FPart::Expr(expr_str, fmt_spec) => {
                            if let Some(fmt) = fmt_spec {
                                let fmt_idx = self.add_constant(Value::String(fmt.clone()));
                                self.emit(Opcode::LoadConst(fmt_idx), 0);
                                let mut inner_lexer = arabi_lexer::Lexer::new(expr_str);
                                if let Ok(tokens) = inner_lexer.tokenize() {
                                    let mut inner_parser = arabi_parser::Parser::new(tokens);
                                    if let Ok(program) = inner_parser.parse() {
                                        if let Some(arabi_parser::ast::Stmt::Expr(expr)) = program.stmts.first() {
                                            self.compile_expr(expr)?;
                                            self.emit(Opcode::StringFormat, 0);
                                        }
                                    }
                                }
                            } else {
                                let mut inner_lexer = arabi_lexer::Lexer::new(expr_str);
                                if let Ok(tokens) = inner_lexer.tokenize() {
                                    let mut inner_parser = arabi_parser::Parser::new(tokens);
                                    if let Ok(program) = inner_parser.parse() {
                                        if let Some(arabi_parser::ast::Stmt::Expr(expr)) = program.stmts.first() {
                                            self.compile_expr(expr)?;
                                            self.emit(Opcode::StringCoerce, 0);
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    let empty = self.add_constant(Value::String(String::new()));
                    self.emit(Opcode::LoadConst(empty), 0);
                    for part in &parts {
                        match part {
                            FPart::Literal(lit) => {
                                if !lit.is_empty() {
                                    let idx = self.add_constant(Value::String(lit.clone()));
                                    self.emit(Opcode::LoadConst(idx), 0);
                                    self.emit(Opcode::BinaryAdd, 0);
                                }
                            }
                            FPart::Expr(expr_str, fmt_spec) => {
                                if let Some(fmt) = fmt_spec {
                                    let fmt_idx = self.add_constant(Value::String(fmt.clone()));
                                    self.emit(Opcode::LoadConst(fmt_idx), 0);
                                    let mut inner_lexer = arabi_lexer::Lexer::new(expr_str);
                                    if let Ok(tokens) = inner_lexer.tokenize() {
                                        let mut inner_parser = arabi_parser::Parser::new(tokens);
                                        if let Ok(program) = inner_parser.parse() {
                                            if let Some(arabi_parser::ast::Stmt::Expr(expr)) = program.stmts.first() {
                                                self.compile_expr(expr)?;
                                                self.emit(Opcode::StringFormat, 0);
                                                self.emit(Opcode::BinaryAdd, 0);
                                            }
                                        }
                                    }
                                } else {
                                    let mut inner_lexer = arabi_lexer::Lexer::new(expr_str);
                                    if let Ok(tokens) = inner_lexer.tokenize() {
                                        let mut inner_parser = arabi_parser::Parser::new(tokens);
                                        if let Ok(program) = inner_parser.parse() {
                                            if let Some(arabi_parser::ast::Stmt::Expr(expr)) = program.stmts.first() {
                                                self.compile_expr(expr)?;
                                                self.emit(Opcode::StringCoerce, 0);
                                                self.emit(Opcode::BinaryAdd, 0);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Expr::Dict(pairs) => {
                for (key, val) in pairs {
                    self.compile_expr(key)?;
                    self.compile_expr(val)?;
                }
                self.emit(Opcode::BuildDict(pairs.len()), 0);
            }
            Expr::Set(items) => {
                for item in items {
                    self.compile_expr(item)?;
                }
                self.emit(Opcode::BuildSet(items.len()), 0);
            }
            Expr::Tuple(items) => {
                for item in items {
                    self.compile_expr(item)?;
                }
                self.emit(Opcode::BuildTuple(items.len()), 0);
            }
            Expr::Index { object, index } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit(Opcode::Subscript, 0);
            }
            Expr::Slice { object, start, end, step } => {
                self.compile_expr(object)?;
                if let Some(s) = start {
                    self.compile_expr(s)?;
                } else {
                    self.emit(Opcode::LoadNone, 0);
                }
                if let Some(e) = end {
                    self.compile_expr(e)?;
                } else {
                    self.emit(Opcode::LoadNone, 0);
                }
                if let Some(st) = step {
                    self.compile_expr(st)?;
                } else {
                    self.emit(Opcode::LoadNone, 0);
                }
                self.emit(Opcode::BuildSlice, 0);
                self.emit(Opcode::Subscript, 0);
            }
            Expr::Attribute { object, name } => {
                if let Expr::Identifier(obj_name) = &**object {
                    if let Some(sl) = self.self_local {
                        if let Some(&local_idx) = self.local_map.get(obj_name.as_str()) {
                            if local_idx == sl {
                                self.emit(Opcode::LoadFast(local_idx), 0);
                                // Try compile-time field offset resolution
                                let idx = if let Some(ref class_name) = self.current_class {
                                    if let Some(fields) = self.class_field_map.get(class_name) {
                                        if let Some((_, offset)) = fields.iter().find(|(n, _)| n == name) {
                                            self.add_constant(Value::Integer(*offset as i64))
                                        } else {
                                            self.add_constant(Value::String(name.clone()))
                                        }
                                    } else {
                                        self.add_constant(Value::String(name.clone()))
                                    }
                                } else {
                                    self.add_constant(Value::String(name.clone()))
                                };
                                self.emit(Opcode::GetInstanceField(idx), 0);
                                return Ok(());
                            }
                        }
                    }
                }
                self.compile_expr(object)?;
                let idx = self.add_constant(Value::String(name.clone()));
                self.emit(Opcode::LoadConst(idx), 0);
                self.emit(Opcode::GetAttribute, 0);
            }
            Expr::Lambda { params, body } => {
                let saved_map = self.local_map.clone();
                let saved_names = self.local_names.clone();
                let saved_num_locals = self.num_locals;
                let saved_nonlocal = self.nonlocal_names.clone();
                let saved_free_vars = self.free_vars.clone();

                // Push current scope onto enclosing stack
                self.enclosing_local_maps.push(saved_map.clone());
                self.nonlocal_names.clear();
                self.free_vars.clear();
                self.pending_captures_stack.push(Vec::new());

                // Reset local scope for the lambda — only params are local
                self.local_map = std::collections::HashMap::new();
                self.local_names = Vec::new();
                self.num_locals = 0;

                // Pre-create param locals
                let param_indices: Vec<usize> = params.iter().map(|p| self.get_or_create_local(p)).collect();

                // Jump over body
                self.emit(Opcode::JumpForward(0), 0);
                let skip_jump_idx = self.instructions.len() - 1;

                // Body start
                let body_start = self.instructions.len();

                // Compile body
                self.compile_expr(body)?;
                self.emit(Opcode::ReturnValue, 0);

                // Patch jump
                let body_end = self.instructions.len();
                if let Some(offset) = self.instructions[skip_jump_idx].opcode.as_jump_offset_mut() {
                    *offset = body_end - skip_jump_idx;
                }

                // Collect free variables
                let captured_free_vars = self.free_vars.clone();

                // Pop enclosing scope and pending captures
                self.enclosing_local_maps.pop();
                self.pending_captures_stack.pop();
                self.nonlocal_names = saved_nonlocal;
                let param_names_str = params.join(",");
                let param_names_idx = self.add_constant(Value::String(param_names_str));
                let param_indices_str = param_indices.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
                let param_indices_idx = self.add_constant(Value::String(param_indices_str));
                let defaults_idx = self.add_constant(Value::String("".to_string()));
                let empty_varargs_idx = self.add_constant(Value::String("".to_string()));
                let empty_kwargs_idx = self.add_constant(Value::String("".to_string()));
                let free_vars_str: Vec<String> = captured_free_vars.iter()
                    .map(|(_, outer_idx, inner_idx)| format!("{}:{}", inner_idx, outer_idx))
                    .collect();
                let free_vars_idx = self.add_constant(Value::String(free_vars_str.join("|")));
                let func_info_idx = self.add_constant(Value::String(format!("{},{},{},{},{},{},{},{},{}", body_start, param_names_idx, param_indices_idx, defaults_idx, empty_varargs_idx, empty_kwargs_idx, "0", free_vars_idx, self.num_locals)));
                let lambda_name_idx = self.intern_name("<lambda>");
                self.emit_with_operand(Opcode::MakeFunction(lambda_name_idx, params.len(), func_info_idx), body_start, 0);
                self.local_map = saved_map;
                self.local_names = saved_names;
                self.num_locals = saved_num_locals;
                self.free_vars = saved_free_vars;
            }
            Expr::IfExpr { condition, true_expr, false_expr } => {
                self.compile_expr(condition)?;
                let jump_false = self.instructions.len();
                self.emit(Opcode::PopJumpIfFalse(0), 0);
                self.compile_expr(true_expr)?;
                let jump_end = self.instructions.len();
                self.emit(Opcode::JumpForward(0), 0);
                let false_start = self.instructions.len();
                if let Some(offset) = self.instructions[jump_false].opcode.as_jump_offset_mut() {
                    *offset = false_start - jump_false;
                }
                self.compile_expr(false_expr)?;
                let end = self.instructions.len();
                if let Some(offset) = self.instructions[jump_end].opcode.as_jump_offset_mut() {
                    *offset = end - jump_end;
                }
            }
            Expr::ListComp { expr, iter, target, condition } => {
                // Create empty result list
                let result_idx = self.get_or_create_local("__lc_result");
                self.emit(Opcode::LoadNone, 0);
                self.emit(Opcode::BuildList(0), 0);
                self.emit(Opcode::StoreFast(result_idx), 0);

                // Compile iterable
                let iter_idx = self.get_or_create_local("__lc_iter");
                self.compile_expr(iter)?;
                self.emit(Opcode::StoreFast(iter_idx), 0);

                // Get length
                self.emit(Opcode::LoadFast(iter_idx), 0);
                let attr_idx = self.add_constant(Value::String("طول".to_string()));
                self.emit(Opcode::LoadConst(attr_idx), 0);
                self.emit(Opcode::GetAttribute, 0);
                let len_idx = self.get_or_create_local("__lc_len");
                self.emit(Opcode::StoreFast(len_idx), 0);

                // Init index
                let idx_idx = self.get_or_create_local("__lc_idx");
                let zero_idx = self.add_constant(Value::Integer(0));
                self.emit(Opcode::LoadConst(zero_idx), 0);
                self.emit(Opcode::StoreFast(idx_idx), 0);

                // Loop start
                let loop_start = self.instructions.len();
                self.emit(Opcode::LoadFast(idx_idx), 0);
                self.emit(Opcode::LoadFast(len_idx), 0);
                self.emit(Opcode::CompareLtIntInt, 0);
                let jump_to_end = self.instructions.len();
                self.emit(Opcode::PopJumpIfFalse(0), 0);

                // Load item
                self.emit(Opcode::LoadFast(iter_idx), 0);
                self.emit(Opcode::LoadFast(idx_idx), 0);
                self.emit(Opcode::SubscriptLocal(iter_idx, idx_idx), 0);
                let target_idx = self.get_or_create_local(target);
                self.emit(Opcode::StoreFast(target_idx), 0);

                // if clause
                if let Some(cond) = condition {
                    self.compile_expr(cond)?;
                    let jump_to_incr = self.instructions.len();
                    self.emit(Opcode::PopJumpIfFalse(0), 0);

                    // Compile expression and append to result (O(1) amortized)
                    self.compile_expr(expr)?;
                    self.emit(Opcode::ListAppendLocal(result_idx), 0);

                    // Increment
                    self.emit_with_operand(Opcode::IncrementInt(idx_idx), 1, 0);
                    self.emit(Opcode::JumpBackward(loop_start), 0);

                    // Patch condition jump to increment
                    let incr_pos = self.instructions.len() - 2;
                    if let Some(offset) = self.instructions[jump_to_incr].opcode.as_jump_offset_mut() {
                        *offset = incr_pos - jump_to_incr;
                    }
                } else {
                    // Compile expression and append to result (O(1) amortized)
                    self.compile_expr(expr)?;
                    self.emit(Opcode::ListAppendLocal(result_idx), 0);

                    // Increment
                    self.emit_with_operand(Opcode::IncrementInt(idx_idx), 1, 0);
                    self.emit(Opcode::JumpBackward(loop_start), 0);
                }

                // Patch end
                let loop_end = self.instructions.len();
                if let Some(offset) = self.instructions[jump_to_end].opcode.as_jump_offset_mut() {
                    *offset = loop_end - jump_to_end;
                }

                // Result is on the stack
                self.emit(Opcode::LoadFast(result_idx), 0);
            }
            Expr::SetComp { expr, iter, target, condition } => {
                let result_idx = self.get_or_create_local("__sc_result");
                self.emit(Opcode::LoadNone, 0);
                self.emit(Opcode::BuildSet(0), 0);
                self.emit(Opcode::StoreFast(result_idx), 0);

                let iter_idx = self.get_or_create_local("__sc_iter");
                self.compile_expr(iter)?;
                self.emit(Opcode::StoreFast(iter_idx), 0);

                self.emit(Opcode::LoadFast(iter_idx), 0);
                let attr_idx = self.add_constant(Value::String("طول".to_string()));
                self.emit(Opcode::LoadConst(attr_idx), 0);
                self.emit(Opcode::GetAttribute, 0);
                let len_idx = self.get_or_create_local("__sc_len");
                self.emit(Opcode::StoreFast(len_idx), 0);

                let idx_idx = self.get_or_create_local("__sc_idx");
                let zero_idx = self.add_constant(Value::Integer(0));
                self.emit(Opcode::LoadConst(zero_idx), 0);
                self.emit(Opcode::StoreFast(idx_idx), 0);

                let loop_start = self.instructions.len();
                self.emit(Opcode::LoadFast(idx_idx), 0);
                self.emit(Opcode::LoadFast(len_idx), 0);
                self.emit(Opcode::CompareLt, 0);
                let jump_to_end = self.instructions.len();
                self.emit(Opcode::PopJumpIfFalse(0), 0);

                self.emit(Opcode::LoadFast(iter_idx), 0);
                self.emit(Opcode::LoadFast(idx_idx), 0);
                self.emit(Opcode::Subscript, 0);
                let target_idx = self.get_or_create_local(target);
                self.emit(Opcode::StoreFast(target_idx), 0);

                if let Some(cond) = condition {
                    self.compile_expr(cond)?;
                    let jump_to_incr = self.instructions.len();
                    self.emit(Opcode::PopJumpIfFalse(0), 0);

                    self.emit(Opcode::LoadFast(result_idx), 0);
                    self.compile_expr(expr)?;
                    self.emit(Opcode::BuildSet(1), 0);
                    self.emit(Opcode::BinaryAdd, 0);
                    self.emit(Opcode::StoreFast(result_idx), 0);

                    self.emit_with_operand(Opcode::IncrementInt(idx_idx), 1, 0);
                    self.emit(Opcode::JumpBackward(loop_start), 0);

                    let incr_pos = self.instructions.len() - 2;
                    if let Some(offset) = self.instructions[jump_to_incr].opcode.as_jump_offset_mut() {
                        *offset = incr_pos - jump_to_incr;
                    }
                } else {
                    self.emit(Opcode::LoadFast(result_idx), 0);
                    self.compile_expr(expr)?;
                    self.emit(Opcode::BuildSet(1), 0);
                    self.emit(Opcode::BinaryAdd, 0);
                    self.emit(Opcode::StoreFast(result_idx), 0);

                    self.emit_with_operand(Opcode::IncrementInt(idx_idx), 1, 0);
                    self.emit(Opcode::JumpBackward(loop_start), 0);
                }

                let loop_end = self.instructions.len();
                if let Some(offset) = self.instructions[jump_to_end].opcode.as_jump_offset_mut() {
                    *offset = loop_end - jump_to_end;
                }

                self.emit(Opcode::LoadFast(result_idx), 0);
            }
            Expr::DictComp { key, value, iter, target, condition } => {
                let result_idx = self.get_or_create_local("__dc_result");
                self.emit(Opcode::LoadNone, 0);
                self.emit(Opcode::BuildDict(0), 0);
                self.emit(Opcode::StoreFast(result_idx), 0);

                let iter_idx = self.get_or_create_local("__dc_iter");
                self.compile_expr(iter)?;
                self.emit(Opcode::StoreFast(iter_idx), 0);

                self.emit(Opcode::LoadFast(iter_idx), 0);
                let attr_idx = self.add_constant(Value::String("طول".to_string()));
                self.emit(Opcode::LoadConst(attr_idx), 0);
                self.emit(Opcode::GetAttribute, 0);
                let len_idx = self.get_or_create_local("__dc_len");
                self.emit(Opcode::StoreFast(len_idx), 0);

                let idx_idx = self.get_or_create_local("__dc_idx");
                let zero_idx = self.add_constant(Value::Integer(0));
                self.emit(Opcode::LoadConst(zero_idx), 0);
                self.emit(Opcode::StoreFast(idx_idx), 0);

                let loop_start = self.instructions.len();
                self.emit(Opcode::LoadFast(idx_idx), 0);
                self.emit(Opcode::LoadFast(len_idx), 0);
                self.emit(Opcode::CompareLt, 0);
                let jump_to_end = self.instructions.len();
                self.emit(Opcode::PopJumpIfFalse(0), 0);

                self.emit(Opcode::LoadFast(iter_idx), 0);
                self.emit(Opcode::LoadFast(idx_idx), 0);
                self.emit(Opcode::Subscript, 0);
                let target_idx = self.get_or_create_local(target);
                self.emit(Opcode::StoreFast(target_idx), 0);

                if let Some(cond) = condition {
                    self.compile_expr(cond)?;
                    let jump_to_incr = self.instructions.len();
                    self.emit(Opcode::PopJumpIfFalse(0), 0);

                    self.emit(Opcode::LoadFast(result_idx), 0);
                    self.compile_expr(key)?;
                    self.compile_expr(value)?;
                    self.emit(Opcode::DictSetItem, 0);
                    self.emit(Opcode::StoreFast(result_idx), 0);

                    self.emit_with_operand(Opcode::IncrementInt(idx_idx), 1, 0);
                    self.emit(Opcode::JumpBackward(loop_start), 0);

                    let incr_pos = self.instructions.len() - 2;
                    if let Some(offset) = self.instructions[jump_to_incr].opcode.as_jump_offset_mut() {
                        *offset = incr_pos - jump_to_incr;
                    }
                } else {
                    self.emit(Opcode::LoadFast(result_idx), 0);
                    self.compile_expr(key)?;
                    self.compile_expr(value)?;
                    self.emit(Opcode::DictSetItem, 0);
                    self.emit(Opcode::StoreFast(result_idx), 0);

                    self.emit_with_operand(Opcode::IncrementInt(idx_idx), 1, 0);
                    self.emit(Opcode::JumpBackward(loop_start), 0);
                }

                let loop_end = self.instructions.len();
                if let Some(offset) = self.instructions[jump_to_end].opcode.as_jump_offset_mut() {
                    *offset = loop_end - jump_to_end;
                }

                self.emit(Opcode::LoadFast(result_idx), 0);
            }
            Expr::YieldExpr(value) => {
                if let Some(expr) = value {
                    self.compile_expr(expr)?;
                } else {
                    self.emit(Opcode::LoadNone, 0);
                }
                self.emit(Opcode::YieldValue, 0);
            }
            Expr::WalrusExpr { name, value } => {
                self.compile_expr(value)?;
                // Duplicate the value on stack (one for store, one for use)
                self.emit(Opcode::DupTop, 0);
                if self.global_names.contains(name) {
                    let name_idx = self.intern_name(name);
                    self.emit(Opcode::StoreName(name_idx), 0);
                } else if self.nonlocal_names.contains(name) {
                    if let Some((_, inner_idx, _)) = self.find_enclosing_var(name) {
                        self.emit(Opcode::StoreFast(inner_idx), 0);
                    } else {
                        let name_idx = self.intern_name(name);
                        self.emit(Opcode::StoreName(name_idx), 0);
                    }
                } else {
                    let idx = self.get_or_create_local(name);
                    self.emit(Opcode::StoreFast(idx), 0);
                }
            }
        }
        Ok(())
    }

    fn try_fold_constants(&self, left: &Expr, op: &BinOp, right: &Expr) -> Option<Value> {
        match (left, right) {
            (Expr::Integer(a), Expr::Integer(b)) => {
                let result = match op {
                    BinOp::Add => a.checked_add(*b)?,
                    BinOp::Sub => a.checked_sub(*b)?,
                    BinOp::Mul => a.checked_mul(*b)?,
                    BinOp::Div if *b != 0 => {
                        return Some(Value::Float(*a as f64 / *b as f64));
                    }
                    BinOp::FloorDiv if *b != 0 => a.checked_div(*b)?,
                    BinOp::Mod if *b != 0 => a.checked_rem(*b)?,
                    BinOp::Pow => (*a).checked_pow(*b as u32)?,
                    BinOp::BitAnd => a & b,
                    BinOp::BitOr => a | b,
                    BinOp::BitXor => a ^ b,
                    BinOp::Shl => a.checked_shl(*b as u32)?,
                    BinOp::Shr => a.checked_shr(*b as u32)?,
                    BinOp::Eq => return Some(Value::Boolean(a == b)),
                    BinOp::NotEq => return Some(Value::Boolean(a != b)),
                    BinOp::Lt => return Some(Value::Boolean(a < b)),
                    BinOp::Gt => return Some(Value::Boolean(a > b)),
                    BinOp::LtEq => return Some(Value::Boolean(a <= b)),
                    BinOp::GtEq => return Some(Value::Boolean(a >= b)),
                    _ => return None,
                };
                Some(Value::Integer(result))
            }
            (Expr::Float(a), Expr::Float(b)) => {
                let result = match op {
                    BinOp::Add => a + b,
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Div if *b != 0.0 => a / b,
                    BinOp::Eq => return Some(Value::Boolean(a == b)),
                    BinOp::NotEq => return Some(Value::Boolean(a != b)),
                    BinOp::Lt => return Some(Value::Boolean(a < b)),
                    BinOp::Gt => return Some(Value::Boolean(a > b)),
                    BinOp::LtEq => return Some(Value::Boolean(a <= b)),
                    BinOp::GtEq => return Some(Value::Boolean(a >= b)),
                    _ => return None,
                };
                Some(Value::Float(result))
            }
            (Expr::Integer(a), Expr::Float(b)) => {
                let af = *a as f64;
                let result = match op {
                    BinOp::Add => af + b,
                    BinOp::Sub => af - b,
                    BinOp::Mul => af * b,
                    BinOp::Div if *b != 0.0 => af / b,
                    _ => return None,
                };
                Some(Value::Float(result))
            }
            (Expr::Float(a), Expr::Integer(b)) => {
                let bf = *b as f64;
                let result = match op {
                    BinOp::Add => a + bf,
                    BinOp::Sub => a - bf,
                    BinOp::Mul => a * bf,
                    BinOp::Div if bf != 0.0 => a / bf,
                    _ => return None,
                };
                Some(Value::Float(result))
            }
            (Expr::String(a), Expr::String(b)) => {
                match op {
                    BinOp::Add => Some(Value::String(format!("{}{}", a, b))),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn try_fold_unary(&self, op: &UnaryOp, operand: &Expr) -> Option<Value> {
        match operand {
            Expr::Integer(n) => {
                match op {
                    UnaryOp::Neg => Some(Value::Integer(-n)),
                    UnaryOp::Not => Some(Value::Boolean(*n == 0)),
                    UnaryOp::BitNot => Some(Value::Integer(!n)),
                }
            }
            Expr::Float(f) => {
                match op {
                    UnaryOp::Neg => Some(Value::Float(-f)),
                    UnaryOp::Not => Some(Value::Boolean(*f == 0.0)),
                    _ => None,
                }
            }
            Expr::Boolean(b) => {
                match op {
                    UnaryOp::Not => Some(Value::Boolean(!b)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn compile_bin_op(&mut self, op: &BinOp) -> Result<()> {
        match op {
            BinOp::Add => { self.emit(Opcode::BinaryAdd, 0); Ok(()) },
            BinOp::Sub => { self.emit(Opcode::BinarySubtract, 0); Ok(()) },
            BinOp::Mul => { self.emit(Opcode::BinaryMultiply, 0); Ok(()) },
            BinOp::Div => { self.emit(Opcode::BinaryDivide, 0); Ok(()) },
            BinOp::FloorDiv => { self.emit(Opcode::BinaryFloorDivide, 0); Ok(()) },
            BinOp::Mod => { self.emit(Opcode::BinaryModulo, 0); Ok(()) },
            BinOp::Pow => { self.emit(Opcode::BinaryPower, 0); Ok(()) },
            BinOp::Eq => { self.emit(Opcode::CompareEq, 0); Ok(()) },
            BinOp::NotEq => { self.emit(Opcode::CompareNotEq, 0); Ok(()) },
            BinOp::Lt => { self.emit(Opcode::CompareLt, 0); Ok(()) },
            BinOp::Gt => { self.emit(Opcode::CompareGt, 0); Ok(()) },
            BinOp::LtEq => { self.emit(Opcode::CompareLtEq, 0); Ok(()) },
            BinOp::GtEq => { self.emit(Opcode::CompareGtEq, 0); Ok(()) },
            BinOp::And => { self.emit(Opcode::LogicalAnd, 0); Ok(()) },
            BinOp::Or => { self.emit(Opcode::LogicalOr, 0); Ok(()) },
            BinOp::NotIn => { self.emit(Opcode::CompareNotIn, 0); Ok(()) },
            BinOp::In => { self.emit(Opcode::CompareIn, 0); Ok(()) },
            BinOp::Is => { self.emit(Opcode::CompareIs, 0); Ok(()) },
            BinOp::IsNot => { self.emit(Opcode::CompareIsNot, 0); Ok(()) },
            BinOp::BitAnd => { self.emit(Opcode::BinaryBitAnd, 0); Ok(()) },
            BinOp::BitOr => { self.emit(Opcode::BinaryBitOr, 0); Ok(()) },
            BinOp::BitXor => { self.emit(Opcode::BinaryBitXor, 0); Ok(()) },
            BinOp::Shl => { self.emit(Opcode::BinaryShl, 0); Ok(()) },
            BinOp::Shr => { self.emit(Opcode::BinaryShr, 0); Ok(()) },
        }
    }

    fn compile_aug_op(&mut self, op: &AugOp) -> Result<()> {
        match op {
            AugOp::Add => { self.emit(Opcode::BinaryAdd, 0); Ok(()) },
            AugOp::Sub => { self.emit(Opcode::BinarySubtract, 0); Ok(()) },
            AugOp::Mul => { self.emit(Opcode::BinaryMultiply, 0); Ok(()) },
            AugOp::Div => { self.emit(Opcode::BinaryDivide, 0); Ok(()) },
            AugOp::FloorDiv => { self.emit(Opcode::BinaryFloorDivide, 0); Ok(()) },
            AugOp::Mod => { self.emit(Opcode::BinaryModulo, 0); Ok(()) },
            AugOp::Pow => { self.emit(Opcode::BinaryPower, 0); Ok(()) },
            AugOp::BitAnd => { self.emit(Opcode::BinaryBitAnd, 0); Ok(()) },
            AugOp::BitOr => { self.emit(Opcode::BinaryBitOr, 0); Ok(()) },
            AugOp::BitXor => { self.emit(Opcode::BinaryBitXor, 0); Ok(()) },
            AugOp::Shl => { self.emit(Opcode::BinaryShl, 0); Ok(()) },
            AugOp::Shr => { self.emit(Opcode::BinaryShr, 0); Ok(()) },
        }
    }

    fn compile_if(&mut self, condition: &Expr, body: &Block, elifs: &[(Expr, Block)], else_body: &Option<Block>) -> Result<()> {
        self.compile_expr(condition)?;
        let body_start = self.instructions.len();
        self.emit(Opcode::PopJumpIfFalse(0), 0);
        for stmt in &body.stmts {
            self.compile_stmt(stmt)?;
        }
        let else_start = if !elifs.is_empty() || else_body.is_some() {
            self.emit(Opcode::JumpForward(0), 0);
            Some(self.instructions.len())
        } else {
            None
        };
        let jump_target = self.instructions.len();
        if let Some(offset) = self.instructions[body_start].opcode.as_jump_offset_mut() {
            *offset = jump_target - body_start;
        }
        let mut elif_skip_jumps: Vec<usize> = Vec::new();
        for (elif_cond, elif_body) in elifs {
            self.compile_expr(elif_cond)?;
            let elif_jump = self.instructions.len();
            self.emit(Opcode::PopJumpIfFalse(0), 0);
            for stmt in &elif_body.stmts {
                self.compile_stmt(stmt)?;
            }
            let elif_end = self.instructions.len();
            self.emit(Opcode::JumpForward(0), 0);
            let elif_target = self.instructions.len();
            if let Some(offset) = self.instructions[elif_jump].opcode.as_jump_offset_mut() {
                *offset = elif_target - elif_jump;
            }
            elif_skip_jumps.push(elif_end);
        }
        if let Some(else_b) = else_body {
            for stmt in &else_b.stmts {
                self.compile_stmt(stmt)?;
            }
        }
        let after_if = self.instructions.len();
        for skip_idx in elif_skip_jumps {
            if let Some(offset) = self.instructions[skip_idx].opcode.as_jump_offset_mut() {
                *offset = after_if - skip_idx;
            }
        }
        if let Some(start) = else_start {
            if let Some(offset) = self.instructions[start - 1].opcode.as_jump_offset_mut() {
                *offset = after_if - (start - 1);
            }
        }
        Ok(())
    }

    fn compile_while(&mut self, condition: &Expr, body: &Block, else_body: Option<&Block>) -> Result<()> {
        let has_else = else_body.is_some();
        let else_flag = if has_else { Some(self.get_or_create_local("__while_else")) } else { None };
        if let Some(flag) = else_flag {
            let zero_idx = self.add_constant(Value::Integer(0));
            self.emit(Opcode::LoadConst(zero_idx), 0);
            self.emit(Opcode::StoreFast(flag), 0);
        }

        let limit_slot = if let Expr::BinaryOp { left, op: BinOp::Lt, right } = condition {
            if let (Expr::Identifier(_), Expr::Integer(_)) = (left.as_ref(), right.as_ref()) {
                let slot = self.get_or_create_local("__loop_limit");
                self.compile_expr(right)?;
                self.emit(Opcode::StoreFast(slot), 0);
                Some(slot)
            } else {
                None
            }
        } else {
            None
        };

        let loop_start = self.instructions.len();
        self.loop_stack.push(LoopContext {
            break_jumps: Vec::new(),
            continue_jumps: Vec::new(),
            continue_target: loop_start,
        });
        self.compile_expr(condition)?;
        let jump_to_end = self.instructions.len();
        self.emit(Opcode::PopJumpIfFalse(0), 0);
        for stmt in &body.stmts {
            self.compile_stmt(stmt)?;
        }
        if let Some(ctx) = self.loop_stack.last() {
            let target = ctx.continue_target;
            for &jmp_idx in &ctx.continue_jumps {
                if let Some(offset) = self.instructions[jmp_idx].opcode.as_jump_offset_mut() {
                    *offset = target;
                }
            }
        }
        self.emit(Opcode::JumpBackward(loop_start), 0);

        self.try_fuse_while_incremented_lt(loop_start, jump_to_end, limit_slot);

        let loop_end = self.instructions.len();
        if let Some(else_b) = else_body {
            for stmt in &else_b.stmts {
                self.compile_stmt(stmt)?;
            }
        }
        let final_end = self.instructions.len();
        if let Some(ctx) = self.loop_stack.pop() {
            for jmp_idx in ctx.break_jumps {
                if let Some(offset) = self.instructions[jmp_idx].opcode.as_jump_offset_mut() {
                    *offset = final_end - jmp_idx;
                }
            }
        }
        if let Opcode::PopJumpIfFalse(ref mut target) = self.instructions[jump_to_end].opcode {
            *target = loop_end - jump_to_end;
        }

        if let Some(flag) = else_flag {
            let one_idx = self.add_constant(Value::Integer(1));
            self.emit(Opcode::LoadConst(one_idx), 0);
            self.emit(Opcode::StoreFast(flag), 0);
        }

        Ok(())
    }

    fn try_fuse_while_incremented_lt(&mut self, loop_start: usize, _jump_to_end: usize, limit_slot: Option<usize>) {
        let n = self.instructions.len();
        // Need exactly 6 instructions in the loop pattern:
        // [0] LoadFast(local_idx)
        // [1] LoadConst(const_idx)
        // [2] CompareLt
        // [3] PopJumpIfFalse(offset)
        // [4] IncrementInt(local_idx)  (same local_idx as [0])
        // [5] JumpBackward(loop_start)
        if n - loop_start != 6 {
            return;
        }

        let local_idx = match &self.instructions[loop_start].opcode {
            Opcode::LoadFast(idx) => *idx,
            _ => return,
        };
        let const_idx = match &self.instructions[loop_start + 1].opcode {
            Opcode::LoadConst(idx) => *idx,
            _ => return,
        };
        match &self.instructions[loop_start + 2].opcode {
            Opcode::CompareLt => {}
            _ => return,
        }
        match &self.instructions[loop_start + 3].opcode {
            Opcode::PopJumpIfFalse(_) => {}
            _ => return,
        }
        let increment = match &self.instructions[loop_start + 4].opcode {
            Opcode::IncrementInt(idx) => {
                if *idx != local_idx { return; }
                self.instructions[loop_start + 4].operand
            }
            _ => return,
        }
        as i64;
        match &self.instructions[loop_start + 5].opcode {
            Opcode::JumpBackward(target) => {
                if *target != loop_start { return; }
            }
            _ => return,
        }

        // Pattern matched! Replace 6 instructions with fused opcode + 5 Nops.
        // Pack operands into a single usize (bitfield) to avoid string allocation/parsing at runtime:
        // bits 0-15:  local_idx (loop counter)
        // bits 16-31: loop_start (backward jump target)
        // bits 32-47: increment (i16)
        // bits 48-63: limit source — if limit_slot is Some, this is a local slot index;
        //              otherwise it's a constant pool index
        let limit_val = if let Some(slot) = limit_slot {
            slot | 0x8000  // High bit = 1 means local slot index
        } else {
            const_idx       // High bit = 0 means constant pool index
        };
        let packed = local_idx
            | (loop_start << 16)
            | (((increment as i16) as u16 as usize) << 32)
            | (limit_val << 48);

        self.instructions[loop_start].opcode = Opcode::JumpWhileIncrementedLt(packed);
        for i in 1..6 {
            self.instructions[loop_start + i].opcode = Opcode::Nop;
        }

        // Patch the PopJumpIfFalse offset: the loop now exits to loop_start + 6
        // The original PopJumpIfFalse had offset = loop_end - jump_to_end
        // Since we replaced the body, loop_end is still loop_start + 6
        // The jump_to_end index hasn't changed, and the target is still loop_start + 6
    }

    fn compile_for(&mut self, target: &Expr, iterable: &Expr, body: &Block, else_body: Option<&Block>) -> Result<()> {
        let is_tuple_target = matches!(target, Expr::Tuple(_));
        let target_names: Vec<String> = match target {
            Expr::Identifier(name) => vec![name.clone()],
            Expr::Tuple(names) => names.iter().map(|e| match e {
                Expr::Identifier(n) => n.clone(),
                _ => String::new(),
            }).collect(),
            _ => return Err(ArabiError::CompileError {
                message: "هدف لكل يجب ان يكون معرّفاً او مرجلاً".to_string(),
                span: arabi_core::span::Span::single(arabi_core::span::Position::start()),
            }),
        };

        let actual_target = if is_tuple_target {
            "__for_temp".to_string()
        } else {
            target_names[0].clone()
        };

        let is_range = if let Expr::Call { function, .. } = iterable {
            if let Expr::Identifier(name) = function.as_ref() {
                name == "مدى"
            } else {
                false
            }
        } else {
            false
        };

        if is_range {
            self.compile_for_range(&actual_target, iterable, &target_names, is_tuple_target, body)?;
        } else {
            self.compile_for_generic(&actual_target, iterable, &target_names, is_tuple_target, body)?;
        }

        if let Some(else_b) = else_body {
            for stmt in &else_b.stmts {
                self.compile_stmt(stmt)?;
            }
        }
        Ok(())
    }

    fn compile_for_range(&mut self, target_name: &str, iterable: &Expr, target_names: &[String], is_tuple: bool, body: &Block) -> Result<()> {
        self.loop_depth += 1;
        let depth = self.loop_depth;
        let iter_idx = self.get_or_create_local(&format!("__iter_{}", depth));
        let idx_idx = self.get_or_create_local(&format!("__idx_{}", depth));
        let target_idx = self.get_or_create_local(target_name);

        self.compile_expr(iterable)?;
        self.emit(Opcode::StoreFast(iter_idx), 0);

        let zero_idx = self.add_constant(Value::Integer(0));
        self.emit(Opcode::LoadConst(zero_idx), 0);
        self.emit(Opcode::StoreFast(idx_idx), 0);

        let for_range_ip = self.instructions.len();
        self.emit_with_operand(Opcode::ForRange(0), 0, 0);

        self.loop_stack.push(LoopContext {
            break_jumps: Vec::new(),
            continue_jumps: Vec::new(),
            continue_target: 0,
        });

        // Tuple unpacking: unpack __for_temp into individual targets
        if is_tuple {
            self.emit(Opcode::LoadFast(target_idx), 0);
            for (i, name) in target_names.iter().enumerate() {
                self.emit(Opcode::DupTop, 0);
                let idx = self.add_constant(Value::Integer(i as i64));
                self.emit(Opcode::LoadConst(idx), 0);
                self.emit(Opcode::Subscript, 0);
                let tidx = self.get_or_create_local(name);
                self.emit(Opcode::StoreFast(tidx), 0);
            }
            self.emit(Opcode::PopTop, 0);
        }

        for stmt in &body.stmts {
            self.compile_stmt(stmt)?;
        }

        // Patch continue jumps to point to ForRange (next iteration check)
        if let Some(ctx) = self.loop_stack.last_mut() {
            ctx.continue_target = for_range_ip;
        }
        if let Some(ctx) = self.loop_stack.last() {
            let target = ctx.continue_target;
            for &jmp_idx in &ctx.continue_jumps {
                if let Some(offset) = self.instructions[jmp_idx].opcode.as_jump_offset_mut() {
                    *offset = target;
                }
            }
        }

        // Pop loop context and patch break jumps
        let break_target = self.instructions.len() + 1; // after JumpBackward
        if let Some(ctx) = self.loop_stack.pop() {
            for jmp_idx in ctx.break_jumps {
                if let Some(offset) = self.instructions[jmp_idx].opcode.as_jump_offset_mut() {
                    *offset = break_target - jmp_idx;
                }
            }
        }

        // Jump back to ForRange for next iteration
        self.emit(Opcode::JumpBackward(for_range_ip), 0);

        // Patch ForRange operand with loop_end and bitfield encoding
        let loop_end = self.instructions.len();
        let packed = iter_idx
            | (target_idx << 16)
            | (idx_idx << 32)
            | (loop_end << 48);
        self.instructions[for_range_ip].operand = packed;

        self.loop_depth -= 1;
        Ok(())
    }

    fn compile_for_generic(&mut self, target_name: &str, iterable: &Expr, target_names: &[String], is_tuple: bool, body: &Block) -> Result<()> {
        self.loop_depth += 1;
        let depth = self.loop_depth;
        let iter_idx = self.get_or_create_local(&format!("__iter_{}", depth));
        let idx_idx = self.get_or_create_local(&format!("__idx_{}", depth));
        let len_idx = self.get_or_create_local(&format!("__len_{}", depth));
        let target_idx = self.get_or_create_local(target_name);

        // Compile iterable and materialize via ForIter
        // ForIter pops iterable, pushes: List(items), Integer(0), Integer(len)
        self.compile_expr(iterable)?;
        self.emit(Opcode::ForIter(0), 0);
        let for_iter_ip = self.instructions.len() - 1;

        // Store the three values: stack top = len, then idx=0, then list
        self.emit(Opcode::StoreFast(len_idx), 0);
        self.emit(Opcode::StoreFast(idx_idx), 0);
        self.emit(Opcode::StoreFast(iter_idx), 0);

        // Loop start (condition check)
        let loop_start = self.instructions.len();

        // Check if idx < len
        self.emit(Opcode::LoadFast(idx_idx), 0);
        self.emit(Opcode::LoadFast(len_idx), 0);
        self.emit(Opcode::CompareLt, 0);

        // Jump to end if false
        let jump_to_end = self.instructions.len();
        self.emit(Opcode::PopJumpIfFalse(0), 0);

        // Load current item: __iter[__idx]
        self.emit(Opcode::LoadFast(iter_idx), 0);
        self.emit(Opcode::LoadFast(idx_idx), 0);
        self.emit(Opcode::Subscript, 0);
        self.emit(Opcode::StoreFast(target_idx), 0);

        // Tuple unpacking: unpack __for_temp into individual targets
        if is_tuple {
            self.emit(Opcode::LoadFast(target_idx), 0);
            for (i, name) in target_names.iter().enumerate() {
                self.emit(Opcode::DupTop, 0);
                let idx = self.add_constant(Value::Integer(i as i64));
                self.emit(Opcode::LoadConst(idx), 0);
                self.emit(Opcode::Subscript, 0);
                let tidx = self.get_or_create_local(name);
                self.emit(Opcode::StoreFast(tidx), 0);
            }
            self.emit(Opcode::PopTop, 0);
        }

        // Push loop context with placeholder continue target
        self.loop_stack.push(LoopContext {
            break_jumps: Vec::new(),
            continue_jumps: Vec::new(),
            continue_target: 0, // will be patched
        });

        // Compile body
        for stmt in &body.stmts {
            self.compile_stmt(stmt)?;
        }

        // The increment step comes right after the body
        let increment_ip = self.instructions.len();

        // Patch the continue_target in the loop context
        if let Some(ctx) = self.loop_stack.last_mut() {
            ctx.continue_target = increment_ip;
        }

        // Patch all continue jumps
        if let Some(ctx) = self.loop_stack.last() {
            let target = ctx.continue_target;
            for &jmp_idx in &ctx.continue_jumps {
                if let Some(offset) = self.instructions[jmp_idx].opcode.as_jump_offset_mut() {
                    *offset = target;
                }
            }
        }

        // Pop loop context and patch break jumps
        let break_target = increment_ip + 2; // after IncrementInt + JumpBackward
        if let Some(ctx) = self.loop_stack.pop() {
            for jmp_idx in ctx.break_jumps {
                if let Some(offset) = self.instructions[jmp_idx].opcode.as_jump_offset_mut() {
                    *offset = break_target - jmp_idx;
                }
            }
        }

        // Increment index
        self.emit_with_operand(Opcode::IncrementInt(idx_idx), 1, 0);

        // Jump back to loop start
        self.emit(Opcode::JumpBackward(loop_start), 0);

        // Patch the conditional jump to end
        let loop_end = self.instructions.len();
        if let Some(offset) = self.instructions[jump_to_end].opcode.as_jump_offset_mut() {
            *offset = loop_end - jump_to_end;
        }

        // Patch ForIter skip operand to jump to loop_end if empty
        if let Some(offset) = self.instructions[for_iter_ip].opcode.as_jump_offset_mut() {
            *offset = loop_end - for_iter_ip;
        }

        self.loop_depth -= 1;
        Ok(())
    }

    /// Recursively collect self.field_name assignments from a block (constructor body).
    fn collect_fields_from_block(block: &Block, fields: &mut Vec<String>, self_name: &str) {
        for stmt in &block.stmts {
            match stmt {
                Stmt::Assign { target, .. } => {
                    if let Expr::Attribute { object, name } = target {
                        if let Expr::Identifier(obj_name) = &**object {
                            if obj_name == self_name && !fields.contains(name) {
                                fields.push(name.clone());
                            }
                        }
                    }
                }
                Stmt::If { body, elifs, else_body, .. } => {
                    Self::collect_fields_from_block(body, fields, self_name);
                    for (_, ebody) in elifs {
                        Self::collect_fields_from_block(ebody, fields, self_name);
                    }
                    if let Some(ebody) = else_body {
                        Self::collect_fields_from_block(ebody, fields, self_name);
                    }
                }
                Stmt::While { body, .. } => {
                    Self::collect_fields_from_block(body, fields, self_name);
                }
                Stmt::For { body, .. } => {
                    Self::collect_fields_from_block(body, fields, self_name);
                }
                _ => {}
            }
        }
    }

    fn compile_class_def(&mut self, name: &str, bases: &[Expr], body: &Block) -> Result<()> {
        // Collect method names
        let methods: Vec<String> = body.stmts.iter().filter_map(|s| {
            if let Stmt::FunctionDef { name: mname, .. } = s {
                Some(mname.clone())
            } else {
                None
            }
        }).collect();

        // Collect field layout from constructor (__تهيئة__)
        // This builds the compile-time field offset map for fast self.field access
        let mut field_layout: Vec<String> = Vec::new();
        for stmt in &body.stmts {
            if let Stmt::FunctionDef { name: mname, params: mparams, body: mbody, .. } = stmt {
                if mname == "__تهيئة__" {
                    // The first parameter is 'self' — use it for field collection
                    let self_name = mparams.first().map(|p| p.name.as_str()).unwrap_or("هذا");
                    Self::collect_fields_from_block(mbody, &mut field_layout, self_name);
                }
            }
        }
        if !field_layout.is_empty() {
            let field_map: Vec<(String, usize)> = field_layout.iter().enumerate()
                .map(|(i, name)| (name.clone(), i))
                .collect();
            self.class_field_map.insert(name.to_string(), field_map);
        }

        // Save local_map to avoid param name leaking into outer scope
        let saved_local_map = self.local_map.clone();
        let saved_current_class = self.current_class.take();

        // Set current class for self.field offset resolution in methods
        self.current_class = Some(name.to_string());

        // Compile body (defines methods as functions in globals)
        for stmt in &body.stmts {
            self.compile_stmt(stmt)?;
        }

        // Restore local_map (keep num_locals since function param indices reference it)
        self.local_map = saved_local_map;
        self.current_class = saved_current_class;

        // Compile base classes (load parent class objects onto stack)
        for base in bases {
            self.compile_expr(base)?;
        }

        // Create class: load each method by name, then use CreateClass
        let method_count = methods.len();
        let methods_str = methods.join(",");
        let methods_idx = self.add_constant(Value::String(methods_str));

        for mname in &methods {
            let method_name_idx = self.intern_name(mname);
            self.emit(Opcode::LoadName(method_name_idx), 0);
        }

        let parent_count = bases.len();
        let class_name_idx = self.intern_name(name);
        self.emit_with_operand(Opcode::CreateClass(class_name_idx, method_count, parent_count), methods_idx, 0);
        let store_name_idx = self.intern_name(name);
        self.emit(Opcode::StoreName(store_name_idx), 0);

        // Register exception hierarchy if parent is an exception class
        if !bases.is_empty() {
            let parent_names: Vec<String> = bases.iter().filter_map(|b| {
                if let Expr::Identifier(name) = b { Some(name.clone()) } else { None }
            }).collect();
            let hierarchy_str = format!("{}:{}", name, parent_names.join(","));
            let hier_idx = self.add_constant(Value::String(hierarchy_str));
            self.emit(Opcode::RegisterExceptionClass(hier_idx), 0);
        }

        Ok(())
    }

    fn compile_function_def(&mut self, name: &str, params: &[Param], body: &Block) -> Result<()> {
        let saved_map = self.local_map.clone();
        let saved_names = self.local_names.clone();
        let saved_num_locals = self.num_locals;
        let saved_in_constructor = self.in_constructor;

        // Save and set current function name for TCO
        let saved_func_name = self.current_function_name.take();
        self.current_function_name = Some(name.to_string());

        // Set in_constructor when compiling __تهيئة__ inside a class
        if name == "__تهيئة__" && self.current_class.is_some() {
            self.in_constructor = true;
        }
        let saved_nonlocal = self.nonlocal_names.clone();
        let saved_free_vars = self.free_vars.clone();
        let saved_self_local = self.self_local;

        // Push current scope onto enclosing stack for nested function resolution
        self.enclosing_local_maps.push(saved_map.clone());
        self.nonlocal_names.clear();
        self.free_vars.clear();

        // Push a new pending captures scope for this function
        self.pending_captures_stack.push(Vec::new());

        // Reset local scope for the inner function — only params are local
        self.local_map = std::collections::HashMap::new();
        self.local_names = Vec::new();
        self.num_locals = 0;

        // Pre-create param locals so we know their indices
        let param_indices: Vec<usize> = params.iter().map(|p| self.get_or_create_local(&p.name)).collect();

        // If first param is "هذا" or "ذ", mark it as self for instance field optimization
        if let Some(first) = params.first() {
            if first.name == "هذا" || first.name == "ذ" {
                self.self_local = Some(param_indices[0]);
            } else {
                self.self_local = None;
            }
        }

        // Emit JumpForward to skip over the function body
        self.emit(Opcode::JumpForward(0), 0);
        let skip_jump_idx = self.instructions.len() - 1;

        // Record body start
        let body_start = self.instructions.len();

        // Compile body in function scope
        for stmt in &body.stmts {
            self.compile_stmt(stmt)?;
        }

        // Process THIS function's pending captures (from inner functions referencing grandparent vars)
        // Add closure entries to self.local_map (the function's own scope, starting empty)
        let my_pending = self.pending_captures_stack.pop().unwrap_or_default();
        for (name, original_outer_idx, _parent_outer_idx) in &my_pending {
            if let Some(enclosing_map) = self.enclosing_local_maps.last() {
                if enclosing_map.contains_key(name) {
                    let closure_key = format!("__closure_{}", name);
                    if !self.local_map.contains_key(&closure_key) {
                        let idx = self.get_or_create_local(&closure_key);
                        self.free_vars.push((name.clone(), *original_outer_idx, idx));
                    }
                }
            }
        }

        // Detect if body contains yield (check if YieldValue was emitted)
        let is_generator = self.instructions[body_start..].iter()
            .any(|i| matches!(i.opcode, Opcode::YieldValue));

        // Emit default return (null) in case body doesn't have explicit return
        self.emit(Opcode::LoadNone, 0);
        self.emit(Opcode::ReturnValue, 0);

        // Patch the JumpForward to jump past the body
        let body_end = self.instructions.len();
        if let Some(offset) = self.instructions[skip_jump_idx].opcode.as_jump_offset_mut() {
            *offset = body_end - skip_jump_idx;
        }

        // Collect free variables info
        let captured_free_vars = self.free_vars.clone();

        // Pop enclosing scope
        self.enclosing_local_maps.pop();
        self.nonlocal_names = saved_nonlocal;

        // Compile default values (as constants)
        let defaults: Vec<Option<Value>> = params.iter().map(|p| {
            p.default.as_ref().and_then(|expr| {
                match expr {
                    Expr::Integer(n) => Some(Value::Integer(*n)),
                    Expr::Float(f) => Some(Value::Float(*f)),
                    Expr::String(s) => Some(Value::String(s.clone())),
                    Expr::Boolean(b) => Some(Value::Boolean(*b)),
                    Expr::Null => Some(Value::Null),
                    _ => None,
                }
            })
        }).collect();

        // Store defaults as a serialized string (for now, just store type+value pairs)
        let defaults_str: Vec<String> = defaults.iter().map(|d| {
            match d {
                Some(Value::Integer(n)) => format!("i{}", n),
                Some(Value::Float(f)) => format!("f{}", f),
                Some(Value::String(s)) => format!("s{}", s),
                Some(Value::Boolean(b)) => format!("b{}", b),
                Some(Value::Null) => "n".to_string(),
                None => "x".to_string(),
            }
        }).collect();
        let defaults_idx = self.add_constant(Value::String(defaults_str.join("|")));

        // Store param_names and param_indices
        let param_names_str = params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(",");
        let param_names_idx = self.add_constant(Value::String(param_names_str));

        let param_indices_str = param_indices.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        let param_indices_idx = self.add_constant(Value::String(param_indices_str));

        // Encode varargs and kwargs param names
        let varargs_name = params.iter().find(|p| p.is_varargs).map(|p| p.name.clone()).unwrap_or_default();
        let kwargs_name = params.iter().find(|p| p.is_kwargs).map(|p| p.name.clone()).unwrap_or_default();
        let varargs_idx = self.add_constant(Value::String(varargs_name));
        let kwargs_idx = self.add_constant(Value::String(kwargs_name));

        // Encode free variables as "inner_idx:outer_idx" pairs
        let free_vars_str: Vec<String> = captured_free_vars.iter()
            .map(|(_, outer_idx, inner_idx)| format!("{}:{}", inner_idx, outer_idx))
            .collect();
        let free_vars_idx = self.add_constant(Value::String(free_vars_str.join("|")));

        // Encode: body_start,param_names_idx,param_indices_idx,defaults_idx,varargs_idx,kwargs_idx,is_generator,free_vars_idx,num_locals
        let func_num_locals = self.num_locals;
        let func_info_idx = self.add_constant(Value::String(format!("{},{},{},{},{},{},{},{},{}", body_start, param_names_idx, param_indices_idx, defaults_idx, varargs_idx, kwargs_idx, if is_generator { "1" } else { "0" }, free_vars_idx, func_num_locals)));
        let func_name_idx = self.intern_name(name);
        self.emit_with_operand(Opcode::MakeFunction(func_name_idx, params.len(), func_info_idx), body_start, 0);
        let store_func_name_idx = self.intern_name(name);
        self.emit(Opcode::StoreName(store_func_name_idx), 0);
        self.local_map = saved_map;
        self.local_names = saved_names;
        self.num_locals = saved_num_locals;
        self.free_vars = saved_free_vars;
        self.self_local = saved_self_local;
        self.in_constructor = saved_in_constructor;
        self.current_function_name = saved_func_name;
        Ok(())
    }

    fn add_constant(&mut self, value: Value) -> usize {
        if let Value::String(ref s) = value {
            if let Some(&idx) = self.string_pool.get(s) {
                return idx;
            }
            let idx = self.constants.len();
            self.string_pool.insert(s.clone(), idx);
            self.constants.push(value);
            idx
        } else {
            let idx = self.constants.len();
            self.constants.push(value);
            idx
        }
    }

    fn intern_name(&mut self, name: &str) -> usize {
        if let Some(&idx) = self.name_index.get(name) {
            idx
        } else {
            let idx = self.names.len();
            self.names.push(name.to_string());
            self.name_index.insert(name.to_string(), idx);
            idx
        }
    }

    fn emit(&mut self, opcode: Opcode, _line: usize) {
        self.instructions.push(Instruction { opcode, line: self.current_line, operand: 0, quick: 0 });
    }

    fn emit_with_operand(&mut self, opcode: Opcode, operand: usize, _line: usize) {
        self.instructions.push(Instruction { opcode, line: self.current_line, operand, quick: 0 });
    }

    fn parse_fstring_parts(&self, s: &str) -> Vec<FPart> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '{' {
                if !current.is_empty() {
                    parts.push(FPart::Literal(current.clone()));
                    current.clear();
                }
                let mut expr = String::new();
                let mut depth = 1;
                while let Some(&next) = chars.peek() {
                    if next == '{' {
                        depth += 1;
                        expr.push(next);
                        chars.next();
                    } else if next == '}' {
                        depth -= 1;
                        if depth == 0 {
                            chars.next();
                            break;
                        }
                        expr.push(next);
                        chars.next();
                    } else {
                        expr.push(next);
                        chars.next();
                    }
                }
                // Split on first ':' to separate expression from format spec
                if let Some(colon_pos) = expr.find(':') {
                    let fmt_spec = Some(expr[colon_pos + 1..].to_string());
                    expr.truncate(colon_pos);
                    parts.push(FPart::Expr(expr, fmt_spec));
                } else {
                    parts.push(FPart::Expr(expr, None));
                }
            } else {
                current.push(c);
            }
        }
        if !current.is_empty() {
            parts.push(FPart::Literal(current));
        }
        parts
    }
}
