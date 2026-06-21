#[derive(Debug, Clone)]
pub enum Opcode {
    // Stack operations
    LoadConst(usize),
    PopTop,
    DupTop,

    // Variable operations
    LoadFast(usize),
    StoreFast(usize),
    LoadGlobal(usize),
    StoreGlobal(usize),
    LoadAttr(usize),
    StoreAttr(usize),
    LoadName(usize),
    StoreName(usize),

    // Arithmetic
    BinaryAdd,
    BinarySubtract,
    BinaryMultiply,
    BinaryDivide,
    BinaryFloorDivide,
    BinaryModulo,
    BinaryPower,
    UnaryNegative,
    UnaryNot,

    // Bitwise
    BinaryBitAnd,
    BinaryBitOr,
    BinaryBitXor,
    BinaryShl,
    BinaryShr,
    UnaryBitNot,

    // Comparison
    CompareEq,
    CompareNotEq,
    CompareLt,
    CompareGt,
    CompareLtEq,
    CompareGtEq,
    CompareIn,
    CompareNotIn,
    CompareIs,
    CompareIsNot,

    // Logical
    LogicalAnd,
    LogicalOr,

    // Control flow
    JumpForward(usize),
    JumpBackward(usize),
    PopJumpIfFalse(usize),
    PopJumpIfTrue(usize),

    // Functions
    CallFunction(usize),
    CallFunctionKw(usize),
    CallFunctionUnpacked(usize, usize, usize),
    ReturnValue,
    TailCall(usize),
    MakeFunction(usize, usize, usize),

    // Loops
    SetupLoop(usize),
    BreakLoop,
    ContinueLoop,

    // Exceptions
    SetupExcept(usize),
    SetupFinally(usize),
    EndExcept,
    Raise,
    CheckExceptionType(usize),
    RegisterExceptionClass(usize),

    // Import
    ImportModule(usize),
    ImportFrom(usize, usize),

    // Classes
    CreateClass(usize, usize, usize),
    CreateInstance(usize),
    CallMethod(usize, usize, u32),
    CallMethodVoid(usize, usize, u32),

    // Data structures
    BuildList(usize),
    BuildTuple(usize),
    BuildDict(usize),
    BuildSet(usize),
    BuildSlice,
    ListAppend(usize),
    DictSetItem,

    // Indexing
    Subscript,
    StoreSubscript,
    GetAttribute,

    // Iteration
    GetIter,
    ForIter(usize),

    // Special
    LoadNone,
    LoadSuper,
    LoadTrue,
    LoadFalse,
    StringCoerce,
    StringFormat,
    YieldValue,
    IncrementInt(usize),
    Nop,
    Halt,

    // Fused opcodes (Tier 2 optimizations)
    JumpWhileIncrementedLt(usize),

    // Fused for-range opcode (Tier 2 optimization)
    ForRange(usize),

    // Fused load+arithmetic opcodes (Tier 2 - eliminate intermediate stack ops)
    AddLocalImm(usize, i64),      // local[idx] + imm
    SubtractLocalImm(usize, i64), // local[idx] - imm

    // Fused load+compare+jump opcodes (Tier 2 - eliminate intermediate stack ops)
    PopJumpIfLeLocalImm(usize, i64, usize),  // if local[idx] <= imm, jump to target
    PopJumpIfLtLocalImm(usize, i64, usize),  // if local[idx] < imm, jump to target

    // Specialized integer opcodes (Tier 3 - PEP 659 adaptive specialization)
    BinaryAddIntInt,
    BinarySubIntInt,
    BinaryMulIntInt,
    BinaryDivIntInt,
    BinaryFloorDivIntInt,
    BinaryModIntInt,
    CompareLtIntInt,
    CompareGtIntInt,
    CompareEqIntInt,
    CompareLeIntInt,
    CompareGeIntInt,
    CompareNotEqIntInt,

    // Inline Cache specialized attribute opcodes (Tier 3 adaptive specialization)
    GetAttrICInstance,  // Fast path for instance attribute access (cached field slot)
    GetAttrICClass,     // Fast path for class attribute access (cached method slot)

    // Super-fused loop body opcodes (Tier 4 - eliminate per-iteration stack traffic)
    ModJumpIfNotZero(usize, usize, usize),  // if local[a] % local[b] != 0, jump to target
    ModAddIfZero(usize, usize, usize),  // if local[a] % local[b] == 0, local[c] += 1

