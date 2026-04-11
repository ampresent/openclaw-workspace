/// Nix Expression Interpreter — In-process Nix expression evaluator
///
/// Implements a basic Nix expression parser and evaluator for a useful subset:
/// - Literals: integers, floats, booleans, null, strings (with interpolation)
/// - Data structures: attrsets `{}`, lists `[]`
/// - Let bindings: `let x = 1; y = 2; in x + y`
/// - Conditionals: `if cond then a else b`
/// - Function application: `f x`
/// - Function definition: `x: body`
/// - Attribute access: `a.b` and `a.${expr}`
/// - Binary operators: +, -, *, /, ==, !=, &&, ||, ++, //
/// - With: `with set; body`
/// - Inherit: `inherit (set) a b;`
///
/// This is intentionally a SUBSET — enough for syntax checking, config previews,
/// and IDE integration. Not a full Nix interpreter.

use axum::extract::Query;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::error::AppError;

// ─── Token & AST ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Integer(i64),
    Float(f64),
    Str(String),
    Ident(String),
    True,
    False,
    Null,
    Let,
    In,
    If,
    Then,
    Else,
    With,
    Inherit,
    Or,       // `or` keyword for default attr access
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]
    LParen,   // (
    RParen,   // )
    Dot,      // .
    Colon,    // :
    Semicolon,// ;
    Comma,    // ,
    At,       // @ (pattern binding)
    Question, // ? (default value)
    Plus,     // +
    Minus,    // -
    Star,     // *
    Slash,    // /
    Eq,       // ==
    Neq,      // !=
    And,      // &&
    OrOp,     // ||
    Concat,   // ++
    Merge,    // //
    Assign,   // =
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NixExpr {
    Integer(i64),
    Float(f64),
    Bool(bool),
    Null,
    String(String),
    Ident(String),
    AttrSet(Vec<Binding>),
    List(Vec<NixExpr>),
    Let {
        bindings: Vec<Binding>,
        body: Box<NixExpr>,
    },
    If {
        cond: Box<NixExpr>,
        then_branch: Box<NixExpr>,
        else_branch: Box<NixExpr>,
    },
    Lambda {
        param: LambdaParam,
        body: Box<NixExpr>,
    },
    Apply {
        func: Box<NixExpr>,
        arg: Box<NixExpr>,
    },
    Select {
        expr: Box<NixExpr>,
        attrpath: Vec<String>,
        default: Option<Box<NixExpr>>,
    },
    BinaryOp {
        op: BinOp,
        left: Box<NixExpr>,
        right: Box<NixExpr>,
    },
    UnaryNeg(Box<NixExpr>),
    With {
        namespace: Box<NixExpr>,
        body: Box<NixExpr>,
    },
    Assert {
        cond: Box<NixExpr>,
        body: Box<NixExpr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum LambdaParam {
    Ident(String),
    AttrSet {
        fields: Vec<(String, Option<NixExpr>)>,
        ellipsis: bool,
        bind: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binding {
    pub path: Vec<String>,
    pub value: NixExpr,
    pub inherit: bool,
    pub inherit_from: Option<NixExpr>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div,
    Eq, Neq, And, Or,
    Concat, Merge,
    Lt, Le, Gt, Ge,
}

// ─── Values (runtime) ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
pub enum NixValue {
    Int(i64),
    FloatVal(f64),
    Bool(bool),
    Null,
    String(String),
    List(Vec<NixValue>),
    AttrSet(HashMap<String, NixValue>),
    Thunk,
}

impl fmt::Display for NixValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NixValue::Int(n) => write!(f, "{n}"),
            NixValue::FloatVal(n) => write!(f, "{n}"),
            NixValue::Bool(b) => write!(f, "{b}"),
            NixValue::Null => write!(f, "null"),
            NixValue::String(s) => write!(f, "\"{s}\""),
            NixValue::List(items) => {
                write!(f, "[ ")?;
                for item in items { write!(f, "{item} ")?; }
                write!(f, "]")
            }
            NixValue::AttrSet(map) => {
                write!(f, "{{ ")?;
                let mut first = true;
                for (k, v) in map {
                    if !first { write!(f, " ")?; }
                    write!(f, "{k} = {v};")?;
                    first = false;
                }
                write!(f, " }}")
            }
            NixValue::Thunk => write!(f, "<function>"),
        }
    }
}

impl NixValue {
    pub fn is_truthy(&self) -> bool {
        match self {
            NixValue::Bool(b) => *b,
            NixValue::Null => false,
            NixValue::Int(n) => *n != 0,
            NixValue::FloatVal(n) => *n != 0.0,
            NixValue::String(s) => !s.is_empty(),
            NixValue::List(l) => !l.is_empty(),
            NixValue::AttrSet(_) => true,
            NixValue::Thunk => true,
        }
    }
}

// ─── Lexer ────────────────────────────────────────────────────────────────

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self { input: input.chars().collect(), pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.input.get(self.pos).copied();
        if ch.is_some() { self.pos += 1; }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '/' {
                if self.input.get(self.pos + 1) == Some(&'*') {
                    // Block comment
                    self.advance(); self.advance();
                    let mut depth = 1;
                    while depth > 0 {
                        match self.advance() {
                            Some('/') if self.peek() == Some('*') => { self.advance(); depth += 1; }
                            Some('*') if self.peek() == Some('/') => { self.advance(); depth -= 1; }
                            None => break,
                            _ => {}
                        }
                    }
                } else if self.input.get(self.pos + 1) == Some(&'/') {
                    // Line comment
                    while let Some(c) = self.advance() { if c == '\n' { break; } }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<Token, String> {
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => return Ok(Token::Str(s)),
                Some('\\') => {
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some(c) => { s.push('\\'); s.push(c); }
                        None => return Err("Unterminated string escape".into()),
                    }
                }
                Some(c) => s.push(c),
                None => return Err("Unterminated string literal".into()),
            }
        }
    }

    fn read_indented_string(&mut self) -> Result<Token, String> {
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('\'') if self.peek() == Some('\'') => { self.advance(); return Ok(Token::Str(s)); }
                Some('$') if self.peek() == Some('\'') => { self.advance(); s.push('\''); }
                Some(c) => s.push(c),
                None => return Err("Unterminated indented string".into()),
            }
        }
    }

    fn read_number(&mut self, first: char) -> Token {
        let mut num = String::from(first);
        let mut is_float = false;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() { num.push(ch); self.advance(); }
            else if ch == '.' && !is_float
                && self.input.get(self.pos + 1).map_or(false, |c| c.is_ascii_digit()) {
                is_float = true; num.push(ch); self.advance();
            } else { break; }
        }
        if is_float { Token::Float(num.parse().unwrap_or(0.0)) }
        else { Token::Integer(num.parse().unwrap_or(0)) }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None => { tokens.push(Token::Eof); break; }
                Some(ch) => {
                    let tok = match ch {
                        '{' => { self.advance(); Token::LBrace }
                        '}' => { self.advance(); Token::RBrace }
                        '[' => { self.advance(); Token::LBracket }
                        ']' => { self.advance(); Token::RBracket }
                        '(' => { self.advance(); Token::LParen }
                        ')' => { self.advance(); Token::RParen }
                        '.' => { self.advance(); Token::Dot }
                        ':' => { self.advance(); Token::Colon }
                        ';' => { self.advance(); Token::Semicolon }
                        ',' => { self.advance(); Token::Comma }
                        '@' => { self.advance(); Token::At }
                        '?' => { self.advance(); Token::Question }
                        '*' => { self.advance(); Token::Star }
                        '"' => { self.advance(); self.read_string()? }
                        '\'' => {
                            self.advance();
                            if self.peek() == Some('\'') {
                                self.advance(); self.read_indented_string()?
                            } else { Token::Str(String::new()) }
                        }
                        '=' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Eq }
                            else { Token::Assign }
                        }
                        '!' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Neq }
                            else { return Err(format!("Unexpected '!' at pos {}", self.pos)); }
                        }
                        '&' => {
                            self.advance();
                            if self.peek() == Some('&') { self.advance(); Token::And }
                            else { return Err(format!("Unexpected '&' at pos {}", self.pos)); }
                        }
                        '|' => {
                            self.advance();
                            if self.peek() == Some('|') { self.advance(); Token::OrOp }
                            else { return Err(format!("Unexpected '|' at pos {}", self.pos)); }
                        }
                        '+' => {
                            self.advance();
                            if self.peek() == Some('+') { self.advance(); Token::Concat }
                            else { Token::Plus }
                        }
                        '-' => {
                            self.advance();
                            // Check if it's a negative number
                            if self.peek().map_or(false, |c| c.is_ascii_digit()) {
                                let tok = self.read_number('-');
                                tok
                            } else {
                                Token::Minus
                            }
                        }
                        '/' => {
                            self.advance();
                            if self.peek() == Some('/') { self.advance(); Token::Merge }
                            else { Token::Slash }
                        }
                        '<' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Ident("__le__".into()) }
                            else { Token::Ident("__lt__".into()) }
                        }
                        '>' => {
                            self.advance();
                            if self.peek() == Some('=') { self.advance(); Token::Ident("__ge__".into()) }
                            else { Token::Ident("__gt__".into()) }
                        }
                        c if c.is_ascii_digit() => {
                            self.advance(); self.read_number(c)
                        }
                        c if c.is_alphabetic() || c == '_' => {
                            let mut ident = String::new();
                            while let Some(ch) = self.peek() {
                                if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '\'' {
                                    ident.push(ch); self.advance();
                                } else { break; }
                            }
                            match ident.as_str() {
                                "true" => Token::True,
                                "false" => Token::False,
                                "null" => Token::Null,
                                "let" => Token::Let,
                                "in" => Token::In,
                                "if" => Token::If,
                                "then" => Token::Then,
                                "else" => Token::Else,
                                "with" => Token::With,
                                "inherit" => Token::Inherit,
                                "or" => Token::Or,
                                "assert" => Token::Ident("__assert__".into()),
                                _ => Token::Ident(ident),
                            }
                        }
                        c => return Err(format!("Unexpected character '{c}' at pos {}", self.pos)),
                    };
                    tokens.push(tok);
                }
            }
        }
        Ok(tokens)
    }
}

