//! AST types for code understanding
//!
//! This module provides Abstract Syntax Tree representations for Rust code.

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use super::token::Span;

/// AST Node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

impl NodeId {
    /// Create new node ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Private (default)
    Private,
    /// `pub`
    Public,
    /// `pub(crate)`
    Crate,
    /// `pub(super)`
    Super,
    /// `pub(in path)`
    Restricted,
}

/// Mutability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mutability {
    /// Immutable
    Immutable,
    /// Mutable
    Mutable,
}

/// Type reference
#[derive(Debug, Clone)]
pub enum TypeRef {
    /// Named type
    Named {
        path: Vec<String>,
        generics: Vec<TypeRef>,
    },
    /// Reference type
    Reference {
        mutability: Mutability,
        inner: Box<TypeRef>,
        lifetime: Option<String>,
    },
    /// Pointer type
    Pointer {
        mutability: Mutability,
        inner: Box<TypeRef>,
    },
    /// Array type
    Array {
        inner: Box<TypeRef>,
        size: Option<u64>,
    },
    /// Slice type
    Slice { inner: Box<TypeRef> },
    /// Tuple type
    Tuple { elements: Vec<TypeRef> },
    /// Function type
    Function {
        params: Vec<TypeRef>,
        ret: Option<Box<TypeRef>>,
    },
    /// Never type (!)
    Never,
    /// Inferred type (_)
    Inferred,
    /// Self type
    SelfType,
    /// Unit type ()
    Unit,
}

/// Pattern
#[derive(Debug, Clone)]
pub enum Pattern {
    /// Identifier pattern
    Ident {
        name: String,
        mutability: Mutability,
        binding: Option<Box<Pattern>>,
    },
    /// Wildcard pattern (_)
    Wildcard,
    /// Literal pattern
    Literal { value: String },
    /// Tuple pattern
    Tuple { elements: Vec<Pattern> },
    /// Struct pattern
    Struct {
        path: Vec<String>,
        fields: Vec<(String, Pattern)>,
    },
    /// Reference pattern
    Ref {
        mutability: Mutability,
        inner: Box<Pattern>,
    },
    /// Or pattern (|)
    Or { patterns: Vec<Pattern> },
    /// Range pattern
    Range {
        start: Option<Box<Pattern>>,
        end: Option<Box<Pattern>>,
        inclusive: bool,
    },
}

/// Expression
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal
    Literal { kind: LiteralKind, value: String },
    /// Path expression
    Path { segments: Vec<String> },
    /// Binary expression
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// Unary expression
    Unary { op: UnaryOp, operand: Box<Expr> },
    /// Block expression
    Block {
        stmts: Vec<Stmt>,
        expr: Option<Box<Expr>>,
    },
    /// If expression
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },
    /// Match expression
    Match {
        expr: Box<Expr>,
        arms: Vec<MatchArm>,
    },
    /// Loop expression
    Loop {
        body: Box<Expr>,
        label: Option<String>,
    },
    /// While expression
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
        label: Option<String>,
    },
    /// For expression
    For {
        pattern: Pattern,
        iter: Box<Expr>,
        body: Box<Expr>,
        label: Option<String>,
    },
    /// Call expression
    Call { func: Box<Expr>, args: Vec<Expr> },
    /// Method call
    MethodCall {
        receiver: Box<Expr>,
        method: String,
        args: Vec<Expr>,
    },
    /// Field access
    Field { expr: Box<Expr>, field: String },
    /// Index expression
    Index { expr: Box<Expr>, index: Box<Expr> },
    /// Reference expression
    Ref {
        mutability: Mutability,
        expr: Box<Expr>,
    },
    /// Dereference expression
    Deref { expr: Box<Expr> },
    /// Cast expression
    Cast { expr: Box<Expr>, ty: TypeRef },
    /// Tuple expression
    Tuple { elements: Vec<Expr> },
    /// Array expression
    Array { elements: Vec<Expr> },
    /// Struct expression
    Struct {
        path: Vec<String>,
        fields: Vec<(String, Expr)>,
        base: Option<Box<Expr>>,
    },
    /// Closure expression
    Closure {
        params: Vec<(Pattern, Option<TypeRef>)>,
        body: Box<Expr>,
        is_move: bool,
    },
    /// Return expression
    Return { expr: Option<Box<Expr>> },
    /// Break expression
    Break {
        label: Option<String>,
        expr: Option<Box<Expr>>,
    },
    /// Continue expression
    Continue { label: Option<String> },
    /// Try expression (?)
    Try { expr: Box<Expr> },
    /// Await expression
    Await { expr: Box<Expr> },
    /// Unsafe block
    Unsafe { body: Box<Expr> },
    /// Async block
    Async { body: Box<Expr> },
}

/// Literal kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralKind {
    /// Integer
    Integer,
    /// Float
    Float,
    /// String
    String,
    /// Char
    Char,
    /// Boolean
    Bool,
}

/// Binary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// +
    Add,
    /// -
    Sub,
    /// *
    Mul,
    /// /
    Div,
    /// %
    Rem,
    /// &
    BitAnd,
    /// |
    BitOr,
    /// ^
    BitXor,
    /// <<
    Shl,
    /// >>
    Shr,
    /// ==
    Eq,
    /// !=
    Ne,
    /// <
    Lt,
    /// <=
    Le,
    /// >
    Gt,
    /// >=
    Ge,
    /// &&
    And,
    /// ||
    Or,
    /// =
    Assign,
    /// +=
    AddAssign,
    /// -=
    SubAssign,
    /// *=
    MulAssign,
    /// /=
    DivAssign,
    /// %=
    RemAssign,
    /// &=
    BitAndAssign,
    /// |=
    BitOrAssign,
    /// ^=
    BitXorAssign,
    /// <<=
    ShlAssign,
    /// >>=
    ShrAssign,
}