    // Fused subscript opcodes (Tier 5 - eliminate subscript stack traffic)
    SubscriptLocal(usize, usize),           // push locals[a][locals[b]]
    SubscriptLocal2D(usize, usize, usize),  // push locals[a][locals[b]][locals[c]]
    StoreSubscriptLocal(usize, usize),      // pop value → locals[a][locals[b]] = value
    StoreSubscriptLocal2D(usize, usize, usize), // pop value → locals[a][locals[b]][locals[c]] = value
    SubscriptAddImm(usize, usize, i64),     // push locals[a][locals[b] + imm]
    StoreSubscriptAddImm(usize, usize, i64), // pop value → locals[a][locals[b] + imm] = value
    AddToSubscript2D(usize, usize, usize, usize), // locals[a][locals[b]][locals[c]] += locals[d]
    SwapAdjacent(usize, usize),                     // swap locals[a][locals[b]] <-> locals[a][locals[b]+1]
    PopJumpIfSubscriptGt(usize, usize, i64, usize), // if locals[a][locals[b]] > locals[a][locals[b]+imm], jump to target

    // Fused float math opcodes (Tier 6 - N-body / float-heavy loops)
    FloatAddMulLocal(usize, usize, usize),          // local[a] += local[b] * local[c]
    FloatAddMulImm(usize, usize, usize),             // local[a] += local[b] * const[c]
    NegFloorDivSqrAddImm(usize, usize, usize),       // local[a] = (-1.0) // (local[b] * local[b] + const[c])
    PopJumpIfLtLocal(usize, usize, usize),            // if local[a] < local[b], jump to target c

    // Fused Mandelbrot opcodes (Tier 7 - Mandelbrot inner loop)
    FloatSqrSubAddImm(usize, usize, usize, usize),    // local[a] = local[b]*local[b] - local[c]*local[c] + local[d]
    FloatMulMulAddImm(usize, usize, usize, usize),     // local[a] = 2.0 * local[b] * local[c] + local[d]
    PopJumpIfNotSqrAddSqrGtImm(usize, usize, usize, usize), // if local[a]*local[a] + local[b]*local[b] <= const[c], jump to target d

    // Fused list opcodes (Tier 8 - list-heavy benchmarks)
    ListAppendLocal(usize),                   // pop value from stack → locals[a].push(value) — bypass method dispatch
    AddLocalFromStack(usize),
    GetInstanceField(usize),
    SetInstanceField(usize),                 // pop value from stack → locals[a] += value — fused sum accumulation
    InplaceAddStrConst(usize),               // locals[a] += const[b] — in-place string concatenation

    // Fused subscript+compare+jump opcodes (Tier 9 - N-Queens / array-heavy benchmarks)
    PopJumpIfSubscriptEqLocal(usize, usize, usize, usize), // if locals[a][locals[b]] == locals[c], jump to target d
    PopJumpIfSubscriptNeLocal(usize, usize, usize, usize), // if locals[a][locals[b]] != locals[c], jump to target d
}

// Quick opcode constants for adaptive specialization
pub const QUICK_NONE: u8 = 0;
pub const QUICK_BINARY_ADD_INT: u8 = 1;
pub const QUICK_BINARY_SUB_INT: u8 = 2;
pub const QUICK_BINARY_MUL_INT: u8 = 3;
pub const QUICK_BINARY_DIV_INT: u8 = 4;
pub const QUICK_BINARY_FLOORDIV_INT: u8 = 5;
pub const QUICK_BINARY_MOD_INT: u8 = 6;
pub const QUICK_COMPARE_LT_INT: u8 = 7;
pub const QUICK_COMPARE_GT_INT: u8 = 8;
pub const QUICK_COMPARE_EQ_INT: u8 = 9;
pub const QUICK_COMPARE_LE_INT: u8 = 10;
pub const QUICK_COMPARE_GE_INT: u8 = 11;
pub const QUICK_COMPARE_NE_INT: u8 = 12;