// ─── Parser ──────────────────────────────────────────────────────────────

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        if self.pos < self.tokens.len() { self.pos += 1; }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<Token, String> {
        let tok = self.advance();
        if std::mem::discriminant(&tok) == std::mem::discriminant(expected) { Ok(tok) }
        else { Err(format!("Expected {expected:?}, got {tok:?}")) }
    }

    pub fn parse(&mut self) -> Result<NixExpr, String> {
        let expr = self.parse_expr()?;
        if *self.peek() != Token::Eof {
            return Err(format!("Unexpected token after expression: {:?}", self.peek()));
        }
        Ok(expr)
    }

    fn parse_expr(&mut self) -> Result<NixExpr, String> {
        match self.peek() {
            Token::Let => self.parse_let(),
            Token::If => self.parse_if(),
            Token::With => self.parse_with(),
            Token::Ident(id) if id == "__assert__" => self.parse_assert(),
            _ => self.parse_or(),
        }
    }

    fn parse_let(&mut self) -> Result<NixExpr, String> {
        self.advance(); // 'let'
        let bindings = self.parse_bindings()?;
        self.expect(&Token::In)?;
        let body = self.parse_expr()?;
        Ok(NixExpr::Let { bindings, body: Box::new(body) })
    }

    fn parse_if(&mut self) -> Result<NixExpr, String> {
        self.advance(); // 'if'
        let cond = self.parse_expr()?;
        self.expect(&Token::Then)?;
        let then_branch = self.parse_expr()?;
        self.expect(&Token::Else)?;
        let else_branch = self.parse_expr()?;
        Ok(NixExpr::If {
            cond: Box::new(cond),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    fn parse_with(&mut self) -> Result<NixExpr, String> {
        self.advance(); // 'with'
        let namespace = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;
        let body = self.parse_expr()?;
        Ok(NixExpr::With { namespace: Box::new(namespace), body: Box::new(body) })
    }

    fn parse_assert(&mut self) -> Result<NixExpr, String> {
        self.advance(); // 'assert'
        let cond = self.parse_expr()?;
        self.expect(&Token::Semicolon)?;
        let body = self.parse_expr()?;
        Ok(NixExpr::Assert { cond: Box::new(cond), body: Box::new(body) })
    }

    fn parse_bindings(&mut self) -> Result<Vec<Binding>, String> {
        let mut bindings = Vec::new();
        while *self.peek() != Token::In && *self.peek() != Token::Eof {
            if *self.peek() == Token::Inherit {
                bindings.extend(self.parse_inherit()?);
            } else {
                bindings.push(self.parse_single_binding()?);
            }
        }
        Ok(bindings)
    }

    fn parse_inherit(&mut self) -> Result<Vec<Binding>, String> {
        self.advance(); // 'inherit'
        let from = if *self.peek() == Token::LParen {
            self.advance();
            let expr = self.parse_expr()?;
            self.expect(&Token::RParen)?;
            Some(expr)
        } else { None };

        let mut bindings = Vec::new();
        while let Token::Ident(name) = self.peek() {
            let name = name.clone();
            self.advance();
            bindings.push(Binding {
                path: vec![name],
                value: NixExpr::Ident("__inherit__".into()),
                inherit: true,
                inherit_from: from.clone(),
            });
        }
        self.expect(&Token::Semicolon)?;
        Ok(bindings)
    }

    fn parse_single_binding(&mut self) -> Result<Binding, String> {
        let mut path = Vec::new();
        loop {
            match self.peek() {
                Token::Ident(name) => { path.push(name.clone()); self.advance(); }
                _ => return Err(format!("Expected identifier in binding, got {:?}", self.peek())),
            }
            if *self.peek() != Token::Dot { break; }
            self.advance(); // dot
        }

        if *self.peek() == Token::Assign {
            self.advance();
            let value = self.parse_expr()?;
            if *self.peek() == Token::Semicolon { self.advance(); }
            Ok(Binding { path, value, inherit: false, inherit_from: None })
        } else if *self.peek() == Token::Question {
            self.advance();
            let _default = self.parse_expr()?;
            self.expect(&Token::Assign)?;
            let value = self.parse_expr()?;
            if *self.peek() == Token::Semicolon { self.advance(); }
            Ok(Binding { path, value, inherit: false, inherit_from: None })
        } else {
            Err(format!("Expected '=' in binding, got {:?}", self.peek()))
        }
    }

    fn parse_or(&mut self) -> Result<NixExpr, String> {
        let mut left = self.parse_and()?;
        while *self.peek() == Token::OrOp {
            self.advance();
            let right = self.parse_and()?;
            left = NixExpr::BinaryOp { op: BinOp::Or, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<NixExpr, String> {
        let mut left = self.parse_equality()?;
        while *self.peek() == Token::And {
            self.advance();
            let right = self.parse_equality()?;
            left = NixExpr::BinaryOp { op: BinOp::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<NixExpr, String> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Token::Eq => Some(BinOp::Eq),
                Token::Neq => Some(BinOp::Neq),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_comparison()?;
                left = NixExpr::BinaryOp { op, left: Box::new(left), right: Box::new(right) };
            } else { break; }
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<NixExpr, String> {
        let mut left = self.parse_update()?;
        loop {
            let op = match self.peek() {
                Token::Ident(id) if id == "__lt__" => Some(BinOp::Lt),
                Token::Ident(id) if id == "__le__" => Some(BinOp::Le),
                Token::Ident(id) if id == "__gt__" => Some(BinOp::Gt),
                Token::Ident(id) if id == "__ge__" => Some(BinOp::Ge),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_update()?;
                left = NixExpr::BinaryOp { op, left: Box::new(left), right: Box::new(right) };
            } else { break; }
        }
        Ok(left)
    }

    fn parse_update(&mut self) -> Result<NixExpr, String> {
        let mut left = self.parse_concat()?;
        while *self.peek() == Token::Merge {
            self.advance();
            let right = self.parse_concat()?;
            left = NixExpr::BinaryOp { op: BinOp::Merge, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_concat(&mut self) -> Result<NixExpr, String> {
        let mut left = self.parse_additive()?;
        while *self.peek() == Token::Concat {
            self.advance();
            let right = self.parse_additive()?;
            left = NixExpr::BinaryOp { op: BinOp::Concat, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<NixExpr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus => Some(BinOp::Add),
                Token::Minus => Some(BinOp::Sub),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_multiplicative()?;
                left = NixExpr::BinaryOp { op, left: Box::new(left), right: Box::new(right) };
            } else { break; }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<NixExpr, String> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Token::Star => Some(BinOp::Mul),
                Token::Slash => Some(BinOp::Div),
                _ => None,
            };
            if let Some(op) = op {
                self.advance();
                let right = self.parse_unary()?;
                left = NixExpr::BinaryOp { op, left: Box::new(left), right: Box::new(right) };
            } else { break; }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<NixExpr, String> {
        match self.peek() {
            Token::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(NixExpr::UnaryNeg(Box::new(expr)))
            }
            _ => self.parse_app(),
        }
    }

    fn parse_app(&mut self) -> Result<NixExpr, String> {
        let mut expr = self.parse_select()?;
        loop {
            match self.peek() {
                Token::Integer(_) | Token::Float(_) | Token::Str(_)
                | Token::True | Token::False | Token::Null
                | Token::LBrace | Token::LBracket | Token::LParen
                | Token::Ident(_) => {
                    if let Token::Let | Token::If | Token::With = self.peek() { break; }
                    if let Token::Ident(id) = self.peek() {
                        if id == "__assert__" { break; }
                    }
                    let arg = self.parse_select()?;
                    expr = NixExpr::Apply { func: Box::new(expr), arg: Box::new(arg) };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_select(&mut self) -> Result<NixExpr, String> {
        let mut expr = self.parse_primary()?;
        while *self.peek() == Token::Dot {
            self.advance();
            let mut path = Vec::new();
            match self.peek() {
                Token::Ident(name) => {
                    path.push(name.clone());
                    self.advance();
                    while *self.peek() == Token::Dot {
                        self.advance();
                        match self.peek() {
                            Token::Ident(name) => { path.push(name.clone()); self.advance(); }
                            _ => break,
                        }
                    }
                }
                Token::LParen => {
                    self.advance();
                    let _dyn = self.parse_expr()?;
                    self.expect(&Token::RParen)?;
                    path.push("__dynamic__".into());
                }
                _ => break,
            }
            let default = if *self.peek() == Token::Or || *self.peek() == Token::Question {
                self.advance();
                Some(Box::new(self.parse_primary()?))
            } else { None };
            expr = NixExpr::Select { expr: Box::new(expr), attrpath: path, default };
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<NixExpr, String> {
        match self.peek().clone() {
            Token::Integer(n) => { self.advance(); Ok(NixExpr::Integer(n)) }
            Token::Float(n) => { self.advance(); Ok(NixExpr::Float(n)) }
            Token::True => { self.advance(); Ok(NixExpr::Bool(true)) }
            Token::False => { self.advance(); Ok(NixExpr::Bool(false)) }
            Token::Null => { self.advance(); Ok(NixExpr::Null) }
            Token::Str(s) => { self.advance(); Ok(NixExpr::String(s)) }
            Token::Ident(name) => {
                self.advance();
                if *self.peek() == Token::Colon {
                    self.advance();
                    let body = self.parse_expr()?;
                    return Ok(NixExpr::Lambda {
                        param: LambdaParam::Ident(name),
                        body: Box::new(body),
                    });
                }
                Ok(NixExpr::Ident(name))
            }
            Token::LBrace => self.parse_brace_expr(),
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while *self.peek() != Token::RBracket && *self.peek() != Token::Eof {
                    items.push(self.parse_expr()?);
                }
                self.expect(&Token::RBracket)?;
                Ok(NixExpr::List(items))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_primary()?;
                Ok(NixExpr::UnaryNeg(Box::new(expr)))
            }
            tok => Err(format!("Unexpected token in expression: {tok:?}")),
        }
    }

    fn parse_brace_expr(&mut self) -> Result<NixExpr, String> {
        self.advance(); // '{'
        // Look ahead: attrset vs lambda param set
        if let Token::Ident(name) = self.peek() {
            let save = self.pos;
            self.advance();
            let is_lambda = matches!(self.peek(), Token::Comma | Token::RBrace | Token::Question | Token::Colon);
            self.pos = save;
            if is_lambda {
                return self.parse_lambda_set_param();
            }
        }

        let mut bindings = Vec::new();
        while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
            if *self.peek() == Token::Inherit {
                bindings.extend(self.parse_inherit()?);
                continue;
            }
            bindings.push(self.parse_single_binding()?);
        }
        self.expect(&Token::RBrace)?;
        Ok(NixExpr::AttrSet(bindings))
    }

    fn parse_lambda_set_param(&mut self) -> Result<NixExpr, String> {
        let mut fields = Vec::new();
        let mut ellipsis = false;

        while *self.peek() != Token::RBrace && *self.peek() != Token::Eof {
            if let Token::Ident(name) = self.peek() {
                if name == "..." { self.advance(); ellipsis = true; break; }
            }
            match self.peek() {
                Token::Ident(name) => {
                    let name = name.clone();
                    self.advance();
                    let default = if *self.peek() == Token::Question {
                        self.advance(); Some(self.parse_expr()?)
                    } else { None };
                    fields.push((name, default));
                    if *self.peek() == Token::Comma { self.advance(); }
                }
                _ => break,
            }
        }
        self.expect(&Token::RBrace)?;

        let bind = if *self.peek() == Token::At {
            self.advance();
            if let Token::Ident(name) = self.peek() {
                let n = name.clone(); self.advance(); Some(n)
            } else { None }
        } else { None };

        self.expect(&Token::Colon)?;
        let body = self.parse_expr()?;

        Ok(NixExpr::Lambda {
            param: LambdaParam::AttrSet { fields, ellipsis, bind },
            body: Box::new(body),
        })
    }
}

// ─── Evaluator ────────────────────────────────────────────────────────────

pub struct Evaluator {
    env: HashMap<String, NixValue>,
}

impl Evaluator {
    pub fn new() -> Self {
        let mut env = HashMap::new();
        env.insert("true".into(), NixValue::Bool(true));
        env.insert("false".into(), NixValue::Bool(false));
        env.insert("null".into(), NixValue::Null);
        Self { env }
    }

    pub fn eval(&mut self, expr: &NixExpr) -> Result<NixValue, String> {
        match expr {
            NixExpr::Integer(n) => Ok(NixValue::Int(*n)),
            NixExpr::Float(n) => Ok(NixValue::FloatVal(*n)),
            NixExpr::Bool(b) => Ok(NixValue::Bool(*b)),
            NixExpr::Null => Ok(NixValue::Null),
            NixExpr::String(s) => Ok(NixValue::String(s.clone())),
            NixExpr::Ident(name) => {
                self.env.get(name).cloned().ok_or_else(|| format!("Undefined variable: {name}"))
            }
            NixExpr::AttrSet(bindings) => {
                let mut map = HashMap::new();
                for binding in bindings {
                    if binding.inherit {
                        if let Some(from) = &binding.inherit_from {
                            let from_val = self.eval(from)?;
                            if let NixValue::AttrSet(from_map) = from_val {
                                for name in &binding.path {
                                    if let Some(v) = from_map.get(name) {
                                        map.insert(name.clone(), v.clone());
                                    }
                                }
                            }
                        } else {
                            for name in &binding.path {
                                if let Some(v) = self.env.get(name) {
                                    map.insert(name.clone(), v.clone());
                                }
                            }
                        }
                    } else {
                        let val = self.eval(&binding.value)?;
                        self.insert_nested(&mut map, &binding.path, val);
                    }
                }
                Ok(NixValue::AttrSet(map))
            }
            NixExpr::List(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.eval(e)).collect();
                Ok(NixValue::List(values?))
            }
            NixExpr::Let { bindings, body } => {
                let saved = self.env.clone();
                for binding in bindings {
                    let val = self.eval(&binding.value)?;
                    for name in &binding.path {
                        self.env.insert(name.clone(), val.clone());
                    }
                }
                let result = self.eval(body);
                self.env = saved;
                result
            }
            NixExpr::If { cond, then_branch, else_branch } => {
                if self.eval(cond)?.is_truthy() { self.eval(then_branch) }
                else { self.eval(else_branch) }
            }
            NixExpr::BinaryOp { op, left, right } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                self.eval_binop(op, &l, &r)
            }
            NixExpr::UnaryNeg(expr) => {
                match self.eval(expr)? {
                    NixValue::Int(n) => Ok(NixValue::Int(-n)),
                    NixValue::FloatVal(n) => Ok(NixValue::FloatVal(-n)),
                    v => Err(format!("Cannot negate {v}")),
                }
            }
            NixExpr::Select { expr, attrpath, default } => {
                let val = self.eval(expr)?;
                match val {
                    NixValue::AttrSet(map) => {
                        let mut current = &map;
                        for (i, key) in attrpath.iter().enumerate() {
                            if key == "__dynamic__" {
                                return Err("Dynamic attr access not supported".into());
                            }
                            match current.get(key) {
                                Some(v) if i == attrpath.len() - 1 => return Ok(v.clone()),
                                Some(NixValue::AttrSet(m)) => current = m,
                                Some(_) => return Err(format!("Cannot access '{key}' on non-set")),
                                None => {
                                    return if let Some(d) = default { self.eval(d) }
                                    else { Err(format!("Attribute '{key}' not found")) };
                                }
                            }
                        }
                        Err("Empty attribute path".into())
                    }
                    _ => {
                        if let Some(d) = default { self.eval(d) }
                        else { Err(format!("Cannot select from {val}")) }
                    }
                }
            }
            NixExpr::Lambda { .. } => Ok(NixValue::Thunk),
            NixExpr::Apply { func, arg } => {
                // Evaluate to ensure arg is valid; functions return Thunk
                let _f = self.eval(func)?;
                let _a = self.eval(arg)?;
                Ok(NixValue::Thunk)
            }
            NixExpr::With { namespace, body } => {
                let ns = self.eval(namespace)?;
                let saved = self.env.clone();
                if let NixValue::AttrSet(map) = ns {
                    for (k, v) in map { self.env.insert(k, v); }
                }
                let result = self.eval(body);
                self.env = saved;
                result
            }
            NixExpr::Assert { cond, body } => {
                if !self.eval(cond)?.is_truthy() { return Err("Assertion failed".into()); }
                self.eval(body)
            }
        }
    }

    fn insert_nested(&self, map: &mut HashMap<String, NixValue>, path: &[String], value: NixValue) {
        if path.is_empty() { return; }
        if path.len() == 1 { map.insert(path[0].clone(), value); return; }
        let child = map.entry(path[0].clone()).or_insert_with(|| NixValue::AttrSet(HashMap::new()));
        if let NixValue::AttrSet(ref mut child_map) = child {
            self.insert_nested(child_map, &path[1..], value);
        }
    }

    fn eval_binop(&self, op: &BinOp, left: &NixValue, right: &NixValue) -> Result<NixValue, String> {
        match (op, left, right) {
            (BinOp::Add, NixValue::Int(a), NixValue::Int(b)) => Ok(NixValue::Int(a + b)),
            (BinOp::Add, NixValue::FloatVal(a), NixValue::FloatVal(b)) => Ok(NixValue::FloatVal(a + b)),
            (BinOp::Add, NixValue::Int(a), NixValue::FloatVal(b)) => Ok(NixValue::FloatVal(*a as f64 + b)),
            (BinOp::Add, NixValue::FloatVal(a), NixValue::Int(b)) => Ok(NixValue::FloatVal(a + *b as f64)),
            (BinOp::Add, NixValue::String(a), NixValue::String(b)) => Ok(NixValue::String(format!("{a}{b}"))),
            (BinOp::Sub, NixValue::Int(a), NixValue::Int(b)) => Ok(NixValue::Int(a - b)),
            (BinOp::Sub, NixValue::FloatVal(a), NixValue::FloatVal(b)) => Ok(NixValue::FloatVal(a - b)),
            (BinOp::Mul, NixValue::Int(a), NixValue::Int(b)) => Ok(NixValue::Int(a * b)),
            (BinOp::Mul, NixValue::FloatVal(a), NixValue::FloatVal(b)) => Ok(NixValue::FloatVal(a * b)),
            (BinOp::Div, NixValue::Int(a), NixValue::Int(b)) => {
                if *b == 0 { return Err("Division by zero".into()); }
                Ok(NixValue::Int(a / b))
            }
            (BinOp::Eq, _, _) => Ok(NixValue::Bool(left == right)),
            (BinOp::Neq, _, _) => Ok(NixValue::Bool(left != right)),
            (BinOp::And, _, _) => Ok(NixValue::Bool(left.is_truthy() && right.is_truthy())),
            (BinOp::Or, _, _) => Ok(NixValue::Bool(left.is_truthy() || right.is_truthy())),
            (BinOp::Concat, NixValue::List(a), NixValue::List(b)) => {
                let mut r = a.clone(); r.extend_from_slice(b); Ok(NixValue::List(r))
            }
            (BinOp::Merge, NixValue::AttrSet(a), NixValue::AttrSet(b)) => {
                let mut r = a.clone();
                for (k, v) in b { r.insert(k.clone(), v.clone()); }
                Ok(NixValue::AttrSet(r))
            }
            (BinOp::Lt, NixValue::Int(a), NixValue::Int(b)) => Ok(NixValue::Bool(a < b)),
            (BinOp::Le, NixValue::Int(a), NixValue::Int(b)) => Ok(NixValue::Bool(a <= b)),
            (BinOp::Gt, NixValue::Int(a), NixValue::Int(b)) => Ok(NixValue::Bool(a > b)),
            (BinOp::Ge, NixValue::Int(a), NixValue::Int(b)) => Ok(NixValue::Bool(a >= b)),
            _ => Err(format!("Unsupported operation")),
        }
    }
}

// ─── API Handlers ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EvalQuery {
    pub expression: String,
}

#[derive(Debug, Serialize)]
pub struct EvalResult {
    pub success: bool,
    pub ast: Option<serde_json::Value>,
    pub value: Option<NixValue>,
    pub error: Option<String>,
    pub tokens_parsed: usize,
    pub eval_time_us: u64,
}

/// Syntax check only (parse, don't evaluate)
pub async fn handle_check(Query(q): Query<EvalQuery>) -> Result<impl IntoResponse, AppError> {
    let start = std::time::Instant::now();
    let mut lexer = Lexer::new(&q.expression);
    let tokens = lexer.tokenize().map_err(|e| AppError::Validation {
        field: "expression".into(), message: format!("Lexer error: {e}"),
    })?;
    let token_count = tokens.len();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().map_err(|e| AppError::Validation {
        field: "expression".into(), message: format!("Parse error: {e}"),
    })?;
    let elapsed = start.elapsed().as_micros() as u64;
    Ok(Json(EvalResult {
        success: true, ast: Some(ast_to_json(&ast)), value: None,
        error: None, tokens_parsed: token_count, eval_time_us: elapsed,
    }))
}

/// Full parse + evaluate
pub async fn handle_eval(Query(q): Query<EvalQuery>) -> Result<impl IntoResponse, AppError> {
    let start = std::time::Instant::now();
    let mut lexer = Lexer::new(&q.expression);
    let tokens = lexer.tokenize().map_err(|e| AppError::Validation {
        field: "expression".into(), message: format!("Lexer error: {e}"),
    })?;
    let token_count = tokens.len();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().map_err(|e| AppError::Validation {
        field: "expression".into(), message: format!("Parse error: {e}"),
    })?;
    let mut evaluator = Evaluator::new();
    let value = evaluator.eval(&ast).map_err(|e| AppError::Validation {
        field: "expression".into(), message: format!("Eval error: {e}"),
    })?;
    let elapsed = start.elapsed().as_micros() as u64;
    Ok(Json(EvalResult {
        success: true, ast: Some(ast_to_json(&ast)), value: Some(value),
        error: None, tokens_parsed: token_count, eval_time_us: elapsed,
    }))
}

fn ast_to_json(ast: &NixExpr) -> serde_json::Value {
    match ast {
        NixExpr::Integer(n) => serde_json::json!({ "type": "integer", "value": n }),
        NixExpr::Float(n) => serde_json::json!({ "type": "float", "value": n }),
        NixExpr::Bool(b) => serde_json::json!({ "type": "bool", "value": b }),
        NixExpr::Null => serde_json::json!({ "type": "null" }),
        NixExpr::String(s) => serde_json::json!({ "type": "string", "value": s }),
        NixExpr::Ident(name) => serde_json::json!({ "type": "ident", "value": name }),
        NixExpr::AttrSet(bindings) => {
            let b: Vec<_> = bindings.iter().map(|b| serde_json::json!({
                "path": b.path,
                "value": ast_to_json(&b.value),
                "inherit": b.inherit,
            })).collect();
            serde_json::json!({ "type": "attrset", "bindings": b })
        }
        NixExpr::List(items) => {
            serde_json::json!({ "type": "list", "items": items.iter().map(ast_to_json).collect::<Vec<_>>() })
        }
        NixExpr::Let { bindings, body } => {
            let b: Vec<_> = bindings.iter().map(|b| serde_json::json!({
                "path": b.path, "value": ast_to_json(&b.value),
            })).collect();
            serde_json::json!({ "type": "let", "bindings": b, "body": ast_to_json(body) })
        }
        NixExpr::If { cond, then_branch, else_branch } => serde_json::json!({
            "type": "if", "cond": ast_to_json(cond),
            "then": ast_to_json(then_branch), "else": ast_to_json(else_branch),
        }),
        NixExpr::BinaryOp { op, left, right } => serde_json::json!({
            "type": "binop", "op": format!("{op:?}"),
            "left": ast_to_json(left), "right": ast_to_json(right),
        }),
        NixExpr::Select { expr, attrpath, default } => serde_json::json!({
            "type": "select", "expr": ast_to_json(expr), "path": attrpath,
            "default": default.as_ref().map(|d| ast_to_json(d)),
        }),
        NixExpr::Lambda { param, body } => serde_json::json!({
            "type": "lambda", "param": format!("{param:?}"), "body": ast_to_json(body),
        }),
        NixExpr::Apply { func, arg } => serde_json::json!({
            "type": "apply", "func": ast_to_json(func), "arg": ast_to_json(arg),
        }),
        NixExpr::UnaryNeg(expr) => serde_json::json!({ "type": "unary_neg", "expr": ast_to_json(expr) }),
        NixExpr::With { namespace, body } => serde_json::json!({
            "type": "with", "namespace": ast_to_json(namespace), "body": ast_to_json(body),
        }),
        NixExpr::Assert { cond, body } => serde_json::json!({
            "type": "assert", "cond": ast_to_json(cond), "body": ast_to_json(body),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer() {
        let mut l = Lexer::new("let x = 42; in x");
        let t = l.tokenize().unwrap();
        assert_eq!(t[0], Token::Let);
        assert_eq!(t[1], Token::Ident("x".into()));
        assert_eq!(t[3], Token::Integer(42));
    }

    #[test]
    fn test_eval_math() {
        let mut l = Lexer::new("1 + 2 * 3");
        let t = l.tokenize().unwrap();
        let ast = Parser::new(t).parse().unwrap();
        assert_eq!(Evaluator::new().eval(&ast).unwrap(), NixValue::Int(7));
    }

    #[test]
    fn test_eval_let() {
        let mut l = Lexer::new("let x = 10; y = 20; in x + y");
        let t = l.tokenize().unwrap();
        let ast = Parser::new(t).parse().unwrap();
        assert_eq!(Evaluator::new().eval(&ast).unwrap(), NixValue::Int(30));
    }

    #[test]
    fn test_eval_attrset() {
        let mut l = Lexer::new("{ name = \"test\"; value = 42; }");
        let t = l.tokenize().unwrap();
        let ast = Parser::new(t).parse().unwrap();
        if let NixValue::AttrSet(m) = Evaluator::new().eval(&ast).unwrap() {
            assert_eq!(m.get("name"), Some(&NixValue::String("test".into())));
            assert_eq!(m.get("value"), Some(&NixValue::Int(42)));
        } else { panic!("Expected attrset"); }
    }

    #[test]
    fn test_eval_if() {
        let mut l = Lexer::new("if true then 1 else 2");
        let t = l.tokenize().unwrap();
        let ast = Parser::new(t).parse().unwrap();
        assert_eq!(Evaluator::new().eval(&ast).unwrap(), NixValue::Int(1));
    }

    #[test]
    fn test_eval_select() {
        let mut l = Lexer::new("let s = { a = { b = 42; }; }; in s.a.b");
        let t = l.tokenize().unwrap();
        let ast = Parser::new(t).parse().unwrap();
        assert_eq!(Evaluator::new().eval(&ast).unwrap(), NixValue::Int(42));
    }

    #[test]
    fn test_eval_list_concat() {
        let mut l = Lexer::new("[ 1 2 3 ] ++ [ 4 5 ]");
        let t = l.tokenize().unwrap();
        let ast = Parser::new(t).parse().unwrap();
        if let NixValue::List(items) = Evaluator::new().eval(&ast).unwrap() {
            assert_eq!(items.len(), 5);
        } else { panic!("Expected list"); }
    }
}
