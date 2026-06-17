use arabi_core::span::Span;

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Assign {
        target: Expr,
        value: Expr,
    },
    MultiAssign {
        targets: Vec<(String, bool)>, // (name, is_star)
        value: Expr,
    },
    AugAssign {
        target: Expr,
        op: AugOp,
        value: Expr,
    },
    If {
        condition: Expr,
        body: Block,
        elifs: Vec<(Expr, Block)>,
        else_body: Option<Block>,
    },
    While {
        condition: Expr,
        body: Block,
        else_body: Option<Block>,
    },
    For {
        target: Expr,
        iterable: Expr,
        body: Block,
        else_body: Option<Block>,
    },
    Break,
    Continue,
    Pass,
    Return(Option<Expr>),
    Delete(Expr),
    Assert {
        condition: Expr,
        message: Option<Expr>,
    },
    Global(Vec<String>),
    Nonlocal(Vec<String>),
    FunctionDef {
        name: String,
        params: Vec<Param>,
        body: Block,
    },
    ClassDef {
        name: String,
        bases: Vec<Expr>,
        body: Block,
    },
    Import {
        module: String,
        alias: Option<String>,
    },
    ImportFrom {
        module: String,
        names: Vec<(String, Option<String>)>,
    },
    Try {
        body: Block,
        excepts: Vec<ExceptClause>,
        else_body: Option<Block>,
        finally_body: Option<Block>,
    },
    Raise(Option<Expr>),
    Decorator {
        decorators: Vec<Expr>,
        definition: Box<Stmt>,
    },
    Yield(Option<Expr>),
    YieldFrom(Expr),
    With {
        context: Expr,
        target: Option<String>,
        body: Block,
    },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    String(String),
    FString(String),
    Boolean(bool),
    Null,
    Super,
    Identifier(String),
    BinaryOp {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
    },
    IfExpr {
        condition: Box<Expr>,
        true_expr: Box<Expr>,
        false_expr: Box<Expr>,
    },
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Set(Vec<Expr>),
    Call {
        function: Box<Expr>,
        args: Vec<Expr>,
        kwargs: Vec<(String, Expr)>,
        unpack_args: Vec<Expr>,
        unpack_kwargs: Vec<Expr>,
    },
    Attribute {
        object: Box<Expr>,
        name: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    Slice {
        object: Box<Expr>,
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        step: Option<Box<Expr>>,
    },
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
    },
    ListComp {
        expr: Box<Expr>,
        iter: Box<Expr>,
        target: String,
        condition: Option<Box<Expr>>,
    },
    DictComp {
        key: Box<Expr>,
        value: Box<Expr>,
        iter: Box<Expr>,
        target: String,
        condition: Option<Box<Expr>>,
    },
    SetComp {
        expr: Box<Expr>,
        iter: Box<Expr>,
        target: String,
        condition: Option<Box<Expr>>,
    },
    YieldExpr(Option<Box<Expr>>),
    WalrusExpr {
        name: String,
        value: Box<Expr>,
    },
}

#[derive(Debug, Clone)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    Eq,
    NotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
    In,
    NotIn,
    Is,
    IsNot,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Neg,
    Not,
    BitNot,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AugOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub default: Option<Expr>,
    pub is_varargs: bool,
    pub is_kwargs: bool,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ExceptClause {
    pub type_name: Option<String>,
    pub name: Option<String>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
    pub stmt_lines: Vec<usize>,
}