// ===== Packed instruction opcode constants =====
pub const OP_HALT: u8 = 0;
pub const OP_NOP: u8 = 1;
pub const OP_DUP_TOP: u8 = 2;
pub const OP_POP_TOP: u8 = 3;
pub const OP_LOAD_CONST: u8 = 4;
pub const OP_LOAD_NONE: u8 = 5;
pub const OP_LOAD_SUPER: u8 = 6;
pub const OP_LOAD_TRUE: u8 = 7;
pub const OP_LOAD_FALSE: u8 = 8;
pub const OP_LOAD_FAST: u8 = 9;
pub const OP_STORE_FAST: u8 = 10;
pub const OP_LOAD_GLOBAL: u8 = 11;
pub const OP_STORE_GLOBAL: u8 = 12;
pub const OP_LOAD_ATTR: u8 = 13;
pub const OP_STORE_ATTR: u8 = 14;
pub const OP_LOAD_NAME: u8 = 15;
pub const OP_STORE_NAME: u8 = 16;
pub const OP_BINARY_ADD: u8 = 17;
pub const OP_BINARY_SUBTRACT: u8 = 18;
pub const OP_BINARY_MULTIPLY: u8 = 19;
pub const OP_BINARY_DIVIDE: u8 = 20;
pub const OP_BINARY_FLOOR_DIVIDE: u8 = 21;
pub const OP_BINARY_MODULO: u8 = 22;
pub const OP_BINARY_POWER: u8 = 23;
pub const OP_UNARY_NEGATIVE: u8 = 24;
pub const OP_UNARY_NOT: u8 = 25;
pub const OP_COMPARE_EQ: u8 = 26;
pub const OP_COMPARE_NOT_EQ: u8 = 27;
pub const OP_COMPARE_LT: u8 = 28;
pub const OP_COMPARE_GT: u8 = 29;
pub const OP_COMPARE_LT_EQ: u8 = 30;
pub const OP_COMPARE_GT_EQ: u8 = 31;
pub const OP_COMPARE_IN: u8 = 32;
pub const OP_COMPARE_NOT_IN: u8 = 33;
pub const OP_COMPARE_IS: u8 = 34;
pub const OP_COMPARE_IS_NOT: u8 = 35;
pub const OP_LOGICAL_AND: u8 = 36;
pub const OP_LOGICAL_OR: u8 = 37;
pub const OP_JUMP_FORWARD: u8 = 38;
pub const OP_JUMP_BACKWARD: u8 = 39;
pub const OP_POP_JUMP_IF_FALSE: u8 = 40;
pub const OP_POP_JUMP_IF_TRUE: u8 = 41;
pub const OP_CALL_FUNCTION: u8 = 42;
pub const OP_CALL_FUNCTION_KW: u8 = 43;
pub const OP_CALL_FUNCTION_UNPACKED: u8 = 44;
pub const OP_RETURN_VALUE: u8 = 45;
pub const OP_MAKE_FUNCTION: u8 = 46;
pub const OP_SETUP_LOOP: u8 = 47;
pub const OP_BREAK_LOOP: u8 = 48;
pub const OP_CONTINUE_LOOP: u8 = 49;
pub const OP_SETUP_EXCEPT: u8 = 50;
pub const OP_SETUP_FINALLY: u8 = 51;
pub const OP_END_EXCEPT: u8 = 52;
pub const OP_RAISE: u8 = 53;
pub const OP_CHECK_EXCEPTION_TYPE: u8 = 54;
pub const OP_REGISTER_EXCEPTION_CLASS: u8 = 55;
pub const OP_IMPORT_MODULE: u8 = 56;
pub const OP_IMPORT_FROM: u8 = 57;
pub const OP_CREATE_CLASS: u8 = 58;
pub const OP_CREATE_INSTANCE: u8 = 59;
pub const OP_CALL_METHOD: u8 = 60;
pub const OP_BUILD_LIST: u8 = 61;
pub const OP_BUILD_TUPLE: u8 = 62;
pub const OP_BUILD_DICT: u8 = 63;
pub const OP_BUILD_SET: u8 = 64;
pub const OP_BUILD_SLICE: u8 = 65;
pub const OP_LIST_APPEND: u8 = 66;
pub const OP_DICT_SET_ITEM: u8 = 67;
pub const OP_SUBSCRIPT: u8 = 68;
pub const OP_STORE_SUBSCRIPT: u8 = 69;
pub const OP_GET_ATTRIBUTE: u8 = 70;
pub const OP_GET_ITER: u8 = 71;
pub const OP_FOR_ITER: u8 = 72;
pub const OP_STRING_COERCE: u8 = 73;
pub const OP_STRING_FORMAT: u8 = 74;
pub const OP_YIELD_VALUE: u8 = 75;
pub const OP_INCREMENT_INT: u8 = 76;
pub const OP_JUMP_WHILE_INCREMENTED_LT: u8 = 77;
pub const OP_FOR_RANGE: u8 = 78;
pub const OP_ADD_LOCAL_IMM: u8 = 79;
pub const OP_SUBTRACT_LOCAL_IMM: u8 = 80;
pub const OP_POP_JUMP_IF_LE_LOCAL_IMM: u8 = 81;
pub const OP_POP_JUMP_IF_LT_LOCAL_IMM: u8 = 82;
pub const OP_BINARY_ADD_INT_INT: u8 = 83;
pub const OP_BINARY_SUB_INT_INT: u8 = 84;
pub const OP_BINARY_MUL_INT_INT: u8 = 85;
pub const OP_BINARY_DIV_INT_INT: u8 = 86;
pub const OP_BINARY_FLOOR_DIV_INT_INT: u8 = 87;
pub const OP_BINARY_MOD_INT_INT: u8 = 88;
pub const OP_COMPARE_LT_INT_INT: u8 = 89;
pub const OP_COMPARE_GT_INT_INT: u8 = 90;
pub const OP_COMPARE_EQ_INT_INT: u8 = 91;
pub const OP_COMPARE_LE_INT_INT: u8 = 92;
pub const OP_COMPARE_GE_INT_INT: u8 = 93;
pub const OP_COMPARE_NOT_EQ_INT_INT: u8 = 94;
pub const OP_GET_ATTR_IC_INSTANCE: u8 = 95;
pub const OP_GET_ATTR_IC_CLASS: u8 = 96;
pub const OP_MOD_JUMP_IF_NOT_ZERO: u8 = 97;
pub const OP_SUBSCRIPT_LOCAL: u8 = 98;
pub const OP_SUBSCRIPT_LOCAL_2D: u8 = 99;
pub const OP_STORE_SUBSCRIPT_LOCAL: u8 = 100;
pub const OP_STORE_SUBSCRIPT_LOCAL_2D: u8 = 101;
pub const OP_SUBSCRIPT_ADD_IMM: u8 = 102;
pub const OP_STORE_SUBSCRIPT_ADD_IMM: u8 = 103;
pub const OP_ADD_TO_SUBSCRIPT_2D: u8 = 104;
pub const OP_SWAP_ADJACENT: u8 = 105;
pub const OP_POP_JUMP_IF_SUBSCRIPT_GT: u8 = 106;
pub const OP_MOD_ADD_IF_ZERO: u8 = 107;
pub const OP_FLOAT_ADD_MUL_LOCAL: u8 = 108;
pub const OP_FLOAT_ADD_MUL_IMM: u8 = 109;
pub const OP_NEG_FLOOR_DIV_SQR_ADD_IMM: u8 = 110;
pub const OP_POP_JUMP_IF_LT_LOCAL: u8 = 111;
pub const OP_FLOAT_SQR_SUB_ADD_IMM: u8 = 112;
pub const OP_FLOAT_MUL_MUL_ADD_IMM: u8 = 113;
pub const OP_POP_JUMP_IF_NOT_SQR_ADD_SQR_GT_IMM: u8 = 114;
pub const OP_LIST_APPEND_LOCAL: u8 = 115;
pub const OP_ADD_LOCAL_FROM_STACK: u8 = 116;
pub const OP_GET_INSTANCE_FIELD: u8 = 117;
pub const OP_SET_INSTANCE_FIELD: u8 = 118;
pub const OP_INPLACE_ADD_STR_CONST: u8 = 119;
pub const OP_TAIL_CALL: u8 = 120;
pub const OP_CALL_METHOD_VOID: u8 = 121;
pub const OP_BINARY_BIT_AND: u8 = 122;
pub const OP_BINARY_BIT_OR: u8 = 123;
pub const OP_BINARY_BIT_XOR: u8 = 124;
pub const OP_BINARY_SHL: u8 = 125;
pub const OP_BINARY_SHR: u8 = 126;
pub const OP_UNARY_BIT_NOT: u8 = 127;
pub const OP_POP_JUMP_IF_SUBSCRIPT_EQ_LOCAL: u8 = 128;
pub const OP_POP_JUMP_IF_SUBSCRIPT_NE_LOCAL: u8 = 129;