/// Unary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// -
    Neg,
    /// !
    Not,
    /// *
    Deref,
    /// &
    Ref,
    /// &mut
    RefMut,
}

/// Match arm
#[derive(Debug, Clone)]
pub struct MatchArm {
    /// Pattern
    pub pattern: Pattern,
    /// Guard
    pub guard: Option<Expr>,
    /// Body
    pub body: Expr,
}

/// Statement
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Let statement
    Let {
        pattern: Pattern,
        ty: Option<TypeRef>,
        init: Option<Expr>,
    },
    /// Expression statement
    Expr { expr: Expr, has_semi: bool },
    /// Item statement
    Item { item: Box<Item> },
}

/// Function parameter
#[derive(Debug, Clone)]
pub struct FnParam {
    /// Pattern
    pub pattern: Pattern,
    /// Type
    pub ty: TypeRef,
}

/// Function signature
#[derive(Debug, Clone)]
pub struct FnSig {
    /// Name
    pub name: String,
    /// Generic parameters
    pub generics: Vec<GenericParam>,
    /// Parameters
    pub params: Vec<FnParam>,
    /// Return type
    pub return_type: Option<TypeRef>,
    /// Where clause
    pub where_clause: Vec<WherePredicate>,
    /// Is async
    pub is_async: bool,
    /// Is unsafe
    pub is_unsafe: bool,
    /// Is const
    pub is_const: bool,
    /// ABI
    pub abi: Option<String>,
}

/// Generic parameter
#[derive(Debug, Clone)]
pub enum GenericParam {
    /// Type parameter
    Type {
        name: String,
        bounds: Vec<TypeRef>,
        default: Option<TypeRef>,
    },
    /// Lifetime parameter
    Lifetime { name: String, bounds: Vec<String> },
    /// Const parameter
    Const { name: String, ty: TypeRef },
}

/// Where predicate
#[derive(Debug, Clone)]
pub enum WherePredicate {
    /// Type bound
    TypeBound { ty: TypeRef, bounds: Vec<TypeRef> },
    /// Lifetime bound
    LifetimeBound {
        lifetime: String,
        bounds: Vec<String>,
    },
}

/// Item
#[derive(Debug, Clone)]
pub enum Item {
    /// Function
    Function {
        vis: Visibility,
        sig: FnSig,
        body: Option<Expr>,
    },
    /// Struct
    Struct {
        vis: Visibility,
        name: String,
        generics: Vec<GenericParam>,
        fields: StructFields,
    },
    /// Enum
    Enum {
        vis: Visibility,
        name: String,
        generics: Vec<GenericParam>,
        variants: Vec<EnumVariant>,
    },
    /// Impl block
    Impl {
        generics: Vec<GenericParam>,
        trait_ref: Option<TypeRef>,
        self_ty: TypeRef,
        items: Vec<Item>,
    },
    /// Trait
    Trait {
        vis: Visibility,
        name: String,
        generics: Vec<GenericParam>,
        bounds: Vec<TypeRef>,
        items: Vec<Item>,
    },
    /// Type alias
    TypeAlias {
        vis: Visibility,
        name: String,
        generics: Vec<GenericParam>,
        ty: TypeRef,
    },
    /// Const
    Const {
        vis: Visibility,
        name: String,
        ty: TypeRef,
        value: Expr,
    },
    /// Static
    Static {
        vis: Visibility,
        name: String,
        mutability: Mutability,
        ty: TypeRef,
        value: Expr,
    },
    /// Module
    Module {
        vis: Visibility,
        name: String,
        items: Vec<Item>,
    },
    /// Use
    Use { vis: Visibility, tree: UseTree },
    /// Extern block
    ExternBlock {
        abi: Option<String>,
        items: Vec<Item>,
    },
}

/// Struct fields
#[derive(Debug, Clone)]
pub enum StructFields {
    /// Named fields
    Named(Vec<StructField>),
    /// Tuple fields
    Tuple(Vec<TypeRef>),
    /// Unit struct
    Unit,
}

/// Struct field
#[derive(Debug, Clone)]
pub struct StructField {
    /// Visibility
    pub vis: Visibility,
    /// Name
    pub name: String,
    /// Type
    pub ty: TypeRef,
}

/// Enum variant
#[derive(Debug, Clone)]
pub struct EnumVariant {
    /// Name
    pub name: String,
    /// Fields
    pub fields: StructFields,
    /// Discriminant
    pub discriminant: Option<Expr>,
}

/// Use tree
#[derive(Debug, Clone)]
pub enum UseTree {
    /// Simple path
    Path {
        path: Vec<String>,
        alias: Option<String>,
    },
    /// Glob (*)
    Glob { path: Vec<String> },
    /// Nested
    Nested {
        path: Vec<String>,
        trees: Vec<UseTree>,
    },
}

/// AST Node with metadata
#[derive(Debug, Clone)]
pub struct AstNode {
    /// Node ID
    pub id: NodeId,
    /// Span
    pub span: Span,
    /// Item
    pub item: Item,
    /// Attributes
    pub attrs: Vec<Attribute>,
}

/// Attribute
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Is inner attribute (#![...])
    pub is_inner: bool,
    /// Path
    pub path: Vec<String>,
    /// Arguments
    pub args: Option<String>,
}