impl Opcode {
    pub fn as_jump_offset_mut(&mut self) -> Option<&mut usize> {
        match self {
            Opcode::JumpForward(offset) => Some(offset),
            Opcode::JumpBackward(offset) => Some(offset),
            Opcode::PopJumpIfFalse(offset) => Some(offset),
            Opcode::PopJumpIfTrue(offset) => Some(offset),
            Opcode::ForIter(offset) => Some(offset),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BytecodeModule {
    pub instructions: Vec<Instruction>,
    pub constants: Vec<crate::compiler::Value>,
    pub names: Vec<String>,
    pub num_locals: usize,
    pub local_names: Vec<String>,
    pub local_name_map: std::collections::HashMap<String, usize>,
    pub global_names: std::collections::HashSet<String>,
    pub packed: Vec<PackedInstr>,
    pub lines: Vec<u16>,
}

impl BytecodeModule {
    pub fn pack_all(&mut self) {
        self.packed = self.instructions.iter().map(|i| i.pack()).collect();
        self.lines = self.instructions.iter().map(|i| i.line.min(u16::MAX as usize) as u16).collect();
    }
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub line: usize,
    pub operand: usize,
    pub quick: u8,  // 0 = generic, non-zero = specialized variant (PEP 659 model)
}

// ===== Packed instruction format (u64, 8 bytes) =====
// Layout: [opcode:u8][a:u8][b:u16][c:u32] = 8 bytes total
//   bits 0-7:    opcode byte
//   bits 8-15:   operand a (local index, argc, etc.)
//   bits 16-31:  operand b (const index, i16 immediate, jump offset)
//   bits 32-63:  operand c (jump target, flags, etc.)
pub type PackedInstr = u64;

// ===== FVN-1a hash for method cache (precomputed at compile time) =====
#[inline(always)]
pub fn fnv1a_hash(s: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

#[inline(always)]
pub fn pack_instr(opcode: u8, a: u8, b: u16, c: u32) -> PackedInstr {
    (opcode as u64)
        | ((a as u64) << 8)
        | ((b as u64) << 16)
        | ((c as u64) << 32)
}

#[inline(always)]
pub fn pack_instr_i64(opcode: u8, a: u8, imm: i64, c: u32) -> PackedInstr {
    (opcode as u64)
        | ((a as u64) << 8)
        | (((imm as u16) as u64) << 16)
        | ((c as u64) << 32)
}

impl Instruction {
    pub fn pack(&self) -> PackedInstr {
        match &self.opcode {
            Opcode::Halt => pack_instr(OP_HALT, 0, 0, 0),
            Opcode::Nop => pack_instr(OP_NOP, 0, 0, 0),
            Opcode::DupTop => pack_instr(OP_DUP_TOP, 0, 0, 0),
            Opcode::PopTop => pack_instr(OP_POP_TOP, 0, 0, 0),
            Opcode::LoadConst(idx) => pack_instr(OP_LOAD_CONST, 0, *idx as u16, 0),
            Opcode::LoadNone => pack_instr(OP_LOAD_NONE, 0, 0, 0),
            Opcode::LoadSuper => pack_instr(OP_LOAD_SUPER, 0, 0, 0),
            Opcode::LoadTrue => pack_instr(OP_LOAD_TRUE, 0, 0, 0),
            Opcode::LoadFalse => pack_instr(OP_LOAD_FALSE, 0, 0, 0),
            Opcode::LoadFast(idx) => pack_instr(OP_LOAD_FAST, *idx as u8, 0, 0),
            Opcode::StoreFast(idx) => pack_instr(OP_STORE_FAST, *idx as u8, 0, 0),
            Opcode::LoadGlobal(idx) => pack_instr(OP_LOAD_GLOBAL, 0, *idx as u16, 0),
            Opcode::StoreGlobal(idx) => pack_instr(OP_STORE_GLOBAL, 0, *idx as u16, 0),
            Opcode::LoadAttr(idx) => pack_instr(OP_LOAD_ATTR, 0, *idx as u16, 0),
            Opcode::StoreAttr(idx) => pack_instr(OP_STORE_ATTR, 0, *idx as u16, 0),
            Opcode::LoadName(idx) => pack_instr(OP_LOAD_NAME, 0, *idx as u16, 0),
            Opcode::StoreName(idx) => pack_instr(OP_STORE_NAME, 0, *idx as u16, 0),
            Opcode::BinaryAdd => pack_instr(OP_BINARY_ADD, 0, 0, 0),
            Opcode::BinarySubtract => pack_instr(OP_BINARY_SUBTRACT, 0, 0, 0),
            Opcode::BinaryMultiply => pack_instr(OP_BINARY_MULTIPLY, 0, 0, 0),
            Opcode::BinaryDivide => pack_instr(OP_BINARY_DIVIDE, 0, 0, 0),
            Opcode::BinaryFloorDivide => pack_instr(OP_BINARY_FLOOR_DIVIDE, 0, 0, 0),
            Opcode::BinaryModulo => pack_instr(OP_BINARY_MODULO, 0, 0, 0),
            Opcode::BinaryPower => pack_instr(OP_BINARY_POWER, 0, 0, 0),
            Opcode::UnaryNegative => pack_instr(OP_UNARY_NEGATIVE, 0, 0, 0),
            Opcode::UnaryNot => pack_instr(OP_UNARY_NOT, 0, 0, 0),
            Opcode::BinaryBitAnd => pack_instr(OP_BINARY_BIT_AND, 0, 0, 0),
            Opcode::BinaryBitOr => pack_instr(OP_BINARY_BIT_OR, 0, 0, 0),
            Opcode::BinaryBitXor => pack_instr(OP_BINARY_BIT_XOR, 0, 0, 0),
            Opcode::BinaryShl => pack_instr(OP_BINARY_SHL, 0, 0, 0),
            Opcode::BinaryShr => pack_instr(OP_BINARY_SHR, 0, 0, 0),
            Opcode::UnaryBitNot => pack_instr(OP_UNARY_BIT_NOT, 0, 0, 0),
            Opcode::CompareEq => pack_instr(OP_COMPARE_EQ, 0, 0, 0),
            Opcode::CompareNotEq => pack_instr(OP_COMPARE_NOT_EQ, 0, 0, 0),
            Opcode::CompareLt => pack_instr(OP_COMPARE_LT, 0, 0, 0),
            Opcode::CompareGt => pack_instr(OP_COMPARE_GT, 0, 0, 0),
            Opcode::CompareLtEq => pack_instr(OP_COMPARE_LT_EQ, 0, 0, 0),
            Opcode::CompareGtEq => pack_instr(OP_COMPARE_GT_EQ, 0, 0, 0),
            Opcode::CompareIn => pack_instr(OP_COMPARE_IN, 0, 0, 0),
            Opcode::CompareNotIn => pack_instr(OP_COMPARE_NOT_IN, 0, 0, 0),
            Opcode::CompareIs => pack_instr(OP_COMPARE_IS, 0, 0, 0),
            Opcode::CompareIsNot => pack_instr(OP_COMPARE_IS_NOT, 0, 0, 0),
            Opcode::LogicalAnd => pack_instr(OP_LOGICAL_AND, 0, 0, 0),
            Opcode::LogicalOr => pack_instr(OP_LOGICAL_OR, 0, 0, 0),
            Opcode::JumpForward(offset) => pack_instr(OP_JUMP_FORWARD, 0, 0, *offset as u32),
            Opcode::JumpBackward(offset) => pack_instr(OP_JUMP_BACKWARD, 0, 0, *offset as u32),
            Opcode::PopJumpIfFalse(offset) => pack_instr(OP_POP_JUMP_IF_FALSE, 0, 0, *offset as u32),
            Opcode::PopJumpIfTrue(offset) => pack_instr(OP_POP_JUMP_IF_TRUE, 0, 0, *offset as u32),
            Opcode::CallFunction(argc) => pack_instr(OP_CALL_FUNCTION, *argc as u8, 0, 0),
            Opcode::CallFunctionKw(argc) => pack_instr(OP_CALL_FUNCTION_KW, *argc as u8, 0, 0),
            Opcode::CallFunctionUnpacked(argc, kwargc, star) => {
                pack_instr(OP_CALL_FUNCTION_UNPACKED, *argc as u8, *kwargc as u16, *star as u32)
            }
            Opcode::ReturnValue => pack_instr(OP_RETURN_VALUE, 0, 0, 0),
            Opcode::TailCall(argc) => pack_instr(OP_TAIL_CALL, *argc as u8, 0, 0),
            Opcode::MakeFunction(flags, defaults, closures) => {
                pack_instr(OP_MAKE_FUNCTION, *flags as u8, *defaults as u16, *closures as u32)
            }
            Opcode::SetupLoop(offset) => pack_instr(OP_SETUP_LOOP, 0, 0, *offset as u32),
            Opcode::BreakLoop => pack_instr(OP_BREAK_LOOP, 0, 0, 0),
            Opcode::ContinueLoop => pack_instr(OP_CONTINUE_LOOP, 0, 0, 0),
            Opcode::SetupExcept(offset) => pack_instr(OP_SETUP_EXCEPT, 0, 0, *offset as u32),
            Opcode::SetupFinally(offset) => pack_instr(OP_SETUP_FINALLY, 0, 0, *offset as u32),
            Opcode::EndExcept => pack_instr(OP_END_EXCEPT, 0, 0, 0),
            Opcode::Raise => pack_instr(OP_RAISE, 0, 0, 0),
            Opcode::CheckExceptionType(idx) => pack_instr(OP_CHECK_EXCEPTION_TYPE, 0, *idx as u16, 0),
            Opcode::RegisterExceptionClass(idx) => pack_instr(OP_REGISTER_EXCEPTION_CLASS, 0, *idx as u16, 0),
            Opcode::ImportModule(idx) => pack_instr(OP_IMPORT_MODULE, 0, *idx as u16, 0),
            Opcode::ImportFrom(from, name) => pack_instr(OP_IMPORT_FROM, 0, *from as u16, *name as u32),
            Opcode::CreateClass(class_name, method_count, parent_count) => {
                pack_instr(OP_CREATE_CLASS, *class_name as u8, self.operand as u16, ((*parent_count as u32) << 16) | (*method_count as u32))
            }
            Opcode::CreateInstance(idx) => pack_instr(OP_CREATE_INSTANCE, *idx as u8, 0, 0),
            Opcode::CallMethod(idx, argc, hash) => pack_instr(OP_CALL_METHOD, *idx as u8, *argc as u16, *hash),
            Opcode::CallMethodVoid(idx, argc, hash) => pack_instr(OP_CALL_METHOD_VOID, *idx as u8, *argc as u16, *hash),
            Opcode::BuildList(len) => pack_instr(OP_BUILD_LIST, *len as u8, 0, 0),
            Opcode::BuildTuple(len) => pack_instr(OP_BUILD_TUPLE, *len as u8, 0, 0),
            Opcode::BuildDict(len) => pack_instr(OP_BUILD_DICT, *len as u8, 0, 0),
            Opcode::BuildSet(len) => pack_instr(OP_BUILD_SET, *len as u8, 0, 0),
            Opcode::BuildSlice => pack_instr(OP_BUILD_SLICE, 0, 0, 0),
            Opcode::ListAppend(idx) => pack_instr(OP_LIST_APPEND, 0, *idx as u16, 0),
            Opcode::DictSetItem => pack_instr(OP_DICT_SET_ITEM, 0, 0, 0),
            Opcode::Subscript => pack_instr(OP_SUBSCRIPT, 0, 0, 0),
            Opcode::StoreSubscript => pack_instr(OP_STORE_SUBSCRIPT, 0, 0, 0),
            Opcode::GetAttribute => pack_instr(OP_GET_ATTRIBUTE, 0, 0, 0),
            Opcode::GetIter => pack_instr(OP_GET_ITER, 0, 0, 0),
            Opcode::ForIter(offset) => pack_instr(OP_FOR_ITER, 0, 0, *offset as u32),
            Opcode::StringCoerce => pack_instr(OP_STRING_COERCE, 0, 0, 0),
            Opcode::StringFormat => pack_instr(OP_STRING_FORMAT, 0, 0, 0),
            Opcode::YieldValue => pack_instr(OP_YIELD_VALUE, 0, 0, 0),
            Opcode::IncrementInt(idx) => pack_instr(OP_INCREMENT_INT, *idx as u8, self.operand as u16, 0),
            Opcode::JumpWhileIncrementedLt(packed_val) => {
                let local_idx = (packed_val & 0xFFFF) as u8;
                let loop_start = ((packed_val >> 16) & 0xFFFF) as u16;
                let increment = ((packed_val >> 32) & 0xFFFF) as u16;
                let limit_field = ((packed_val >> 48) & 0xFFFF) as u16;
                pack_instr(OP_JUMP_WHILE_INCREMENTED_LT, local_idx, loop_start, (increment as u32) | ((limit_field as u32) << 16))
            }
            Opcode::ForRange(_) => {
                let packed_val = self.operand as u64;
                let iter_local = (packed_val & 0xFFFF) as u8;
                let target_local = ((packed_val >> 16) & 0xFFFF) as u16;
                let idx_local = ((packed_val >> 32) & 0xFFFF) as u32;
                let loop_end = ((packed_val >> 48) & 0xFFFF) as u32;
                pack_instr(OP_FOR_RANGE, iter_local, target_local, idx_local | (loop_end << 16))
            }
            Opcode::AddLocalImm(idx, imm) => pack_instr_i64(OP_ADD_LOCAL_IMM, *idx as u8, *imm, 0),
            Opcode::SubtractLocalImm(idx, imm) => pack_instr_i64(OP_SUBTRACT_LOCAL_IMM, *idx as u8, *imm, 0),
            Opcode::PopJumpIfLeLocalImm(idx, imm, target) => {
                pack_instr_i64(OP_POP_JUMP_IF_LE_LOCAL_IMM, *idx as u8, *imm, *target as u32)
            }
            Opcode::PopJumpIfLtLocalImm(idx, imm, target) => {
                pack_instr_i64(OP_POP_JUMP_IF_LT_LOCAL_IMM, *idx as u8, *imm, *target as u32)
            }
            Opcode::BinaryAddIntInt => pack_instr(OP_BINARY_ADD_INT_INT, 0, 0, 0),
            Opcode::BinarySubIntInt => pack_instr(OP_BINARY_SUB_INT_INT, 0, 0, 0),
            Opcode::BinaryMulIntInt => pack_instr(OP_BINARY_MUL_INT_INT, 0, 0, 0),
            Opcode::BinaryDivIntInt => pack_instr(OP_BINARY_DIV_INT_INT, 0, 0, 0),
            Opcode::BinaryFloorDivIntInt => pack_instr(OP_BINARY_FLOOR_DIV_INT_INT, 0, 0, 0),
            Opcode::BinaryModIntInt => pack_instr(OP_BINARY_MOD_INT_INT, 0, 0, 0),
            Opcode::CompareLtIntInt => pack_instr(OP_COMPARE_LT_INT_INT, 0, 0, 0),
            Opcode::CompareGtIntInt => pack_instr(OP_COMPARE_GT_INT_INT, 0, 0, 0),
            Opcode::CompareEqIntInt => pack_instr(OP_COMPARE_EQ_INT_INT, 0, 0, 0),
            Opcode::CompareLeIntInt => pack_instr(OP_COMPARE_LE_INT_INT, 0, 0, 0),
            Opcode::CompareGeIntInt => pack_instr(OP_COMPARE_GE_INT_INT, 0, 0, 0),
            Opcode::CompareNotEqIntInt => pack_instr(OP_COMPARE_NOT_EQ_INT_INT, 0, 0, 0),
            Opcode::GetAttrICInstance => pack_instr(OP_GET_ATTR_IC_INSTANCE, 0, 0, 0),
            Opcode::GetAttrICClass => pack_instr(OP_GET_ATTR_IC_CLASS, 0, 0, 0),
            Opcode::ModJumpIfNotZero(a, b, target) => {
                pack_instr(OP_MOD_JUMP_IF_NOT_ZERO, *a as u8, *b as u16, *target as u32)
            }
            Opcode::ModAddIfZero(a, b, c) => {
                pack_instr(OP_MOD_ADD_IF_ZERO, *a as u8, *b as u16, *c as u32)
            }
            Opcode::SubscriptLocal(a, b) => {
                pack_instr(OP_SUBSCRIPT_LOCAL, *a as u8, *b as u16, 0)
            }
            Opcode::SubscriptLocal2D(a, b, c) => {
                pack_instr(OP_SUBSCRIPT_LOCAL_2D, *a as u8, *b as u16, *c as u32)
            }
            Opcode::StoreSubscriptLocal(a, b) => {
                pack_instr(OP_STORE_SUBSCRIPT_LOCAL, *a as u8, *b as u16, 0)
            }
            Opcode::StoreSubscriptLocal2D(a, b, c) => {
                pack_instr(OP_STORE_SUBSCRIPT_LOCAL_2D, *a as u8, *b as u16, *c as u32)
            }
            Opcode::SubscriptAddImm(a, b, imm) => {
                pack_instr_i64(OP_SUBSCRIPT_ADD_IMM, *a as u8, *imm, *b as u32)
            }
            Opcode::StoreSubscriptAddImm(a, b, imm) => {
                pack_instr_i64(OP_STORE_SUBSCRIPT_ADD_IMM, *a as u8, *imm, *b as u32)
            }
            Opcode::AddToSubscript2D(a, b, c, d) => {
                pack_instr(OP_ADD_TO_SUBSCRIPT_2D, *a as u8, *b as u16, ((*c as u32) << 16) | (*d as u32))
            }
            Opcode::SwapAdjacent(a, b) => {
                pack_instr(OP_SWAP_ADJACENT, *a as u8, *b as u16, 0)
            }
            Opcode::PopJumpIfSubscriptGt(a, b, imm, target) => {
                let b_packed = (*b as u16) | ((*imm as u16) << 8);
                pack_instr(OP_POP_JUMP_IF_SUBSCRIPT_GT, *a as u8, b_packed, *target as u32)
            }
            Opcode::FloatAddMulLocal(a, b, c) => {
                pack_instr(OP_FLOAT_ADD_MUL_LOCAL, *a as u8, *b as u16, *c as u32)
            }
            Opcode::FloatAddMulImm(a, b, c) => {
                pack_instr(OP_FLOAT_ADD_MUL_IMM, *a as u8, *b as u16, *c as u32)
            }
            Opcode::NegFloorDivSqrAddImm(a, b, c) => {
                pack_instr(OP_NEG_FLOOR_DIV_SQR_ADD_IMM, *a as u8, *b as u16, *c as u32)
            }
            Opcode::PopJumpIfLtLocal(a, b, target) => {
                pack_instr(OP_POP_JUMP_IF_LT_LOCAL, *a as u8, *b as u16, *target as u32)
            }
            Opcode::FloatSqrSubAddImm(a, b, c, d) => {
                pack_instr(OP_FLOAT_SQR_SUB_ADD_IMM, *a as u8, *b as u16, ((*c as u32) << 16) | (*d as u32))
            }
            Opcode::FloatMulMulAddImm(a, b, c, d) => {
                pack_instr(OP_FLOAT_MUL_MUL_ADD_IMM, *a as u8, *b as u16, ((*c as u32) << 16) | (*d as u32))
            }
            Opcode::PopJumpIfNotSqrAddSqrGtImm(a, b, c, target) => {
                pack_instr(OP_POP_JUMP_IF_NOT_SQR_ADD_SQR_GT_IMM, *a as u8, *b as u16, ((*c as u32) << 16) | (*target as u32))
            }
            Opcode::ListAppendLocal(a) => {
                pack_instr(OP_LIST_APPEND_LOCAL, *a as u8, 0, 0)
            }
            Opcode::AddLocalFromStack(a) => {
                pack_instr(OP_ADD_LOCAL_FROM_STACK, *a as u8, 0, 0)
            }
            Opcode::GetInstanceField(a) => {
                pack_instr(OP_GET_INSTANCE_FIELD, 0, 0, *a as u32)
            }
            Opcode::SetInstanceField(a) => {
                pack_instr(OP_SET_INSTANCE_FIELD, 0, 0, *a as u32)
            }
            Opcode::InplaceAddStrConst(a) => {
                pack_instr(OP_INPLACE_ADD_STR_CONST, *a as u8, self.operand as u16, 0)
            }
            Opcode::PopJumpIfSubscriptEqLocal(a, b, c, target) => {
                let bc = (*b as u16) | ((*c as u16) << 8);
                pack_instr(OP_POP_JUMP_IF_SUBSCRIPT_EQ_LOCAL, *a as u8, bc, *target as u32)
            }
            Opcode::PopJumpIfSubscriptNeLocal(a, b, c, target) => {
                let bc = (*b as u16) | ((*c as u16) << 8);
                pack_instr(OP_POP_JUMP_IF_SUBSCRIPT_NE_LOCAL, *a as u8, bc, *target as u32)
            }
        }
    }
}
