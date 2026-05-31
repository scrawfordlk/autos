#![allow(clippy::assign_op_pattern, while_true, non_snake_case)]

fn main() {
    use std::io::Write as StdWrite;
    use std::option::Option as StdOption;
    use std::path::Path as StdPath;
    use std::string::String as StdString;
    use std::vec::Vec as StdVec;

    let args: StdVec<StdString> = std::env::args().collect();
    if args.len() <= 1 {
        eprintln!(
            "Usage: <program> ( -c <input> [ -o <output> ] [ -e ] [ --unsafe ] | -e <inputllvm> )"
        );
        exit_process(1);
    }

    if args[1] == "-c" {
        if args.len() < 3 {
            eprintln!(
                "Usage: <program> ( -c <input> [ -o <output> ] [ -e ] [ --unsafe ] | -e <inputllvm> )"
            );
            exit_process(1);
        }
        let input = args[2].clone();
        let mut file: StdOption<StdString> = StdOption::None;
        let mut emulate_after = false;
        let mut run_semantic = true;

        let mut i = 3;
        while i < args.len() {
            if args[i] == "-o" {
                if i + 1 >= args.len() {
                    eprintln!(
                        "Usage: <program> ( -c <input> [ -o <output> ] [ -e ] [ --unsafe ] | -e <inputllvm> )"
                    );
                    exit_process(1);
                }
                file = StdOption::Some(args[i + 1].clone());
                i += 2;
            } else if args[i] == "-e" {
                emulate_after = true;
                i += 1;
            } else if args[i] == "--unsafe" {
                run_semantic = false;
                i += 1;
            } else {
                // ignore unknown arguments
                i += 1;
            }
        }

        let code: String = compile(
            &std::fs::read_to_string(&input).expect("no program found"),
            run_semantic,
        );
        let code_clone: String = string_clone(&code);

        let output_name: StdString = match file {
            StdOption::Some(s) => s,
            StdOption::None => {
                let mut base = StdPath::new(&input)
                    .file_stem()
                    .expect("can use base name of input")
                    .to_string_lossy()
                    .to_string();
                base.push_str(".ll");
                base
            },
        };

        let String::Inner(vec): String = code;
        let mut file = std::fs::File::create(&output_name).expect("can create file");
        let slice = unsafe { core::slice::from_raw_parts(vec_ptr(&vec), vec_len(&vec)) };
        file.write_all(slice).expect("can write all code");

        if emulate_after {
            let exit_code: usize = emu_execute_llvm(code_clone);
            exit_process(exit_code);
        }

        return;
    }

    if args[1] == "-e" {
        if args.len() < 3 {
            eprintln!(
                "Usage: <program> ( -c <input> [ -o <output> ] [ -e ] [ --unsafe ] | -e <inputllvm> )"
            );
            exit_process(1);
        }
        let llvm_ir: StdString = std::fs::read_to_string(&args[2]).expect("no llvm file found");
        let exit_code: usize = emu_execute_llvm(string(&llvm_ir));
        exit_process(exit_code);
    }

    eprintln!(
        "Usage: <program> ( -c <input> [ -o <output> ] [ -e ] [ --unsafe ] | -e <inputllvm> )"
    );
    exit_process(1);
}

// -----------------------------------------------------------------
// -----------------------------------------------------------------
// ------------------------- Compiler ------------------------------
// -----------------------------------------------------------------
// -----------------------------------------------------------------

/// Compile source code into LLVM-IR.
fn compile(source: &str, do_semantic_analysis: bool) -> String {
    let mut lexer: Lexer = lexer_new(string(source));
    let ast: RAst = parse_language(&mut lexer);

    let items: StringMap<Item> = collect_items(&ast);
    let items: StringMap<Item> = if do_semantic_analysis {
        semantic_check_run(&ast, items)
    } else {
        items
    };

    let mut codegen: Codegen = codegen_new(items);
    codegen_language(&mut codegen, &ast);

    codegen_into_llvm(codegen)
}

// -----------------------------------------------------------------
// ---------------------- Lexical Analysis -------------------------
// -----------------------------------------------------------------

#[derive(Debug)]
enum Token {
    Fn,              // "fn"
    Enum,            // "enum"
    Extern,          // "extern"
    Let,             // "let"
    If,              // "if"
    Else,            // "else"
    While,           // "while"
    Return,          // "return"
    Match,           // "match"
    As,              // "as"
    Unsafe,          // "unsafe"
    Mut,             // "mut"
    Ampersand,       // "&"
    LBrace,          // "{"
    RBrace,          // "}"
    LParen,          // "("
    RParen,          // ")"
    Colon,           // ":"
    DoubleColon,     // "::"
    SemiColon,       // ";"
    Comma,           // ","
    Pipe,            // "|"
    Assign,          // "="
    Bang,            // "!"
    Cmp(Comparison), // ==, !=, <, <=, >, >=
    FatArrow,        // "=>"
    Plus,            // "+"
    Minus,           // "-"
    Star,            // "*"
    Slash,           // "/"
    Remainder,       // "%"
    Usize,           // "usize"
    U8,              // "u8"
    Bool,            // "bool"
    Char,            // "char"
    Str,             // "str"
    Arrow,           // "->"
    Literal(Literal),
    Identifier(String),
    Eof,
}

/// Comparison tokens
#[derive(Debug)]
enum Comparison {
    Eq,
    Ne,
    Gt,
    Lt,
    Geq,
    Leq,
}

/// Literal tokens.
#[derive(Debug)]
enum Literal {
    Int(usize),
    String(String),
    Char(char),
    Bool(bool),
}

/// A type that encapsulates the state of the lexer
enum Lexer {
    /// source file, current token
    Lexer(SourceFile, Token),
}

/// A type that manages the source file
enum SourceFile {
    /// source, next character index, current line, character index of last newline
    SourceFile(String, usize, usize, usize),
}

/// Get the character at the given index.
fn sourceFile_get_char(file: &SourceFile, index: usize) -> Option<char> {
    let SourceFile::SourceFile(string, _, _, _): &SourceFile = file;
    string_get(string, index)
}

/// Returns the current line.
fn sourceFile_current_line(SourceFile::SourceFile(_, _, line, _): &SourceFile) -> usize {
    *line
}

/// Returns the current column in the current line.
fn sourceFile_current_column(file: &SourceFile) -> usize {
    let SourceFile::SourceFile(_, next_char_idx, _, last_newline_idx): &SourceFile = file;
    *next_char_idx - *last_newline_idx
}

/// Returns the index of the beginning of the current line.
fn sourceFile_current_line_start(file: &SourceFile) -> usize {
    let SourceFile::SourceFile(_, _, _, last_newline_idx): &SourceFile = file;
    *last_newline_idx
}

/// Create a lexer and prime it with the first token.
fn lexer_new(source: String) -> Lexer {
    let source_file: SourceFile = SourceFile::SourceFile(source, 0, 0, 0);
    let mut lexer: Lexer = Lexer::Lexer(source_file, Token::Eof);
    lexer_next_token(&mut lexer);
    lexer
}

/// Get immutable access to the lexer source file state.
fn lexer_sourcefile(Lexer::Lexer(source, _): &Lexer) -> &SourceFile {
    source
}

/// Get mutable access to the lexer source file state.
fn lexer_sourcefile_mut(Lexer::Lexer(source, _): &mut Lexer) -> &mut SourceFile {
    source
}

/// Get the current token from the lexer.
fn lexer_current_token(Lexer::Lexer(_, token): &Lexer) -> &Token {
    token
}

/// Get mutable access to the current lexer token slot.
fn lexer_set_current_token(Lexer::Lexer(_, old_token): &mut Lexer, token: Token) {
    *old_token = token;
}

/// Check whether the current token equals `token`.
fn lexer_current_token_eq(lexer: &Lexer, token: &Token) -> bool {
    token_eq(lexer_current_token(lexer), token)
}

/// Peek at the next character without consuming it.
fn lexer_peek_char(lexer: &Lexer) -> Option<char> {
    let SourceFile::SourceFile(string, index, _, _): &SourceFile = lexer_sourcefile(lexer);
    string_get(string, *index)
}

/// Consume and return the next character.
fn lexer_consume_char(lexer: &mut Lexer) -> Option<char> {
    let SourceFile::SourceFile(source, index, line, last_newline_idx): &mut SourceFile =
        lexer_sourcefile_mut(lexer);

    let current: Option<char> = string_get(source, *index);
    *index = *index + 1;

    match current {
        Option::Some(character) => {
            if character == '\n' {
                *line = *line + 1;
                *last_newline_idx = *index;
            }
        },
        Option::None => {},
    }
    current
}

/// Consume `token` when present and report success.
fn lexer_try_consume(lexer: &mut Lexer, token: &Token) -> bool {
    if lexer_current_token_eq(lexer, token) {
        lexer_next_token(lexer);
        true
    } else {
        false
    }
}

/// Consume the next character, erroring if it doesn't match expected.
fn lexer_expect_char(lexer: &mut Lexer, expected: char) {
    match lexer_consume_char(lexer) {
        Option::Some(c) => {
            if c != expected {
                let mut message: String = string("unexpected character: ");
                string_push_string(&mut message, &literal_to_string(&Literal::Char(c)));
                lexer_error(lexer, &message);
            }
        },
        Option::None => lexer_error(lexer, &string("unexpected end of input")),
    }
}

// ---------------------- Lexer ----------------------

/// Consume and return the next token.
fn lexer_next_token(lexer: &mut Lexer) -> Token {
    lexer_skip_attributes(lexer);
    lexer_skip_whitespace(lexer);

    let token: Token = match lexer_peek_char(lexer) {
        Option::Some(c) => {
            if is_alpha(c) {
                let ident: String = lexer_scan_identifier(lexer);
                identifier_to_token(ident)
            } else if is_digit(c) {
                let value: usize = lexer_scan_integer(lexer);
                Token::Literal(Literal::Int(value))
            } else if c == '\'' {
                let ch: char = lexer_scan_char_literal(lexer);
                Token::Literal(Literal::Char(ch))
            } else if c == '"' {
                let s: String = lexer_scan_string_literal(lexer);
                Token::Literal(Literal::String(s))
            } else {
                lexer_scan_symbol(lexer)
            }
        },
        Option::None => Token::Eof,
    };

    lexer_set_current_token(lexer, token_clone(&token));
    token
}

/// Scan an identifier or keyword.
fn lexer_scan_identifier(lexer: &mut Lexer) -> String {
    let mut ident: String = string_new();
    while true {
        match lexer_peek_char(lexer) {
            Option::Some(c) => {
                if is_alphanumeric(c) {
                    lexer_consume_char(lexer);
                    string_push(&mut ident, c);
                } else {
                    return ident;
                }
            },
            Option::None => return ident,
        }
    }
    ident // satisfy compiler
}

/// Convert an identifier to a keyword token if applicable.
fn identifier_to_token(ident: String) -> Token {
    if string_eq(&ident, &string("fn")) {
        Token::Fn
    } else if string_eq(&ident, &string("enum")) {
        Token::Enum
    } else if string_eq(&ident, &string("extern")) {
        Token::Extern
    } else if string_eq(&ident, &string("let")) {
        Token::Let
    } else if string_eq(&ident, &string("if")) {
        Token::If
    } else if string_eq(&ident, &string("else")) {
        Token::Else
    } else if string_eq(&ident, &string("while")) {
        Token::While
    } else if string_eq(&ident, &string("return")) {
        Token::Return
    } else if string_eq(&ident, &string("match")) {
        Token::Match
    } else if string_eq(&ident, &string("as")) {
        Token::As
    } else if string_eq(&ident, &string("unsafe")) {
        Token::Unsafe
    } else if string_eq(&ident, &string("mut")) {
        Token::Mut
    } else if string_eq(&ident, &string("usize")) {
        Token::Usize
    } else if string_eq(&ident, &string("u8")) {
        Token::U8
    } else if string_eq(&ident, &string("bool")) {
        Token::Bool
    } else if string_eq(&ident, &string("char")) {
        Token::Char
    } else if string_eq(&ident, &string("str")) {
        Token::Str
    } else if string_eq(&ident, &string("true")) {
        Token::Literal(Literal::Bool(true))
    } else if string_eq(&ident, &string("false")) {
        Token::Literal(Literal::Bool(false))
    } else {
        Token::Identifier(ident)
    }
}

fn lexer_scan_integer(lexer: &mut Lexer) -> usize {
    let mut value: String = string_new();

    let mut done: bool = false;
    while not(done) {
        match lexer_peek_char(lexer) {
            Option::Some(c) => {
                if is_digit(c) {
                    string_push(&mut value, c);
                    lexer_consume_char(lexer);
                } else {
                    done = true;
                }
            },
            Option::None => done = true,
        }
    }

    match string_to_integer(&value, 10) {
        Option::Some(int) => int,
        _ => {
            let mut message: String = string("invalid integer literal: ");
            string_push_string(&mut message, &value);
            lexer_error(lexer, &message);
        },
    }
}

fn lexer_scan_char_literal(lexer: &mut Lexer) -> char {
    lexer_expect_char(lexer, '\'');
    let c: char = match lexer_consume_char(lexer) {
        Option::Some('\\') => lexer_scan_escape_char(lexer),
        Option::Some(ch) => ch,
        Option::None => lexer_error(lexer, &string("unexpected end of file")),
    };
    lexer_expect_char(lexer, '\'');
    c
}

fn lexer_scan_string_literal(lexer: &mut Lexer) -> String {
    lexer_expect_char(lexer, '"');
    let mut s: String = string_new();
    while true {
        match lexer_consume_char(lexer) {
            Option::Some('"') => return s,
            Option::Some('\\') => string_push(&mut s, lexer_scan_escape_char(lexer)),
            Option::Some(c) => string_push(&mut s, c),
            Option::None => lexer_error(lexer, &string("unexpected end of string literal")),
        }
    }
    s // satisfy compiler
}

/// Scan an escape sequence after backslash.
fn lexer_scan_escape_char(lexer: &mut Lexer) -> char {
    match lexer_consume_char(lexer) {
        Option::Some('n') => '\n',
        Option::Some('t') => '\t',
        Option::Some('r') => '\r',
        Option::Some('0') => '\0',
        Option::Some(c) => c,
        Option::None => lexer_error(lexer, &string("unexpected end of escape sequence")),
    }
}

fn lexer_scan_symbol(lexer: &mut Lexer) -> Token {
    match unwrap::<char>(lexer_consume_char(lexer)) {
        '{' => Token::LBrace,
        '}' => Token::RBrace,
        '(' => Token::LParen,
        ')' => Token::RParen,
        ';' => Token::SemiColon,
        ',' => Token::Comma,
        '|' => Token::Pipe,
        '+' => Token::Plus,
        '*' => Token::Star,
        '/' => lexer_scan_slash(lexer),
        '%' => Token::Remainder,
        '&' => Token::Ampersand,
        ':' => lexer_scan_colon(lexer),
        '=' => lexer_scan_equals(lexer),
        '-' => lexer_scan_minus(lexer),
        '!' => lexer_scan_bang(lexer),
        '<' => lexer_scan_less(lexer),
        '>' => lexer_scan_greater(lexer),
        c => {
            let mut message: String = string("unexpected character: ");
            string_push_string(&mut message, &literal_to_string(&Literal::Char(c)));
            lexer_error(lexer, &message);
        },
    }
}

fn lexer_scan_slash(lexer: &mut Lexer) -> Token {
    match lexer_peek_char(lexer) {
        Option::Some('/') => {
            lexer_consume_char(lexer);
            lexer_skip_line_comment(lexer);
            lexer_next_token(lexer)
        },
        _ => Token::Slash,
    }
}

fn lexer_scan_colon(lexer: &mut Lexer) -> Token {
    match lexer_peek_char(lexer) {
        Option::Some(':') => {
            lexer_consume_char(lexer);
            Token::DoubleColon
        },
        _ => Token::Colon,
    }
}

fn lexer_scan_equals(lexer: &mut Lexer) -> Token {
    match lexer_peek_char(lexer) {
        Option::Some('=') => {
            lexer_consume_char(lexer);
            Token::Cmp(Comparison::Eq)
        },
        Option::Some('>') => {
            lexer_consume_char(lexer);
            Token::FatArrow
        },
        _ => Token::Assign,
    }
}

fn lexer_scan_minus(lexer: &mut Lexer) -> Token {
    match lexer_peek_char(lexer) {
        Option::Some('>') => {
            lexer_consume_char(lexer);
            Token::Arrow
        },
        _ => Token::Minus,
    }
}

fn lexer_scan_bang(lexer: &mut Lexer) -> Token {
    match lexer_peek_char(lexer) {
        Option::Some('=') => {
            lexer_consume_char(lexer);
            Token::Cmp(Comparison::Ne)
        },
        _ => Token::Bang,
    }
}

fn lexer_scan_less(lexer: &mut Lexer) -> Token {
    match lexer_peek_char(lexer) {
        Option::Some('=') => {
            lexer_consume_char(lexer);
            Token::Cmp(Comparison::Leq)
        },
        _ => Token::Cmp(Comparison::Lt),
    }
}

fn lexer_scan_greater(lexer: &mut Lexer) -> Token {
    match lexer_peek_char(lexer) {
        Option::Some('=') => {
            lexer_consume_char(lexer);
            Token::Cmp(Comparison::Geq)
        },
        _ => Token::Cmp(Comparison::Gt),
    }
}

fn lexer_skip_whitespace(lexer: &mut Lexer) {
    while true {
        match lexer_peek_char(lexer) {
            Option::Some(c) => {
                if is_whitespace(c) {
                    lexer_consume_char(lexer);
                } else {
                    return;
                }
            },
            Option::None => return,
        }
    }
}

fn lexer_skip_line_comment(lexer: &mut Lexer) {
    while true {
        match lexer_consume_char(lexer) {
            Option::Some('\n') => return,
            Option::Some(_) => (),
            Option::None => return,
        }
    }
}

/// Skips attributes which are useful in Rust, but unsupported.
fn lexer_skip_attributes(lexer: &mut Lexer) {
    lexer_skip_whitespace(lexer);
    while true {
        match lexer_peek_char(lexer) {
            Option::Some('#') => {
                lexer_consume_char(lexer);
                lexer_skip_whitespace(lexer);

                match lexer_consume_char(lexer) {
                    Option::Some('[') => {
                        let mut skipping: bool = true;
                        while skipping {
                            match lexer_consume_char(lexer) {
                                Option::Some(']') => skipping = false,
                                _ => {},
                            }
                        }
                    },
                    _ => {
                        lexer_error(lexer, &string("expected '[' after '#'"));
                    },
                }
            },
            _ => return,
        }
    }
}

// -------------------------- Parser -------------------------------

/// Abstract Syntax Tree of a parsed Rust source.
enum RAst {
    Language(Vec<RAstItem>),
}

/// Top-level items.
enum RAstItem {
    Function(RAstFunction),
    Enum(RAstEnum),
    ExternBlock(Vec<RAstExternFunction>),
}

/// Function definition.
enum RAstFunction {
    /// unsafe, name, parameters, return type, body
    Function(bool, String, Vec<RAstVariable>, RAstType, RAstBlock),
}

/// Enum definition.
enum RAstEnum {
    /// name, variants
    Enum(String, Vec<RAstVariant>),
}

/// Extern function declaration.
enum RAstExternFunction {
    /// name, parameters, return type
    ExternFunction(String, Vec<RAstVariable>, RAstType),
}

/// Enum variant.
enum RAstVariant {
    /// name, field types (empty vec for unit-like variants)
    Variant(String, Vec<RAstType>),
}

/// Typed variable (`pattern: type`).
enum RAstVariable {
    Variable(RAstPattern, RAstType),
}

/// Block with statements and optional trailing expression.
enum RAstBlock {
    /// statements ending with `;`, optional final expression without `;`
    Block(Vec<RAstStatement>, Option<Box<RAstExpr>>),
}

/// Statements inside blocks.
enum RAstStatement {
    Let(RAstVariable, Box<RAstExpr>),
    Expression(Box<RAstExpr>),
}

/// Pattern forms.
enum RAstPattern {
    Literal(RAstPatternLiteral),
    /// mutable, identifier
    Identifier(bool, String),
    /// enum, variant, fields
    EnumVariant(String, String, Vec<RAstPattern>),
    Wildcard,
}

enum RAstPatternLiteral {
    Int(usize),
    Char(char),
    Bool(bool),
}

/// Type forms from the Rust subset grammar.
#[derive(Debug)]
enum RAstType {
    U8,
    Usize,
    Bool,
    Char,
    Unit,
    Never,
    Custom(String),
    /// inner, mutable
    Reference(Box<RAstType>, bool),
    /// `*mut T`
    RawPointerMut(Box<RAstType>),
}

/// Literal values.
enum RAstLiteral {
    Int(usize),
    String(String),
    Char(char),
    Bool(bool),
}

/// Path segments joined by `::`.
enum RAstPath {
    Path(Vec<String>),
}

/// A Rust expression.
enum RAstExpr {
    Return(Option<Box<RAstExpr>>),
    Assign(Box<RAstExpr>, Box<RAstExpr>),
    Binary(RAstBinaryOp, Box<RAstExpr>, Box<RAstExpr>),
    Cast(Box<RAstExpr>, RAstType),
    Unary(RAstUnaryOp, Box<RAstExpr>),
    Literal(RAstLiteral),
    VariableUse(String),
    Call(RAstPath, Vec<RAstExpr>),
    /// unsafe, block
    Block(bool, RAstBlock),
    If(RAstIf),
    While(Box<RAstExpr>, RAstBlock),
    Match(Box<RAstExpr>, Vec<RAstArm>),
}

/// Binary operators.
enum RAstBinaryOp {
    Arithmetic(RAstArithmeticOp),
    Comparison(RAstComparisonOp),
}

/// Binary arithmetic operators.
enum RAstArithmeticOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

/// Comparison operators.
enum RAstComparisonOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

/// Unary operators.
enum RAstUnaryOp {
    Dereference,
    /// `&` / `&mut`
    Reference(bool),
}

/// An if expression.
enum RAstIf {
    /// condition, then block, optional else branch
    If(Box<RAstExpr>, RAstBlock, Option<RAstElse>),
}

/// An else branch of an if expression.
enum RAstElse {
    /// `... else if ... { ... } ...`
    If(Box<RAstIf>),
    /// `... else { ... } ...`
    Block(RAstBlock),
}

/// A match arm.
enum RAstArm {
    /// "... => ...,"
    Arm(Vec<RAstPattern>, RAstExpr),
}

/// Convert a parsed AST path to a single string.
fn rAstPath_to_string(RAstPath::Path(segments): &RAstPath) -> String {
    let mut result: String = string_new();
    let mut i: usize = 0;

    while i < vec_len::<String>(segments) {
        if i > 0 {
            string_push_str(&mut result, "::");
        }
        let segment: &String = vec_at::<String>(segments, i);
        string_push_string(&mut result, segment);
        i = i + 1;
    }
    result
}

/// Get the type represented by a literal.
fn rastLiteral_type(literal: &RAstLiteral) -> RAstType {
    match literal {
        RAstLiteral::Int(_) => RAstType::Usize,
        RAstLiteral::Char(_) => RAstType::Char,
        RAstLiteral::Bool(_) => RAstType::Bool,
        RAstLiteral::String(_) => {
            RAstType::Reference(box_new::<RAstType>(RAstType::Custom(string("str"))), false)
        },
    }
}

/// Get the integer value of a pattern literal (integer, char, bool).
fn rAstPatternLiteral_value(literal: &RAstPatternLiteral) -> usize {
    match literal {
        RAstPatternLiteral::Int(value) => *value,
        RAstPatternLiteral::Char(value) => *value as usize,
        RAstPatternLiteral::Bool(value) => *value as usize,
    }
}

/// Convert Rust AST type into a simple LLVM-IR type name.
fn rAstType_to_llvm_name(ty: &RAstType) -> String {
    match ty {
        RAstType::U8 => string("i8"),
        RAstType::Usize => string("i64"), // assume 64-bit for now
        RAstType::Bool => string("i1"),
        RAstType::Char => string("i8"),
        RAstType::Unit => string("void"),
        RAstType::Never => string("void"),
        RAstType::Custom(_) => string("i64"),
        RAstType::Reference(_, _) => string("ptr"),
        RAstType::RawPointerMut(_) => string("ptr"),
    }
}

fn rAstType_is_numeric(ty: &RAstType) -> bool {
    match ty {
        RAstType::U8 => true,
        RAstType::Usize => true,
        _ => false,
    }
}

/// Coerce two types into one type.
/// Assumes that only the following cases can occur:
/// 1. left == right
/// 2. left == Never
/// 3. right == Never
///
/// The type returned is only Never if left == right == Never.
fn rAstType_coerce(left: RAstType, right: RAstType) -> RAstType {
    if rAstType_eq(&left, &RAstType::Never) {
        right
    } else {
        left
    }
}

/// Checks for equality between two types.
/// If one of the arguments is Never, return true, since Never matches every type.
fn type_matches(left: &RAstType, right: &RAstType) -> bool {
    or(
        // Never is a special type that indicates the value is unreachable, so it matches every
        // type
        or(
            rAstType_eq(left, &RAstType::Never),
            rAstType_eq(right, &RAstType::Never),
        ),
        rAstType_eq(left, right),
    )
}

/// Return true if the type has a value.
/// This is true for all types, other than Unit and Never.
fn type_has_value(ty: &RAstType) -> bool {
    not(type_matches(ty, &RAstType::Unit))
}

/// Require and consume the given token.
fn expect_token(lexer: &mut Lexer, token: &Token) {
    if not(lexer_try_consume(lexer, token)) {
        let bad_token: &Token = lexer_current_token(lexer);
        let mut message: String = string("expected ");
        string_push_string(&mut message, &token_to_string(token));
        string_push_str(&mut message, ", but got: ");
        string_push_string(&mut message, &token_to_string(bad_token));
        parse_error(lexer, &message);
    }
}

/// Read and consume the current identifier token.
fn expect_identifier(lexer: &mut Lexer) -> String {
    match lexer_current_token(lexer) {
        Token::Identifier(name) => {
            let name: String = string_clone(name);
            lexer_next_token(lexer);
            name
        },
        token => {
            let mut message: String = string("expected identifier, but got: ");
            string_push_string(&mut message, &token_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

fn parse_language(lexer: &mut Lexer) -> RAst {
    let mut items: Vec<RAstItem> = vec_new::<RAstItem>();

    while not(lexer_current_token_eq(lexer, &Token::Eof)) {
        match lexer_current_token(lexer) {
            Token::Unsafe => match lexer_next_token(lexer) {
                Token::Extern => {
                    let extern_block: RAstItem = RAstItem::ExternBlock(parse_extern_block(lexer));
                    vec_push::<RAstItem>(&mut items, extern_block);
                },
                Token::Fn => {
                    let function: RAstItem = RAstItem::Function(parse_function(lexer, true));
                    vec_push::<RAstItem>(&mut items, function);
                },
                token => {
                    let mut message: String = string("expected fn or extern, but got: ");
                    string_push_string(&mut message, &token_to_string(&token));
                    parse_error(lexer, &message);
                },
            },
            Token::Fn => {
                let function: RAstItem = RAstItem::Function(parse_function(lexer, false));
                vec_push::<RAstItem>(&mut items, function);
            },
            Token::Enum => {
                let enumeration: RAstItem = RAstItem::Enum(parse_enum(lexer));
                vec_push::<RAstItem>(&mut items, enumeration);
            },
            token => {
                let mut message: String =
                    string("expected function, enum, or extern block, but got: ");
                string_push_string(&mut message, &token_to_string(token));
                parse_error(lexer, &message);
            },
        }
    }

    RAst::Language(items)
}

fn parse_extern_block(lexer: &mut Lexer) -> Vec<RAstExternFunction> {
    expect_token(lexer, &Token::Extern);

    match lexer_current_token(lexer) {
        Token::Literal(Literal::String(value)) => {
            if not(string_eq(value, &string("C"))) {
                let mut message: String = string("expected \"C\", but got: ");
                string_push_string(&mut message, &token_to_string(lexer_current_token(lexer)));
                parse_error(lexer, &message);
            }
            lexer_next_token(lexer);
        },
        _ => {
            let mut message: String = string("expected \"C\", but got: ");
            string_push_string(&mut message, &token_to_string(lexer_current_token(lexer)));
            parse_error(lexer, &message);
        },
    }

    expect_token(lexer, &Token::LBrace);

    let mut functions: Vec<RAstExternFunction> = vec_new::<RAstExternFunction>();
    while not(lexer_current_token_eq(lexer, &Token::RBrace)) {
        let function: RAstExternFunction = parse_function_declaration(lexer);
        vec_push::<RAstExternFunction>(&mut functions, function);
    }
    expect_token(lexer, &Token::RBrace);

    functions
}

fn parse_function_declaration(lexer: &mut Lexer) -> RAstExternFunction {
    expect_token(lexer, &Token::Fn);
    let name: String = expect_identifier(lexer);
    expect_token(lexer, &Token::LParen);

    let mut parameters: Vec<RAstVariable> = vec_new::<RAstVariable>();
    if not(lexer_current_token_eq(lexer, &Token::RParen)) {
        let variable: RAstVariable = parse_variable(lexer);
        vec_push::<RAstVariable>(&mut parameters, variable);

        while and(
            lexer_try_consume(lexer, &Token::Comma),
            not(lexer_current_token_eq(lexer, &Token::RParen)),
        ) {
            let variable: RAstVariable = parse_variable(lexer);
            vec_push::<RAstVariable>(&mut parameters, variable);
        }
    }
    expect_token(lexer, &Token::RParen);

    let return_type: RAstType = if lexer_try_consume(lexer, &Token::Arrow) {
        parse_type(lexer)
    } else {
        RAstType::Unit
    };

    expect_token(lexer, &Token::SemiColon);
    RAstExternFunction::ExternFunction(name, parameters, return_type)
}

fn parse_function(lexer: &mut Lexer, is_unsafe: bool) -> RAstFunction {
    expect_token(lexer, &Token::Fn);

    let name: String = expect_identifier(lexer);
    expect_token(lexer, &Token::LParen);

    let mut parameters: Vec<RAstVariable> = vec_new::<RAstVariable>();
    if not(lexer_current_token_eq(lexer, &Token::RParen)) {
        let variable: RAstVariable = parse_variable(lexer);
        vec_push::<RAstVariable>(&mut parameters, variable);

        while and(
            lexer_try_consume(lexer, &Token::Comma),
            not(lexer_current_token_eq(lexer, &Token::RParen)),
        ) {
            let variable: RAstVariable = parse_variable(lexer);
            vec_push::<RAstVariable>(&mut parameters, variable);
        }
    }
    expect_token(lexer, &Token::RParen);

    let return_type: RAstType = if lexer_try_consume(lexer, &Token::Arrow) {
        parse_type(lexer)
    } else {
        RAstType::Unit
    };

    let body: RAstBlock = parse_block(lexer);

    RAstFunction::Function(is_unsafe, name, parameters, return_type, body)
}

fn parse_enum(lexer: &mut Lexer) -> RAstEnum {
    expect_token(lexer, &Token::Enum);
    let name: String = expect_identifier(lexer);
    expect_token(lexer, &Token::LBrace);

    let mut variants: Vec<RAstVariant> = vec_new::<RAstVariant>();
    let first_variant: RAstVariant = parse_variant(lexer);
    vec_push::<RAstVariant>(&mut variants, first_variant);
    expect_token(lexer, &Token::Comma);

    while not(lexer_current_token_eq(lexer, &Token::RBrace)) {
        let variant: RAstVariant = parse_variant(lexer);
        vec_push::<RAstVariant>(&mut variants, variant);
        expect_token(lexer, &Token::Comma);
    }
    expect_token(lexer, &Token::RBrace);

    RAstEnum::Enum(name, variants)
}

fn parse_variant(lexer: &mut Lexer) -> RAstVariant {
    let name: String = expect_identifier(lexer);

    let mut field_types: Vec<RAstType> = vec_new::<RAstType>();
    if lexer_try_consume(lexer, &Token::LParen) {
        vec_push::<RAstType>(&mut field_types, parse_type(lexer));

        while lexer_try_consume(lexer, &Token::Comma) {
            vec_push::<RAstType>(&mut field_types, parse_type(lexer));
        }
        expect_token(lexer, &Token::RParen);
    }

    RAstVariant::Variant(name, field_types)
}

fn parse_block(lexer: &mut Lexer) -> RAstBlock {
    expect_token(lexer, &Token::LBrace);
    let mut statements: Vec<RAstStatement> = vec_new::<RAstStatement>();
    let mut tail: Option<Box<RAstExpr>> = Option::None;

    while not(lexer_current_token_eq(lexer, &Token::RBrace)) {
        if lexer_current_token_eq(lexer, &Token::Let) {
            let let_binding: RAstStatement = parse_binding(lexer);
            vec_push::<RAstStatement>(&mut statements, let_binding);
            expect_token(lexer, &Token::SemiColon);
        } else {
            let expression: RAstExpr = parse_expression(lexer);

            if lexer_current_token_eq(lexer, &Token::RBrace) {
                // end of block with expression as return value
                lexer_next_token(lexer);
                tail = Option::Some(box_new::<RAstExpr>(expression));
                return RAstBlock::Block(statements, tail);
            } else {
                lexer_try_consume(lexer, &Token::SemiColon); // optional semi-colon
                let expr_statement = RAstStatement::Expression(box_new::<RAstExpr>(expression));
                vec_push::<RAstStatement>(&mut statements, expr_statement);
            }
        }
    }
    expect_token(lexer, &Token::RBrace);

    RAstBlock::Block(statements, tail)
}

fn parse_binding(lexer: &mut Lexer) -> RAstStatement {
    expect_token(lexer, &Token::Let);
    let variable: RAstVariable = parse_variable(lexer);
    expect_token(lexer, &Token::Assign);
    let value: RAstExpr = parse_expression(lexer);
    RAstStatement::Let(variable, box_new::<RAstExpr>(value))
}

fn parse_variable(lexer: &mut Lexer) -> RAstVariable {
    let pattern: RAstPattern = parse_pattern(lexer);
    expect_token(lexer, &Token::Colon);
    let ty: RAstType = parse_type(lexer);
    RAstVariable::Variable(pattern, ty)
}

fn parse_type(lexer: &mut Lexer) -> RAstType {
    match lexer_current_token(lexer) {
        Token::U8 => {
            lexer_next_token(lexer);
            RAstType::U8
        },
        Token::Usize => {
            lexer_next_token(lexer);
            RAstType::Usize
        },
        Token::Char => {
            lexer_next_token(lexer);
            RAstType::Char
        },
        Token::Bool => {
            lexer_next_token(lexer);
            RAstType::Bool
        },
        Token::LParen => {
            lexer_next_token(lexer);
            expect_token(lexer, &Token::RParen);
            RAstType::Unit
        },
        Token::Bang => {
            lexer_next_token(lexer);
            RAstType::Never
        },
        Token::Ampersand => {
            lexer_next_token(lexer);

            if lexer_try_consume(lexer, &Token::Str) {
                // TODO: remove this and handle like a user-defined type
                return RAstType::Reference(
                    box_new::<RAstType>(RAstType::Custom(string("str"))),
                    false,
                );
            }

            let mutable: bool = lexer_try_consume(lexer, &Token::Mut);
            let inner: RAstType = parse_type(lexer);
            RAstType::Reference(box_new::<RAstType>(inner), mutable)
        },
        Token::Star => {
            lexer_next_token(lexer);
            expect_token(lexer, &Token::Mut);
            let inner: RAstType = parse_type(lexer);
            RAstType::RawPointerMut(box_new::<RAstType>(inner))
        },
        Token::Identifier(_) => {
            let enum_name: String = expect_identifier(lexer);
            RAstType::Custom(enum_name)
        },
        token => {
            let mut message: String = string("expected a type, but got: ");
            string_push_string(&mut message, &token_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

fn parse_expression(lexer: &mut Lexer) -> RAstExpr {
    match lexer_current_token(lexer) {
        Token::Return => {
            lexer_next_token(lexer);
            match lexer_current_token(lexer) {
                Token::SemiColon | Token::RBrace => RAstExpr::Return(Option::None),
                _ => {
                    let expression: RAstExpr = parse_expression(lexer);
                    RAstExpr::Return(Option::Some(box_new::<RAstExpr>(expression)))
                },
            }
        },
        _ => parse_assignment(lexer),
    }
}

fn parse_assignment(lexer: &mut Lexer) -> RAstExpr {
    let left: RAstExpr = parse_comparison(lexer);
    if lexer_try_consume(lexer, &Token::Assign) {
        let right: RAstExpr = parse_assignment(lexer);
        RAstExpr::Assign(box_new::<RAstExpr>(left), box_new::<RAstExpr>(right))
    } else {
        left
    }
}

fn parse_comparison(lexer: &mut Lexer) -> RAstExpr {
    let left: RAstExpr = parse_arithmetic(lexer);

    match lexer_current_token(lexer) {
        Token::Cmp(comparison) => {
            let comparison: Comparison = comparison_clone(comparison);
            lexer_next_token(lexer);

            let right: RAstExpr = parse_arithmetic(lexer);

            let operator: RAstComparisonOp = match comparison {
                Comparison::Eq => RAstComparisonOp::Eq,
                Comparison::Ne => RAstComparisonOp::Ne,
                Comparison::Gt => RAstComparisonOp::Gt,
                Comparison::Lt => RAstComparisonOp::Lt,
                Comparison::Geq => RAstComparisonOp::Ge,
                Comparison::Leq => RAstComparisonOp::Le,
            };

            RAstExpr::Binary(
                RAstBinaryOp::Comparison(operator),
                box_new::<RAstExpr>(left),
                box_new::<RAstExpr>(right),
            )
        },
        _ => left,
    }
}

fn parse_arithmetic(lexer: &mut Lexer) -> RAstExpr {
    let mut left: RAstExpr = parse_term(lexer);

    while or(
        lexer_current_token_eq(lexer, &Token::Plus),
        lexer_current_token_eq(lexer, &Token::Minus),
    ) {
        let operator: RAstArithmeticOp = match lexer_current_token(lexer) {
            Token::Plus => RAstArithmeticOp::Add,
            Token::Minus => RAstArithmeticOp::Sub,
            _ => panic!("unreachable"),
        };
        lexer_next_token(lexer);

        let right: RAstExpr = parse_term(lexer);

        left = RAstExpr::Binary(
            RAstBinaryOp::Arithmetic(operator),
            box_new::<RAstExpr>(left),
            box_new::<RAstExpr>(right),
        );
    }
    left
}

fn parse_term(lexer: &mut Lexer) -> RAstExpr {
    let mut left: RAstExpr = parse_cast(lexer);

    while or(
        lexer_current_token_eq(lexer, &Token::Star),
        or(
            lexer_current_token_eq(lexer, &Token::Slash),
            lexer_current_token_eq(lexer, &Token::Remainder),
        ),
    ) {
        let operator: RAstArithmeticOp = match lexer_current_token(lexer) {
            Token::Star => RAstArithmeticOp::Mul,
            Token::Slash => RAstArithmeticOp::Div,
            Token::Remainder => RAstArithmeticOp::Rem,
            _ => panic!("unreachable"),
        };
        lexer_next_token(lexer);

        let right: RAstExpr = parse_cast(lexer);

        left = RAstExpr::Binary(
            RAstBinaryOp::Arithmetic(operator),
            box_new::<RAstExpr>(left),
            box_new::<RAstExpr>(right),
        );
    }
    left
}

fn parse_cast(lexer: &mut Lexer) -> RAstExpr {
    let mut expression: RAstExpr = parse_unary(lexer);

    while lexer_try_consume(lexer, &Token::As) {
        let cast_type: RAstType = parse_type(lexer);
        expression = RAstExpr::Cast(box_new::<RAstExpr>(expression), cast_type);
    }
    expression
}

fn parse_unary(lexer: &mut Lexer) -> RAstExpr {
    match lexer_current_token(lexer) {
        Token::Ampersand => {
            lexer_next_token(lexer);
            let mutable: bool = lexer_try_consume(lexer, &Token::Mut);
            let inner: RAstExpr = parse_unary(lexer);
            RAstExpr::Unary(RAstUnaryOp::Reference(mutable), box_new::<RAstExpr>(inner))
        },
        Token::Star => {
            lexer_next_token(lexer);
            let inner: RAstExpr = parse_unary(lexer);
            RAstExpr::Unary(RAstUnaryOp::Dereference, box_new::<RAstExpr>(inner))
        },
        _ => parse_factor(lexer),
    }
}

fn parse_factor(lexer: &mut Lexer) -> RAstExpr {
    match lexer_current_token(lexer) {
        Token::Literal(_) => RAstExpr::Literal(parse_literal(lexer)),
        Token::Identifier(_) => {
            let first_identifier: String = expect_identifier(lexer);

            if lexer_current_token_eq(lexer, &Token::DoubleColon) {
                let path: RAstPath = parse_path(lexer, first_identifier);

                expect_token(lexer, &Token::LParen);
                parse_call(lexer, path)
            } else if lexer_current_token_eq(lexer, &Token::LParen) {
                let mut path_segments: Vec<String> = vec_new::<String>();
                vec_push::<String>(&mut path_segments, first_identifier);
                parse_call(lexer, RAstPath::Path(path_segments))
            } else {
                RAstExpr::VariableUse(first_identifier)
            }
        },
        Token::LParen => {
            lexer_next_token(lexer);
            let expression: RAstExpr = parse_expression(lexer);
            expect_token(lexer, &Token::RParen);
            expression
        },
        Token::Unsafe => {
            lexer_next_token(lexer);
            RAstExpr::Block(true, parse_block(lexer))
        },
        Token::LBrace => RAstExpr::Block(false, parse_block(lexer)),
        Token::If => RAstExpr::If(parse_if(lexer)),
        Token::While => parse_while(lexer),
        Token::Match => parse_match(lexer),
        token => {
            let mut message: String = string("unexpected token: ");
            string_push_string(&mut message, &token_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

fn parse_if(lexer: &mut Lexer) -> RAstIf {
    expect_token(lexer, &Token::If);
    let condition: RAstExpr = parse_expression(lexer);
    let then_block: RAstBlock = parse_block(lexer);

    let else_branch: Option<RAstElse> = if lexer_try_consume(lexer, &Token::Else) {
        if lexer_current_token_eq(lexer, &Token::If) {
            let else_if: RAstIf = parse_if(lexer);
            Option::Some(RAstElse::If(box_new::<RAstIf>(else_if)))
        } else {
            let else_block: RAstBlock = parse_block(lexer);
            Option::Some(RAstElse::Block(else_block))
        }
    } else {
        Option::None
    };

    RAstIf::If(box_new::<RAstExpr>(condition), then_block, else_branch)
}

fn parse_while(lexer: &mut Lexer) -> RAstExpr {
    expect_token(lexer, &Token::While);
    let condition: RAstExpr = parse_expression(lexer);
    let body: RAstBlock = parse_block(lexer);
    RAstExpr::While(box_new::<RAstExpr>(condition), body)
}

fn parse_match(lexer: &mut Lexer) -> RAstExpr {
    expect_token(lexer, &Token::Match);
    let value: RAstExpr = parse_expression(lexer);
    expect_token(lexer, &Token::LBrace);

    let mut arms: Vec<RAstArm> = vec_new::<RAstArm>();
    while not(lexer_current_token_eq(lexer, &Token::RBrace)) {
        let arm: RAstArm = parse_arm(lexer);
        vec_push::<RAstArm>(&mut arms, arm);
    }
    expect_token(lexer, &Token::RBrace);

    RAstExpr::Match(box_new::<RAstExpr>(value), arms)
}

fn parse_arm(lexer: &mut Lexer) -> RAstArm {
    let mut patterns: Vec<RAstPattern> = vec_new::<RAstPattern>();
    let pattern: RAstPattern = parse_pattern(lexer);
    vec_push::<RAstPattern>(&mut patterns, pattern);

    while lexer_try_consume(lexer, &Token::Pipe) {
        let pattern: RAstPattern = parse_pattern(lexer);
        vec_push::<RAstPattern>(&mut patterns, pattern);
    }

    expect_token(lexer, &Token::FatArrow);

    let expression: RAstExpr = parse_expression(lexer);
    expect_token(lexer, &Token::Comma);
    RAstArm::Arm(patterns, expression)
}

fn parse_pattern(lexer: &mut Lexer) -> RAstPattern {
    match lexer_current_token(lexer) {
        Token::Literal(_) => RAstPattern::Literal(match parse_literal(lexer) {
            RAstLiteral::Int(value) => RAstPatternLiteral::Int(value),
            RAstLiteral::Char(value) => RAstPatternLiteral::Char(value),
            RAstLiteral::Bool(value) => RAstPatternLiteral::Bool(value),
            RAstLiteral::String(_) => {
                parse_error(lexer, &string("matching on string literals is unsupported"))
            },
        }),
        Token::Mut => {
            lexer_next_token(lexer);
            let identifier: String = expect_identifier(lexer);
            RAstPattern::Identifier(true, identifier)
        },
        Token::Identifier(_) => {
            let identifier: String = expect_identifier(lexer);

            if string_eq(&identifier, &string("_")) {
                RAstPattern::Wildcard
            } else if lexer_try_consume(lexer, &Token::DoubleColon) {
                let variant_name: String = expect_identifier(lexer);

                let mut fields: Vec<RAstPattern> = vec_new::<RAstPattern>();
                if lexer_try_consume(lexer, &Token::LParen) {
                    if not(lexer_current_token_eq(lexer, &Token::RParen)) {
                        let pattern: RAstPattern = parse_pattern(lexer);
                        vec_push::<RAstPattern>(&mut fields, pattern);

                        while and(
                            lexer_try_consume(lexer, &Token::Comma),
                            not(lexer_current_token_eq(lexer, &Token::RParen)),
                        ) {
                            let pattern: RAstPattern = parse_pattern(lexer);
                            vec_push::<RAstPattern>(&mut fields, pattern);
                        }
                    }
                    expect_token(lexer, &Token::RParen);
                }

                RAstPattern::EnumVariant(identifier, variant_name, fields)
            } else {
                RAstPattern::Identifier(false, identifier)
            }
        },
        token => {
            let mut message: String = string("expected pattern, but got: ");
            string_push_string(&mut message, &token_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

fn parse_call(lexer: &mut Lexer, callee: RAstPath) -> RAstExpr {
    expect_token(lexer, &Token::LParen);

    let mut arguments: Vec<RAstExpr> = vec_new::<RAstExpr>();
    if not(lexer_current_token_eq(lexer, &Token::RParen)) {
        let first_argument: RAstExpr = parse_expression(lexer);
        vec_push::<RAstExpr>(&mut arguments, first_argument);

        while and(
            lexer_try_consume(lexer, &Token::Comma),
            not(lexer_current_token_eq(lexer, &Token::RParen)),
        ) {
            let argument: RAstExpr = parse_expression(lexer);
            vec_push::<RAstExpr>(&mut arguments, argument);
        }
    }
    expect_token(lexer, &Token::RParen);

    RAstExpr::Call(callee, arguments)
}

fn parse_path(lexer: &mut Lexer, first_segment: String) -> RAstPath {
    let mut segments: Vec<String> = vec_new::<String>();
    vec_push::<String>(&mut segments, first_segment);
    while lexer_try_consume(lexer, &Token::DoubleColon) {
        let segment: String = expect_identifier(lexer);
        vec_push::<String>(&mut segments, segment);
    }
    RAstPath::Path(segments)
}

fn parse_literal(lexer: &mut Lexer) -> RAstLiteral {
    match lexer_current_token(lexer) {
        Token::Literal(literal) => {
            let literal: RAstLiteral = match literal {
                Literal::Int(value) => RAstLiteral::Int(*value),
                Literal::String(value) => RAstLiteral::String(string_clone(value)),
                Literal::Char(value) => RAstLiteral::Char(*value),
                Literal::Bool(value) => RAstLiteral::Bool(*value),
            };
            lexer_next_token(lexer);
            literal
        },
        token => {
            let mut message: String = string("expected literal, but got: ");
            string_push_string(&mut message, &token_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

// TODO: This should be shorter. E.g. add a name to FnSignature, which can be used in the AST.
fn collect_items(ast: &RAst) -> StringMap<Item> {
    let RAst::Language(ast_items): &RAst = ast;
    let mut items: StringMap<Item> = stringMap_new::<Item>();

    let mut i: usize = 0;
    while i < vec_len::<RAstItem>(ast_items) {
        match vec_at::<RAstItem>(ast_items, i) {
            RAstItem::Function(RAstFunction::Function(is_unsafe, name, params, return_type, _)) => {
                let mut param_types: Vec<RAstType> = vec_new::<RAstType>();
                let mut param_index: usize = 0;
                while param_index < vec_len::<RAstVariable>(params) {
                    let RAstVariable::Variable(_, parameter_type): &RAstVariable =
                        vec_at::<RAstVariable>(params, param_index);
                    vec_push::<RAstType>(&mut param_types, rAstType_clone(parameter_type));
                    param_index = param_index + 1;
                }

                let signature: FnSignature =
                    FnSignature::Fn(param_types, rAstType_clone(return_type), *is_unsafe);
                stringMap_insert::<Item>(&mut items, string_clone(name), Item::Function(signature));
            },
            RAstItem::Enum(enum_item) => {
                let RAstEnum::Enum(name, variants): &RAstEnum = enum_item;

                let mut cloned_variants: Vec<RAstVariant> = vec_new::<RAstVariant>();
                let mut i: usize = 0;
                while i < vec_len::<RAstVariant>(variants) {
                    let variant: &RAstVariant = vec_at::<RAstVariant>(variants, i);
                    let RAstVariant::Variant(variant_name, fields): &RAstVariant = variant;

                    let mut cloned_fields: Vec<RAstType> = vec_new::<RAstType>();
                    let mut field_index: usize = 0;
                    while field_index < vec_len::<RAstType>(fields) {
                        let field_type: &RAstType = vec_at::<RAstType>(fields, field_index);
                        vec_push::<RAstType>(&mut cloned_fields, rAstType_clone(field_type));
                        field_index = field_index + 1;
                    }
                    vec_push::<RAstVariant>(
                        &mut cloned_variants,
                        RAstVariant::Variant(string_clone(variant_name), cloned_fields),
                    );
                    i = i + 1;
                }

                let cloned_enum: RAstEnum = RAstEnum::Enum(string_clone(name), cloned_variants);
                stringMap_insert::<Item>(&mut items, string_clone(name), Item::Enum(cloned_enum));
            },
            RAstItem::ExternBlock(functions) => {
                let mut i: usize = 0;
                while i < vec_len::<RAstExternFunction>(functions) {
                    let function: &RAstExternFunction = vec_at::<RAstExternFunction>(functions, i);
                    let RAstExternFunction::ExternFunction(name, params, return_type): &RAstExternFunction =
                        function;

                    let mut param_types: Vec<RAstType> = vec_new::<RAstType>();
                    let mut param_index: usize = 0;
                    while param_index < vec_len::<RAstVariable>(params) {
                        let RAstVariable::Variable(_, parameter_type): &RAstVariable =
                            vec_at::<RAstVariable>(params, param_index);
                        vec_push::<RAstType>(&mut param_types, rAstType_clone(parameter_type));
                        param_index = param_index + 1;
                    }

                    let signature: FnSignature =
                        FnSignature::Fn(param_types, rAstType_clone(return_type), true);
                    stringMap_insert::<Item>(
                        &mut items,
                        string_clone(name),
                        Item::Function(signature),
                    );
                    i = i + 1;
                }
            },
        }
        i = i + 1;
    }
    items
}

/// Semantic analysis state.
enum Semantic {
    /// global items, local symbol table, current function return type, unsafe context depth
    Semantic(StringMap<Item>, StringMapStack<Variable>, RAstType, usize),
}

fn semantic_new(items: StringMap<Item>) -> Semantic {
    Semantic::Semantic(items, stringMapStack_new::<Variable>(), RAstType::Unit, 0)
}

fn semantic_globals(Semantic::Semantic(globals, _, _, _): &Semantic) -> &StringMap<Item> {
    globals
}

fn semantic_into_items(Semantic::Semantic(globals, _, _, _): Semantic) -> StringMap<Item> {
    globals
}

fn semantic_locals(semantic: &Semantic) -> &StringMapStack<Variable> {
    let Semantic::Semantic(_, locals, _, _): &Semantic = semantic;
    locals
}

fn semantic_locals_mut(semantic: &mut Semantic) -> &mut StringMapStack<Variable> {
    let Semantic::Semantic(_, locals, _, _): &mut Semantic = semantic;
    locals
}

fn semantic_current_fn_return_type(semantic: &Semantic) -> &RAstType {
    let Semantic::Semantic(_, _, return_type, _): &Semantic = semantic;
    return_type
}

fn semantic_set_current_fn_return_type(semantic: &mut Semantic, ty: RAstType) {
    let Semantic::Semantic(_, _, return_type, _): &mut Semantic = semantic;
    *return_type = ty;
}

/// Get the raw unsafe depth value.
fn semantic_unsafe_depth(semantic: &Semantic) -> usize {
    let Semantic::Semantic(_, _, _, unsafe_depth): &Semantic = semantic;
    *unsafe_depth
}

/// Set the raw unsafe depth value.
fn semantic_set_unsafe_depth(semantic: &mut Semantic, unsafe_depth: usize) {
    let Semantic::Semantic(_, _, _, current_unsafe_depth): &mut Semantic = semantic;
    *current_unsafe_depth = unsafe_depth;
}

/// Enter a new unsafe context.
fn semantic_push_unsafe_context(semantic: &mut Semantic) {
    let current_depth: usize = semantic_unsafe_depth(semantic);
    semantic_set_unsafe_depth(semantic, current_depth + 1);
}

/// Exit an unsafe context.
fn semantic_pop_unsafe_context(semantic: &mut Semantic) {
    let current_depth: usize = semantic_unsafe_depth(semantic);
    if current_depth > 0 {
        semantic_set_unsafe_depth(semantic, current_depth - 1);
    }
}

/// Return true if unsafe operations are allowed.
fn semantic_is_unsafe_context(semantic: &Semantic) -> bool {
    semantic_unsafe_depth(semantic) > 0
}

/// Run semantic analysis and return collected items.
fn semantic_check_run(ast: &RAst, items: StringMap<Item>) -> StringMap<Item> {
    let mut semantic: Semantic = semantic_new(items);
    semantic_check_language(&mut semantic, ast);
    semantic_into_items(semantic)
}

/// Check if the given types are equal, otherwise throw an error.
fn semantic_expect_exact_type_match(left: &RAstType, right: &RAstType) {
    if not(rAstType_eq(left, right)) {
        semantic_check_error("types do not match perfectly");
    }
}

/// Return true if the given types match.
///
/// Two types a, b match if:
/// 1. a == b
/// 2. a == Never
/// 3. b == Never
fn semantic_expect_rough_type_match(left: &RAstType, right: &RAstType) {
    if not(type_matches(left, right)) {
        semantic_check_error("type mismatch");
    }
}

fn semantic_expect_numeric_type(ty: &RAstType) {
    if not(rAstType_is_numeric(ty)) {
        semantic_check_error("expected numeric type");
    }
}

fn semantic_expect_bool_type(ty: &RAstType) {
    if not(rAstType_eq(ty, &RAstType::Bool)) {
        semantic_check_error("expected bool type");
    }
}

/// Lookup a variable in local scopes.
fn semantic_lookup_variable(semantic: &Semantic, name: &String) -> Option<Variable> {
    match stringMapStack_lookup::<Variable>(semantic_locals(semantic), name) {
        Option::Some(entry) => {
            let Variable::Variable(variable_type, mutable) = entry;
            Option::Some(Variable::Variable(rAstType_clone(variable_type), *mutable))
        },
        Option::None => Option::None,
    }
}

/// Lookup a function signature in the global item map.
fn semantic_lookup_function_signature(semantic: &Semantic, name: &String) -> Option<FnSignature> {
    match stringMap_get::<Item>(semantic_globals(semantic), name) {
        Option::Some(Item::Function(signature)) => Option::Some(fnSignature_clone(signature)),
        _ => Option::None,
    }
}

/// Enter a new local scope.
fn semantic_enter_scope(semantic: &mut Semantic) {
    stringMapStack_push_empty::<Variable>(semantic_locals_mut(semantic));
}

/// Leave the current local scope.
fn semantic_leave_scope(semantic: &mut Semantic) -> bool {
    stringMapStack_pop::<Variable>(semantic_locals_mut(semantic))
}

/// Insert a variable into the current local scope.
/// Returns true if the variable name is not already taken, else false.
/// If the name is already taken, the variable is still inserted (= shadowing).
fn semantic_insert_variable(
    semantic: &mut Semantic,
    name: String,
    variable_type: RAstType,
    mutable: bool,
) -> bool {
    stringMapStack_insert::<Variable>(
        semantic_locals_mut(semantic),
        name,
        Variable::Variable(variable_type, mutable),
    )
}

/// Local variable entry.
enum Variable {
    /// type, is mutable
    Variable(RAstType, bool),
}

/// A global item, i.e. either a function or an enum.
enum Item {
    Function(FnSignature),
    Enum(RAstEnum),
}

/// A type that represents the (type) signature of a function.
enum FnSignature {
    /// parameter types, return type, is unsafe
    Fn(Vec<RAstType>, RAstType, bool),
}

/// Different operations that can be done when casting a value.
///
/// ZeroExtend: A type with smaller bitwidth is zero-extended to a larger bitwidth.
/// Truncate: A type with larger bitwidth is truncated to a smaller bitwidth.
/// None: Do not perform a cast (cast would be a no-op which would be illegal in LLVM-IR).
/// Invalid: The cast is illegal.
enum CastOperation {
    /// A type with smaller bitwidth is zero-extended to a larger bitwidth.
    ZeroExtend,
    /// A type with larger bitwidth is truncated to a smaller bitwidth.
    Truncate,
    /// Cast integer to pointer.
    IntToPtr,
    /// Cast pointer to integer.
    PtrToInt,
    /// Do not perform a cast.
    None,
    /// The cast is illegal.
    Invalid,
}

/// Return the CastOperation that is applicable from `left_type` to `right_type` for Rust AST types.
/// See documentation of CastOperation for more details.
fn castOperation_get_cast_operation(left_type: &RAstType, right_type: &RAstType) -> CastOperation {
    if rAstType_eq(left_type, right_type) {
        return CastOperation::None;
    }

    match left_type {
        RAstType::U8 => match right_type {
            RAstType::Usize => CastOperation::ZeroExtend,
            RAstType::Char => CastOperation::None,
            _ => CastOperation::Invalid,
        },
        RAstType::Usize => match right_type {
            RAstType::U8 => CastOperation::Truncate,
            RAstType::RawPointerMut(_) => CastOperation::IntToPtr,
            _ => CastOperation::Invalid,
        },
        RAstType::Bool => match right_type {
            RAstType::U8 | RAstType::Usize => CastOperation::ZeroExtend,
            _ => CastOperation::Invalid,
        },
        RAstType::Char => match right_type {
            RAstType::Usize => CastOperation::ZeroExtend,
            RAstType::U8 => CastOperation::None,
            _ => CastOperation::Invalid,
        },
        RAstType::Reference(left_inner, _) => match right_type {
            RAstType::RawPointerMut(right_inner) => {
                if rAstType_eq(
                    box_deref::<RAstType>(left_inner),
                    box_deref::<RAstType>(right_inner),
                ) {
                    CastOperation::None
                } else {
                    CastOperation::Invalid
                }
            },
            _ => CastOperation::Invalid,
        },
        RAstType::RawPointerMut(_) => match right_type {
            RAstType::RawPointerMut(_) => CastOperation::None,
            RAstType::Usize => CastOperation::PtrToInt,
            _ => CastOperation::Invalid,
        },
        _ => CastOperation::Invalid,
    }
}

// -----------------------------------------------------------------
// --------------------- Semantic Analysis -------------------------
// -----------------------------------------------------------------

/// Run semantic analysis on the full AST.
// TODO: check duplicate functions/enums
fn semantic_check_language(semantic: &mut Semantic, ast: &RAst) {
    let RAst::Language(items): &RAst = ast;
    let mut i: usize = 0;
    let len: usize = vec_len::<RAstItem>(items);
    while i < len {
        let item: &RAstItem = vec_at::<RAstItem>(items, i);
        match item {
            RAstItem::Function(function) => semantic_check_function(semantic, function),
            _ => {}, // TODO: enum/extern checking
        }
        i = i + 1;
    }
}

/// Analyze one function and validate body against its signature.
fn semantic_check_function(semantic: &mut Semantic, function: &RAstFunction) {
    let RAstFunction::Function(is_unsafe, _, parameters, return_type, body): &RAstFunction =
        function;

    semantic_set_current_fn_return_type(semantic, rAstType_clone(return_type));
    semantic_enter_scope(semantic);

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstVariable>(parameters);
    while i < len {
        let RAstVariable::Variable(pattern, parameter_type): &RAstVariable =
            vec_at::<RAstVariable>(parameters, i);

        match pattern {
            RAstPattern::Identifier(is_mutable, name) => {
                let already_used: bool = semantic_insert_variable(
                    semantic,
                    string_clone(name),
                    rAstType_clone(parameter_type),
                    *is_mutable,
                );
                if already_used {
                    semantic_check_error("duplicate parameter name");
                }
            },
            _ => {},
        }
        i = i + 1;
    }

    let block_type: RAstType = semantic_check_block(semantic, body, *is_unsafe);
    semantic_expect_rough_type_match(&block_type, return_type);

    semantic_leave_scope(semantic);
    semantic_set_current_fn_return_type(semantic, RAstType::Unit);
}

/// Analyze one block and return its resulting type.
fn semantic_check_block(semantic: &mut Semantic, block: &RAstBlock, is_unsafe: bool) -> RAstType {
    let RAstBlock::Block(statements, tail): &RAstBlock = block;
    if is_unsafe {
        semantic_push_unsafe_context(semantic);
    }
    semantic_enter_scope(semantic);

    let mut statement_flow_type: RAstType = RAstType::Unit;
    let mut i: usize = 0;
    let len: usize = vec_len::<RAstStatement>(statements);
    while i < len {
        let statement: &RAstStatement = vec_at::<RAstStatement>(statements, i);
        match statement {
            RAstStatement::Let(variable, value) => {
                semantic_check_binding(semantic, variable, box_deref::<RAstExpr>(value));
            },
            RAstStatement::Expression(expression) => {
                let ty: RAstType =
                    semantic_check_expression(semantic, box_deref::<RAstExpr>(expression));
                if rAstType_eq(&ty, &RAstType::Never) {
                    statement_flow_type = RAstType::Never;
                }
            },
        }
        i = i + 1;
    }

    let mut block_type: RAstType = match tail {
        Option::Some(expression) => {
            semantic_check_expression(semantic, box_deref::<RAstExpr>(expression))
        },
        Option::None => RAstType::Unit,
    };

    if rAstType_eq(&statement_flow_type, &RAstType::Never) {
        block_type = RAstType::Never;
    }

    if is_unsafe {
        semantic_pop_unsafe_context(semantic);
    }
    semantic_leave_scope(semantic);
    block_type
}

/// Analyze one let-binding statement.
fn semantic_check_binding(semantic: &mut Semantic, variable: &RAstVariable, value: &RAstExpr) {
    let RAstVariable::Variable(pattern, binding_type): &RAstVariable = variable;
    let actual_type: RAstType = semantic_check_expression(semantic, value);
    semantic_expect_exact_type_match(binding_type, &actual_type);

    match pattern {
        RAstPattern::Identifier(is_mutable, lvalue_name) => {
            // allow shadowing of variables
            let _ = semantic_insert_variable(
                semantic,
                string_clone(lvalue_name),
                rAstType_clone(binding_type),
                *is_mutable,
            );
        },
        _ => {},
    }
}

/// Analyze one expression and return its type.
fn semantic_check_expression(semantic: &mut Semantic, expression: &RAstExpr) -> RAstType {
    match expression {
        RAstExpr::Return(returned) => semantic_check_return(semantic, returned),
        RAstExpr::Assign(left, right) => semantic_check_assignment(
            semantic,
            box_deref::<RAstExpr>(left),
            box_deref::<RAstExpr>(right),
        ),
        RAstExpr::Binary(operator, left, right) => semantic_check_binary_op(
            semantic,
            operator,
            box_deref::<RAstExpr>(left),
            box_deref::<RAstExpr>(right),
        ),
        RAstExpr::Cast(value, to_type) => {
            semantic_check_cast(semantic, box_deref::<RAstExpr>(value), to_type)
        },
        RAstExpr::Unary(operator, value) => {
            semantic_check_unary_op(semantic, operator, box_deref::<RAstExpr>(value))
        },
        RAstExpr::Literal(literal) => rastLiteral_type(literal),
        RAstExpr::VariableUse(name) => semantic_check_variable_use(semantic, name),
        RAstExpr::Call(callee, arguments) => semantic_check_call(semantic, callee, arguments),
        RAstExpr::Block(is_unsafe, block) => semantic_check_block(semantic, block, *is_unsafe),
        RAstExpr::If(if_expression) => semantic_check_if(semantic, if_expression),
        RAstExpr::While(condition, body) => {
            semantic_check_while(semantic, box_deref::<RAstExpr>(condition), body)
        },
        RAstExpr::Match(value, arms) => {
            semantic_check_match(semantic, box_deref::<RAstExpr>(value), arms)
        },
    }
}

fn semantic_check_return(semantic: &mut Semantic, returned: &Option<Box<RAstExpr>>) -> RAstType {
    match returned {
        Option::Some(expression) => {
            let ty: RAstType =
                semantic_check_expression(semantic, box_deref::<RAstExpr>(expression));
            semantic_expect_exact_type_match(&ty, semantic_current_fn_return_type(semantic));
        },
        Option::None => {
            semantic_expect_exact_type_match(
                &RAstType::Unit,
                semantic_current_fn_return_type(semantic),
            );
        },
    }
    RAstType::Never
}

fn semantic_check_assignment(
    semantic: &mut Semantic,
    left: &RAstExpr,
    right: &RAstExpr,
) -> RAstType {
    let right_type: RAstType = semantic_check_expression(semantic, right);
    let left_type: RAstType = semantic_check_assignment_lvalue_type(semantic, left);
    semantic_expect_exact_type_match(&left_type, &right_type);
    RAstType::Unit
}

fn semantic_check_assignment_lvalue_type(
    semantic: &mut Semantic,
    expression: &RAstExpr,
) -> RAstType {
    match expression {
        RAstExpr::VariableUse(name) => match semantic_lookup_variable(semantic, name) {
            Option::Some(Variable::Variable(variable_type, mutable)) => {
                if not(mutable) {
                    semantic_check_error("invalid assignment to immutable variable");
                }
                variable_type
            },
            Option::None => semantic_check_error("undefined variable"),
        },
        RAstExpr::Unary(RAstUnaryOp::Dereference, value) => {
            let pointer_type: RAstType =
                semantic_check_expression(semantic, box_deref::<RAstExpr>(value));
            match pointer_type {
                RAstType::Reference(inner, mutable) => {
                    if not(mutable) {
                        semantic_check_error("invalid assignment using immutable reference");
                    }
                    rAstType_clone(box_deref::<RAstType>(&inner))
                },
                RAstType::RawPointerMut(inner) => {
                    if not(semantic_is_unsafe_context(semantic)) {
                        semantic_check_error("raw pointer dereference requires unsafe");
                    }
                    rAstType_clone(box_deref::<RAstType>(&inner))
                },
                _ => semantic_check_error("invalid assignment to an expression"),
            }
        },
        _ => semantic_check_error("invalid assignment target"),
    }
}

fn semantic_check_binary_op(
    semantic: &mut Semantic,
    operator: &RAstBinaryOp,
    left: &RAstExpr,
    right: &RAstExpr,
) -> RAstType {
    let left_type: RAstType = semantic_check_expression(semantic, left);
    let right_type: RAstType = semantic_check_expression(semantic, right);
    semantic_expect_exact_type_match(&left_type, &right_type);

    match operator {
        RAstBinaryOp::Arithmetic(_) => {
            semantic_expect_numeric_type(&left_type);
            left_type
        },
        RAstBinaryOp::Comparison(_) => RAstType::Bool,
    }
}

fn semantic_check_cast(semantic: &mut Semantic, value: &RAstExpr, to_type: &RAstType) -> RAstType {
    let from_type: RAstType = semantic_check_expression(semantic, value);
    match castOperation_get_cast_operation(&from_type, to_type) {
        CastOperation::Invalid => semantic_check_error("invalid cast"),
        _ => rAstType_clone(to_type),
    }
}

fn semantic_check_unary_op(
    semantic: &mut Semantic,
    operator: &RAstUnaryOp,
    value: &RAstExpr,
) -> RAstType {
    match operator {
        RAstUnaryOp::Reference(mutable_ref) => match value {
            RAstExpr::VariableUse(name) => match semantic_lookup_variable(semantic, name) {
                Option::Some(Variable::Variable(ty, mutable_var)) => {
                    if and(*mutable_ref, not(mutable_var)) {
                        semantic_check_error("cannot take mutable reference to immutable variable");
                    }
                    RAstType::Reference(box_new::<RAstType>(ty), *mutable_ref)
                },
                _ => semantic_check_error("undefined variable"),
            },
            _ => {
                let ty: RAstType = semantic_check_expression(semantic, value);
                RAstType::Reference(box_new::<RAstType>(ty), *mutable_ref)
            },
        },
        RAstUnaryOp::Dereference => {
            let ty: RAstType = semantic_check_expression(semantic, value);
            match ty {
                RAstType::Reference(pointed, _) => rAstType_clone(box_deref::<RAstType>(&pointed)),
                RAstType::RawPointerMut(pointed) => {
                    if not(semantic_is_unsafe_context(semantic)) {
                        semantic_check_error("raw pointer dereference requires unsafe context");
                    }
                    rAstType_clone(box_deref::<RAstType>(&pointed))
                },
                _ => semantic_check_error("cannot dereference this expression"),
            }
        },
    }
}

fn semantic_check_variable_use(semantic: &mut Semantic, name: &String) -> RAstType {
    match semantic_lookup_variable(semantic, name) {
        Option::Some(Variable::Variable(ty, _)) => ty,
        _ => semantic_check_error("undefined variable"),
    }
}

fn semantic_check_call(
    semantic: &mut Semantic,
    callee: &RAstPath,
    arguments: &Vec<RAstExpr>,
) -> RAstType {
    let function_name: String = rAstPath_to_string(callee);

    let FnSignature::Fn(parameter_types, return_type, is_unsafe): FnSignature =
        match semantic_lookup_function_signature(semantic, &function_name) {
            Option::Some(signature) => signature,
            _ => semantic_check_error("call to undefined function"),
        };

    if and(is_unsafe, not(semantic_is_unsafe_context(semantic))) {
        semantic_check_error("calling an unsafe function requires unsafe");
    }

    let mut i: usize = 0;
    while i < vec_len::<RAstExpr>(arguments) {
        let argument: &RAstExpr = vec_at::<RAstExpr>(arguments, i);
        let arg_type: RAstType = semantic_check_expression(semantic, argument);

        match vec_get::<RAstType>(&parameter_types, i) {
            Option::Some(ty) => {
                semantic_expect_exact_type_match(ty, &arg_type);
            },
            _ => {
                semantic_check_error("function call has more arguments than there are parameters");
            },
        }

        i = i + 1;
    }

    return_type
}

fn semantic_check_if(semantic: &mut Semantic, if_expression: &RAstIf) -> RAstType {
    let RAstIf::If(condition, then_block, else_branch): &RAstIf = if_expression;
    let condition_type: RAstType =
        semantic_check_expression(semantic, box_deref::<RAstExpr>(condition));
    semantic_expect_bool_type(&condition_type);

    let then_type: RAstType = semantic_check_block(semantic, then_block, false);
    match else_branch {
        Option::Some(else_branch) => {
            let else_type: RAstType = match else_branch {
                RAstElse::If(nested_if) => {
                    semantic_check_if(semantic, box_deref::<RAstIf>(nested_if))
                },
                RAstElse::Block(block) => semantic_check_block(semantic, block, false),
            };
            semantic_expect_rough_type_match(&then_type, &else_type);

            rAstType_coerce(then_type, else_type)
        },
        Option::None => RAstType::Unit,
    }
}

fn semantic_check_while(
    semantic: &mut Semantic,
    condition: &RAstExpr,
    body: &RAstBlock,
) -> RAstType {
    let condition_type: RAstType = semantic_check_expression(semantic, condition);
    semantic_expect_bool_type(&condition_type);
    let body_type: RAstType = semantic_check_block(semantic, body, false);
    semantic_expect_rough_type_match(&RAstType::Unit, &body_type);
    RAstType::Unit
}

fn semantic_check_match(
    semantic: &mut Semantic,
    value: &RAstExpr,
    arms: &Vec<RAstArm>,
) -> RAstType {
    if vec_len::<RAstArm>(arms) == 0 {
        semantic_check_error("match requires at least one arm");
    }

    let expr_type: RAstType = semantic_check_expression(semantic, value);
    let mut return_type: RAstType = RAstType::Never;

    let mut i: usize = 0;
    while i < vec_len::<RAstArm>(arms) {
        let arm: &RAstArm = vec_at::<RAstArm>(arms, i);
        let RAstArm::Arm(patterns, expression): &RAstArm = arm;

        let mut j: usize = 0;
        while j < vec_len::<RAstPattern>(patterns) {
            let pattern: &RAstPattern = vec_at::<RAstPattern>(patterns, j);

            if vec_len::<RAstPattern>(patterns) > 1 {
                match pattern {
                    RAstPattern::Literal(_) => {},
                    _ => {
                        semantic_check_error(
                            "multi-pattern match arms only support literal patterns",
                        );
                    },
                }
            }

            semantic_check_pattern(pattern, &expr_type);
            j = j + 1;
        }

        let arm_type: RAstType = semantic_check_expression(semantic, expression);
        semantic_expect_rough_type_match(&return_type, &arm_type);

        return_type = rAstType_coerce(return_type, arm_type);
        i = i + 1;
    }
    return_type
}

fn semantic_check_pattern(pattern: &RAstPattern, expression_type: &RAstType) {
    let pattern_type: RAstType = match pattern {
        RAstPattern::Literal(literal) => match literal {
            RAstPatternLiteral::Int(_) => {
                if rAstType_is_numeric(expression_type) {
                    return; // numeric expression matches on numeric pattern
                } else {
                    RAstType::Usize
                }
            },
            RAstPatternLiteral::Char(_) => RAstType::Char,
            RAstPatternLiteral::Bool(_) => RAstType::Bool,
        },
        RAstPattern::Identifier(_, _) | RAstPattern::Wildcard => return, // type agnostic
        RAstPattern::EnumVariant(enum_name, _, _) => RAstType::Custom(string_clone(enum_name)),
    };

    semantic_expect_exact_type_match(&pattern_type, &expression_type);
}

// -----------------------------------------------------------------
// ---------------------- Code Generation --------------------------
// -----------------------------------------------------------------

/// Type that encapsulates the state during LLVM-IR code generation from an AST.
enum Codegen {
    /// llvm code, is main function, SSA numbering counter, local variable slots, global items
    Codegen(Code, bool, usize, StringMapStack<STPair>, StringMap<Item>),
}

fn codegen_new(items: StringMap<Item>) -> Codegen {
    Codegen::Codegen(code_new(), false, 0, stringMapStack_new::<STPair>(), items)
}

/// Get a shared reference to the code.
fn codegen_code(Codegen::Codegen(code, _, _, _, _): &Codegen) -> &Code {
    code
}

/// Get a mutable reference to the code.
fn codegen_code_mut(Codegen::Codegen(code, _, _, _, _): &mut Codegen) -> &mut Code {
    code
}

/// Marks the current function as the main function.
fn codegen_mark_as_main(codegen: &mut Codegen, is_main_function: bool) {
    let Codegen::Codegen(_, is_main, _, _, _): &mut Codegen = codegen;
    *is_main = is_main_function;
}

/// Return true if the current function is the main function.
fn codegen_is_main(Codegen::Codegen(_, is_main, _, _, _): &Codegen) -> bool {
    *is_main
}

/// Push a new empty scope onto the stack.
fn codegen_push_scope(Codegen::Codegen(_, _, _, stack, _): &mut Codegen) {
    stringMapStack_push_empty::<STPair>(stack);
}

/// Pop the last pushed scope.
fn codegen_pop_scope(Codegen::Codegen(_, _, _, stack, _): &mut Codegen) -> bool {
    stringMapStack_pop::<STPair>(stack)
}

/// Insert one variable slot into the current scope.
fn codegen_scope_insert(codegen: &mut Codegen, name: String, ty: RAstType, pointer_name: String) {
    let Codegen::Codegen(_, _, _, stack, _): &mut Codegen = codegen;
    let _ = stringMapStack_insert::<STPair>(stack, name, STPair::ST(pointer_name, ty));
}

/// Lookup variable slot information.
fn codegen_scope_lookup(Codegen::Codegen(_, _, _, stack, _): &Codegen, name: &String) -> STPair {
    match stringMapStack_lookup::<STPair>(stack, name) {
        Option::Some(variable) => stPair_clone(variable),
        Option::None => STPair::ST(string_new(), RAstType::Unit), // should not be reachable
    }
}

/// Lookup one function signature.
fn codegen_function_signature(codegen: &Codegen, name: &String) -> Option<FnSignature> {
    let Codegen::Codegen(_, _, _, _, items): &Codegen = codegen;
    match stringMap_get::<Item>(items, name) {
        Option::Some(Item::Function(signature)) => Option::Some(fnSignature_clone(signature)),
        _ => Option::None,
    }
}

/// Get the current value of the SSA numbering scheme.
fn codegen_ssa_counter(Codegen::Codegen(_, _, counter, _, _): &Codegen) -> usize {
    *counter
}

/// Increment the SSA numbering value by one.
fn codegen_increment_ssa_counter(Codegen::Codegen(_, _, counter, _, _): &mut Codegen) {
    *counter = *counter + 1;
}

/// Get a unique virtual register name.
fn codegen_next_register(codegen: &mut Codegen) -> String {
    let id: usize = codegen_ssa_counter(codegen);
    codegen_increment_ssa_counter(codegen);
    let mut name: String = string("%t");
    string_push_string(&mut name, &integer_to_string(id));
    name
}

/// Get a unique basic block label with a given suffix.
fn codegen_next_label(codegen: &mut Codegen, suffix: &str) -> String {
    let id: usize = codegen_ssa_counter(codegen);
    codegen_increment_ssa_counter(codegen);
    let mut label: String = string("l");
    string_push_string(&mut label, &integer_to_string(id));
    string_push(&mut label, '.');
    string_push_str(&mut label, suffix);
    label
}

/// Pair that contains a String and a Rust Type
enum STPair {
    ST(String, RAstType),
}

/// Emit LLVM-IR for a full Rust AST.
fn codegen_language(codegen: &mut Codegen, ast: &RAst) {
    let RAst::Language(items): &RAst = ast;
    let mut i: usize = 0;
    let len: usize = vec_len::<RAstItem>(items);
    while i < len {
        let item: &RAstItem = vec_at::<RAstItem>(items, i);
        match item {
            RAstItem::ExternBlock(block) => codegen_extern_block(codegen, block),
            RAstItem::Enum(enum_item) => codegen_enum(codegen, enum_item),
            RAstItem::Function(function) => codegen_function(codegen, function),
        }
        i = i + 1;
    }
}

/// Emit LLVM-IR for one extern block.
fn codegen_extern_block(codegen: &mut Codegen, functions: &Vec<RAstExternFunction>) {
    let mut i: usize = 0;
    while i < vec_len::<RAstExternFunction>(functions) {
        let function: &RAstExternFunction = vec_at::<RAstExternFunction>(functions, i);
        let RAstExternFunction::ExternFunction(name, parameters, return_type): &RAstExternFunction =
            function;
        codegen_emit_declare(codegen, name, parameters, return_type);
        i = i + 1;
    }
}

/// Emit LLVM-IR for one enum definition.
fn codegen_enum(_codegen: &mut Codegen, _enum_item: &RAstEnum) {
    // TODO:
}

/// Emit LLVM-IR for one function definition.
fn codegen_function(codegen: &mut Codegen, function: &RAstFunction) {
    let RAstFunction::Function(_, function_name, parameters, return_type, body): &RAstFunction =
        function;

    let llvm_return_type: String = if string_eq(function_name, &string("main")) {
        codegen_mark_as_main(codegen, true);

        if rAstType_eq(&return_type, &RAstType::Unit) {
            string("i64")
        } else {
            rAstType_to_llvm_name(&return_type)
        }
    } else {
        rAstType_to_llvm_name(&return_type)
    };

    codegen_emit_function_header(codegen, function_name, &llvm_return_type, parameters);

    codegen_push_scope(codegen);
    let mut parameter_index: usize = 0;
    while parameter_index < vec_len::<RAstVariable>(parameters) {
        let RAstVariable::Variable(pattern, param_type): &RAstVariable =
            vec_at::<RAstVariable>(parameters, parameter_index);

        match pattern {
            RAstPattern::Identifier(_, name) => {
                // SSA: all variables (including parameters) are stored on the stack
                let param_ptr: String = codegen_emit_alloca(codegen, param_type, 1);
                let mut param_register: String = string("%");
                string_push_string(&mut param_register, name);
                codegen_emit_store(codegen, param_type, &param_register, &param_ptr);

                let name: String = string_clone(name);
                codegen_scope_insert(codegen, name, rAstType_clone(param_type), param_ptr);
            },
            _ => {},
        }

        parameter_index = parameter_index + 1;
    }

    let STPair::ST(value_name, block_type): STPair = codegen_block(codegen, body);
    match &return_type {
        RAstType::Unit | RAstType::Never => {
            if codegen_is_main(codegen) {
                // exit with success
                codegen_emit_ret_value(codegen, &RAstType::Usize, &integer_to_string(0));
            } else {
                codegen_emit_ret_void(codegen);
            }
        },
        _ => {
            if rAstType_eq(&block_type, &RAstType::Never) {
                // return dummy value, it is never reached anyway
                codegen_emit_ret_value(codegen, &return_type, &integer_to_string(0));
            } else {
                codegen_emit_ret_value(codegen, &return_type, &value_name);
            }
        },
    }
    codegen_emit_line(codegen, string("}"));

    codegen_mark_as_main(codegen, false);
    codegen_pop_scope(codegen);
}

/// Emit LLVM-IR for one block expression.
fn codegen_block(codegen: &mut Codegen, block: &RAstBlock) -> STPair {
    let RAstBlock::Block(statements, tail): &RAstBlock = block;
    codegen_push_scope(codegen);

    let mut i: usize = 0;
    let mut block_type: RAstType = RAstType::Unit;
    while i < vec_len::<RAstStatement>(statements) {
        let statement: &RAstStatement = vec_at::<RAstStatement>(statements, i);
        match statement {
            RAstStatement::Let(variable, value) => {
                codegen_binding(codegen, variable, box_deref::<RAstExpr>(value));
            },

            RAstStatement::Expression(expression) => {
                // expression is only used for its side-effects, so we can discard the result
                let STPair::ST(_, ty): STPair =
                    codegen_expression(codegen, box_deref::<RAstExpr>(expression));

                if rAstType_eq(&ty, &RAstType::Never) {
                    // the rest of the block becomes unreachable, so the block type becomes Never
                    block_type = RAstType::Never;
                }
            },
        }
        i = i + 1;
    }

    let STPair::ST(name, mut ty) = match tail {
        Option::Some(expression) => codegen_expression(codegen, box_deref::<RAstExpr>(expression)),
        Option::None => STPair::ST(string_new(), RAstType::Unit),
    };

    if rAstType_eq(&block_type, &RAstType::Never) {
        // set type of block to Never to indicate that it doesn't return normally
        ty = RAstType::Never;
    }

    codegen_pop_scope(codegen);
    STPair::ST(name, ty)
}

/// Emit LLVM-IR for one let binding.
fn codegen_binding(codegen: &mut Codegen, variable: &RAstVariable, value: &RAstExpr) {
    let RAstVariable::Variable(pattern, binding_type): &RAstVariable = variable;

    let STPair::ST(rvalue_name, _): STPair = codegen_expression(codegen, value);

    match pattern {
        RAstPattern::Identifier(_, lvalue_name) => {
            if type_has_value(binding_type) {
                let lvalue_pointer: String = codegen_emit_alloca(codegen, binding_type, 1);
                codegen_emit_store(codegen, binding_type, &rvalue_name, &lvalue_pointer);

                let name: String = string_clone(lvalue_name);
                codegen_scope_insert(codegen, name, rAstType_clone(binding_type), lvalue_pointer);
            }
        },
        _ => {},
    }
}

/// Emit LLVM-IR for one expression and return the resulting value/type pair.
fn codegen_expression(codegen: &mut Codegen, expression: &RAstExpr) -> STPair {
    match expression {
        RAstExpr::Return(returned) => codegen_return(codegen, returned),
        RAstExpr::Assign(left, right) => codegen_assignment(
            codegen,
            box_deref::<RAstExpr>(left),
            box_deref::<RAstExpr>(right),
        ),
        RAstExpr::Binary(operator, left, right) => codegen_binary_op(
            codegen,
            operator,
            box_deref::<RAstExpr>(left),
            box_deref::<RAstExpr>(right),
        ),
        RAstExpr::Cast(value, to_type) => {
            codegen_cast(codegen, box_deref::<RAstExpr>(value), to_type)
        },
        RAstExpr::Unary(operator, value) => {
            codegen_unary_op(codegen, operator, box_deref::<RAstExpr>(value))
        },
        RAstExpr::Literal(literal) => codegen_literal(literal),
        RAstExpr::VariableUse(name) => codegen_variable_use(codegen, name),
        RAstExpr::Call(callee, arguments) => codegen_call(codegen, callee, arguments),
        RAstExpr::Block(_, block) => codegen_block(codegen, block),
        RAstExpr::If(if_expression) => codegen_if(codegen, if_expression),
        RAstExpr::While(condition, body) => {
            codegen_while(codegen, box_deref::<RAstExpr>(condition), body)
        },
        RAstExpr::Match(value, arms) => codegen_match(codegen, box_deref::<RAstExpr>(value), arms),
    }
}

/// Emit LLVM-IR for a return expression.
/// `return` always evaluates to type Never.
fn codegen_return(codegen: &mut Codegen, returned: &Option<Box<RAstExpr>>) -> STPair {
    match returned {
        // return <expression>
        Option::Some(expression) => {
            let STPair::ST(name, ty): STPair =
                codegen_expression(codegen, box_deref::<RAstExpr>(expression));
            codegen_emit_ret_value(codegen, &ty, &name);
        },

        // return;
        Option::None => {
            if codegen_is_main(codegen) {
                codegen_emit_ret_value(codegen, &RAstType::Usize, &string("0"));
            } else {
                codegen_emit_ret_void(codegen);
            }
        },
    }

    STPair::ST(string_new(), RAstType::Never)
}

/// Emit LLVM-IR for an assignment expression.
fn codegen_assignment(codegen: &mut Codegen, left: &RAstExpr, right: &RAstExpr) -> STPair {
    let STPair::ST(right_name, _): STPair = codegen_expression(codegen, right);
    let STPair::ST(pointer_name, left_type): STPair = codegen_assignment_lvalue(codegen, left);

    codegen_emit_store(codegen, &left_type, &right_name, &pointer_name);
    STPair::ST(right_name, RAstType::Unit)
}

fn codegen_assignment_lvalue(codegen: &mut Codegen, expression: &RAstExpr) -> STPair {
    match expression {
        RAstExpr::VariableUse(name) => codegen_scope_lookup(codegen, name),

        RAstExpr::Unary(RAstUnaryOp::Dereference, value) => {
            let STPair::ST(pointer_name, pointer_type): STPair =
                codegen_expression(codegen, box_deref::<RAstExpr>(value));

            match pointer_type {
                RAstType::Reference(inner, _) => {
                    let ty: RAstType = rAstType_clone(box_deref::<RAstType>(&inner));
                    STPair::ST(pointer_name, ty)
                },
                RAstType::RawPointerMut(inner) => {
                    let ty: RAstType = rAstType_clone(box_deref::<RAstType>(&inner));
                    STPair::ST(pointer_name, ty)
                },
                _ => STPair::ST(string_new(), RAstType::Unit), // should not be reachable
            }
        },
        _ => STPair::ST(string_new(), RAstType::Unit), // should not be reachable
    }
}

/// Emit LLVM-IR for a binary expression.
fn codegen_binary_op(
    codegen: &mut Codegen,
    operator: &RAstBinaryOp,
    left: &RAstExpr,
    right: &RAstExpr,
) -> STPair {
    let STPair::ST(left_name, op_type): STPair = codegen_expression(codegen, left);
    let STPair::ST(right_name, _): STPair = codegen_expression(codegen, right);

    match operator {
        RAstBinaryOp::Arithmetic(op) => {
            let name: String = codegen_emit_binary(codegen, op, &op_type, &left_name, &right_name);
            STPair::ST(name, op_type)
        },
        RAstBinaryOp::Comparison(op) => {
            let name: String = codegen_emit_icmp(codegen, op, &op_type, &left_name, &right_name);
            STPair::ST(name, RAstType::Bool)
        },
    }
}

/// Emit LLVM-IR for a cast expression.
fn codegen_cast(codegen: &mut Codegen, value: &RAstExpr, to_type: &RAstType) -> STPair {
    let STPair::ST(from_name, from_type): STPair = codegen_expression(codegen, value);
    let to_type: RAstType = rAstType_clone(to_type);

    match castOperation_get_cast_operation(&from_type, &to_type) {
        CastOperation::ZeroExtend => {
            let name: String = codegen_emit_zext(codegen, &from_type, &to_type, &from_name);
            STPair::ST(name, to_type)
        },
        CastOperation::Truncate => {
            let name: String = codegen_emit_trunc(codegen, &from_type, &to_type, &from_name);
            STPair::ST(name, to_type)
        },
        CastOperation::IntToPtr => {
            let name: String = codegen_emit_inttoptr(codegen, &from_type, &to_type, &from_name);
            STPair::ST(name, to_type)
        },
        CastOperation::PtrToInt => {
            let name: String = codegen_emit_ptrtoint(codegen, &from_type, &to_type, &from_name);
            STPair::ST(name, to_type)
        },
        CastOperation::None => STPair::ST(from_name, to_type),
        CastOperation::Invalid => STPair::ST(from_name, to_type), // should be unreachable
    }
}

/// Emit LLVM-IR for a unary expression.
fn codegen_unary_op(codegen: &mut Codegen, operator: &RAstUnaryOp, value: &RAstExpr) -> STPair {
    match operator {
        RAstUnaryOp::Reference(mutable_ref) => match value {
            RAstExpr::VariableUse(name) => {
                let STPair::ST(pointer_name, ty): STPair = codegen_scope_lookup(codegen, name);
                STPair::ST(
                    pointer_name,
                    RAstType::Reference(box_new::<RAstType>(ty), *mutable_ref),
                )
            },
            _ => {
                let STPair::ST(name, ty): STPair = codegen_expression(codegen, value);
                let reference: String = codegen_emit_alloca(codegen, &ty, 1);
                codegen_emit_store(codegen, &ty, &name, &reference);
                STPair::ST(
                    reference,
                    RAstType::Reference(box_new::<RAstType>(ty), *mutable_ref),
                )
            },
        },

        RAstUnaryOp::Dereference => {
            let STPair::ST(name, ty): STPair = codegen_expression(codegen, value);
            let inner_type: RAstType = match ty {
                RAstType::Reference(pointed, _) => rAstType_clone(box_deref::<RAstType>(&pointed)),
                RAstType::RawPointerMut(pointed) => rAstType_clone(box_deref::<RAstType>(&pointed)),
                _ => RAstType::Unit, // should be unreachable
            };
            let name: String = codegen_emit_load(codegen, &inner_type, &name);
            STPair::ST(name, inner_type)
        },
    }
}

/// Emit LLVM-IR for a literal expression.
fn codegen_literal(literal: &RAstLiteral) -> STPair {
    match literal {
        RAstLiteral::Int(value) => STPair::ST(integer_to_string(*value), RAstType::Usize),
        RAstLiteral::Char(value) => STPair::ST(integer_to_string(*value as usize), RAstType::Char),
        RAstLiteral::Bool(value) => STPair::ST(integer_to_string(*value as usize), RAstType::Bool),
        RAstLiteral::String(_) => STPair::ST(
            string_new(),
            RAstType::Reference(box_new::<RAstType>(RAstType::Custom(string("str"))), false),
        ),
    }
}

/// Emit LLVM-IR for a variable-use expression.
fn codegen_variable_use(codegen: &mut Codegen, variable_name: &String) -> STPair {
    let STPair::ST(pointer_name, ty): STPair = codegen_scope_lookup(codegen, variable_name);
    if type_has_value(&ty) {
        let value_name: String = codegen_emit_load(codegen, &ty, &pointer_name);
        STPair::ST(value_name, ty)
    } else {
        // Unit and Never have no value, so don't load
        STPair::ST(string_new(), ty)
    }
}

/// Emit LLVM-IR for a function call expression.
fn codegen_call(codegen: &mut Codegen, callee: &RAstPath, arguments: &Vec<RAstExpr>) -> STPair {
    let function_name: String = rAstPath_to_string(callee);

    let mut arg_types: Vec<RAstType> = vec_new::<RAstType>();
    let mut arg_values: Vec<String> = vec_new::<String>();
    let mut i: usize = 0;
    while i < vec_len::<RAstExpr>(arguments) {
        let argument: &RAstExpr = vec_at::<RAstExpr>(arguments, i);

        let STPair::ST(arg_value, arg_type): STPair = codegen_expression(codegen, argument);

        vec_push::<RAstType>(&mut arg_types, arg_type);
        vec_push::<String>(&mut arg_values, arg_value);
        i = i + 1;
    }

    match codegen_function_signature(codegen, &function_name) {
        Option::Some(FnSignature::Fn(_, return_type, _)) => {
            let result_name: String = if type_has_value(&return_type) {
                codegen_emit_call_value(
                    codegen,
                    &function_name,
                    &return_type,
                    &arg_types,
                    &arg_values,
                )
            } else {
                codegen_emit_call_void(codegen, &function_name, &arg_types, &arg_values);
                string_new()
            };
            STPair::ST(result_name, return_type)
        },
        Option::None => codegen_error("unknown codegen function"),
    }
}

/// Emit LLVM-IR for an if expression.
fn codegen_if(codegen: &mut Codegen, if_expression: &RAstIf) -> STPair {
    let RAstIf::If(condition, then_block, else_branch): &RAstIf = if_expression;

    let then_label: String = codegen_next_label(codegen, "if.then");
    let else_label: String = codegen_next_label(codegen, "if.else");
    let end_label: String = codegen_next_label(codegen, "if.end");

    let STPair::ST(cond, _): STPair = codegen_expression(codegen, box_deref::<RAstExpr>(condition));

    // Allocate memory for potential result value, though size is still unknown.
    // In the event that the result type is unit, this instruction will be removed later.
    let result_pointer: String = codegen_emit_alloca(codegen, &RAstType::Unit, 1);
    let alloca_idx: usize = codegen_code_last_index(codegen);

    codegen_emit_br_conditional(codegen, &cond, &then_label, &else_label);

    // start of the then block
    codegen_emit_label(codegen, &then_label);

    let STPair::ST(then_value, mut if_type): STPair = codegen_block(codegen, then_block);

    if type_has_value(&if_type) {
        codegen_emit_store(codegen, &if_type, &then_value, &result_pointer);
    }

    // end of then block, so jump to the end
    codegen_emit_br(codegen, &end_label);

    // start of the else block
    codegen_emit_label(codegen, &else_label);

    match else_branch {
        Option::Some(else_branch) => {
            let STPair::ST(else_value, else_type): STPair = match else_branch {
                RAstElse::If(nested_if) => codegen_if(codegen, box_deref::<RAstIf>(nested_if)),
                RAstElse::Block(block) => codegen_block(codegen, block),
            };

            if type_has_value(&else_type) {
                codegen_emit_store(codegen, &else_type, &else_value, &result_pointer);
            }

            if_type = rAstType_coerce(if_type, else_type);
        },
        _ => if_type = RAstType::Unit, // else is implicitly unit, so type of if must be unit
    }

    // end of else block, so jump to the end
    codegen_emit_br(codegen, &end_label);

    // start of the merge block
    codegen_emit_label(codegen, &end_label);

    // load and return the value if there is one
    let result: String = if type_has_value(&if_type) {
        // now we know the type and thus the size to allocate on the stack
        codegen_fixup_alloca(codegen, alloca_idx, &if_type, 1);

        codegen_emit_load(codegen, &if_type, &result_pointer)
    } else {
        codegen_fixup(codegen, alloca_idx, string_new()); // alloca was not needed
        string_new() // no value is returned, so some placeholder
    };

    STPair::ST(result, if_type)
}

/// Emit LLVM-IR for a while expression.
fn codegen_while(codegen: &mut Codegen, condition: &RAstExpr, body: &RAstBlock) -> STPair {
    let entry_label: String = codegen_next_label(codegen, "while.entry");
    let body_label: String = codegen_next_label(codegen, "while.body");
    let end_label: String = codegen_next_label(codegen, "while.end");

    // jump from current block to while-entry block
    codegen_emit_br(codegen, &entry_label);
    // start entry block
    codegen_emit_label(codegen, &entry_label);

    let STPair::ST(condition_name, _): STPair = codegen_expression(codegen, condition);

    // conditionally execute body or skip body
    codegen_emit_br_conditional(codegen, &condition_name, &body_label, &end_label);

    // start body block
    codegen_emit_label(codegen, &body_label);

    codegen_block(codegen, body);

    // jump back to entry to reevaluate condition
    codegen_emit_br(codegen, &entry_label);

    // start block of rest of instructions
    codegen_emit_label(codegen, &end_label);

    STPair::ST(string_new(), RAstType::Unit) // while always returns unit
}

/// Emit LLVM-IR for a match expression.
fn codegen_match(codegen: &mut Codegen, value: &RAstExpr, arms: &Vec<RAstArm>) -> STPair {
    let STPair::ST(expr_name, expr_type): STPair = codegen_expression(codegen, value);

    let end_label: String = codegen_next_label(codegen, "match.end");

    // Allocate memory for potential result value, though size is still unknown.
    // In the event that the result type is unit, this instruction will be removed later.
    let result_pointer: String = codegen_emit_alloca(codegen, &RAstType::Unit, 1);
    let alloca_idx: usize = codegen_code_last_index(codegen);

    let mut return_type: RAstType = RAstType::Never; // still unknown, coercing arm types yields correct type

    let mut i: usize = 0;
    while i < vec_len::<RAstArm>(arms) {
        codegen_push_scope(codegen);

        let is_last_arm: bool = i == vec_len::<RAstArm>(arms) - 1;
        let arm: &RAstArm = vec_at::<RAstArm>(arms, i);

        let arm_type: RAstType = codegen_arm(
            codegen,
            arm,
            is_last_arm,
            &expr_name,
            &expr_type,
            &result_pointer,
            &end_label,
        );

        return_type = rAstType_coerce(return_type, arm_type);
        codegen_pop_scope(codegen);
        i = i + 1;
    }

    // start of the merge block
    codegen_emit_label(codegen, &end_label);

    let result: String = if type_has_value(&return_type) {
        // now we know the type and thus the size to allocate on the stack
        codegen_fixup_alloca(codegen, alloca_idx, &return_type, 1);

        codegen_emit_load(codegen, &return_type, &result_pointer)
    } else {
        codegen_fixup(codegen, alloca_idx, string_new()); // alloca was not needed
        string_new() // no value is returned, so some placeholder
    };

    STPair::ST(result, return_type)
}

/// Generate code for a single match arm.
///
/// * `codegen`: The state of the code generator
/// * `arm`: The arm to generate code for
/// * `is_last_arm`: True, if the given arm is the last arm of the match expression.
/// * `expr_name`: The name of the expression that is being matched on.
/// * `expr_type`: The type of the expression that is being matched on.
/// * `result_pointer`: The name of the pointer to the memory where the match result is stored
/// * `end_label`: The merge-label of the match.
fn codegen_arm(
    codegen: &mut Codegen,
    RAstArm::Arm(patterns, arm_expr): &RAstArm,
    is_last_arm: bool,
    expr_name: &String,
    expr_type: &RAstType,
    result_pointer: &String,
    end_label: &String,
) -> RAstType {
    let arm_label: String = codegen_next_label(codegen, "match.arm");
    let else_label: String = codegen_next_label(codegen, "match.else");

    let mut j: usize = 0;
    while j < vec_len::<RAstPattern>(patterns) {
        let pattern: &RAstPattern = vec_at::<RAstPattern>(patterns, j);
        let is_last_pattern: bool = j == vec_len::<RAstPattern>(patterns) - 1;

        match pattern {
            RAstPattern::Literal(literal) => {
                if not(is_last_arm) {
                    let value: String = integer_to_string(rAstPatternLiteral_value(literal));

                    let cond_name: String = codegen_emit_icmp(
                        codegen,
                        &RAstComparisonOp::Eq,
                        expr_type,
                        expr_name,
                        &value,
                    );

                    let fail_label: String = if is_last_pattern {
                        string_clone(&else_label) // next arm
                    } else {
                        codegen_next_label(codegen, "match.check") // next pattern
                    };

                    codegen_emit_br_conditional(codegen, &cond_name, &arm_label, &fail_label);

                    if not(is_last_pattern) {
                        codegen_emit_label(codegen, &fail_label); // next pattern of arm
                    }
                } // otherwise no branch, arm is executed unconditionally
            },
            RAstPattern::Identifier(_, identifier) => {
                let pointer_name: String = codegen_emit_alloca(codegen, &expr_type, 1);
                codegen_emit_store(codegen, &expr_type, &expr_name, &pointer_name);

                let variable_name: String = string_clone(identifier);
                let variable_type: RAstType = rAstType_clone(&expr_type);
                codegen_scope_insert(codegen, variable_name, variable_type, pointer_name);
            },
            RAstPattern::Wildcard => {},
            RAstPattern::EnumVariant(_, _, _) => {}, // unimplemented
        }
        j = j + 1;
    }

    if not(is_last_arm) {
        codegen_emit_label(codegen, &arm_label);
    }

    let STPair::ST(arm_value, arm_type) = codegen_expression(codegen, arm_expr);
    if type_has_value(&arm_type) {
        codegen_emit_store(codegen, &arm_type, &arm_value, &result_pointer);
    }

    // arm evaluated, so jump to end
    codegen_emit_br(codegen, &end_label);

    if not(is_last_arm) {
        codegen_emit_label(codegen, &else_label);
    }

    arm_type
}

// ---------------------------- Code Emission ---------------------------------

/// The emitted LLVM-IR code.
enum Code {
    /// code lines, cursor index
    Code(Vec<String>),
}

fn code_new() -> Code {
    Code::Code(vec_new::<String>())
}

/// Emit the given string as a new line of LLVM-IR code.
fn codegen_emit_line(codegen: &mut Codegen, line: String) {
    let Code::Code(lines): &mut Code = codegen_code_mut(codegen);
    vec_push::<String>(lines, line);
}

/// Get the line index of the last emitted line.
fn codegen_code_last_index(codegen: &Codegen) -> usize {
    let Code::Code(lines): &Code = codegen_code(codegen);
    vec_len::<String>(lines) - 1
}

/// Fixup the emitted line at index `i` by replacing it with `line`.
fn codegen_fixup(codegen: &mut Codegen, i: usize, line: String) {
    let Code::Code(lines): &mut Code = codegen_code_mut(codegen);
    vec_set(lines, i, line);
}

/// Get the emitted LLVM-IR from Codegen.
fn codegen_into_llvm(Codegen::Codegen(Code::Code(lines), _, _, _, _): Codegen) -> String {
    let mut code: String = string_new();
    let mut i: usize = 0;
    let len: usize = vec_len::<String>(&lines);
    while i < len {
        let line: &String = vec_at::<String>(&lines, i);
        if string_len(line) > 0 {
            string_push_string(&mut code, line);
            string_push(&mut code, '\n');
        }
        i = i + 1;
    }
    code
}

/// Emit a binary instruction of the following form:
/// `name` = `op` `ty` `lhs`,`rhs`
/// where `op` can be one of the following: `add`, `sub`, `mul`, `udiv`, `urem`
/// and return `name`.
fn codegen_emit_binary(
    codegen: &mut Codegen,
    op: &RAstArithmeticOp,
    ty: &RAstType,
    lhs: &String,
    rhs: &String,
) -> String {
    let op_name: &str = match op {
        RAstArithmeticOp::Add => "add",
        RAstArithmeticOp::Sub => "sub",
        RAstArithmeticOp::Mul => "mul",
        RAstArithmeticOp::Div => "udiv",
        RAstArithmeticOp::Rem => "urem",
    };
    let name: String = codegen_next_register(codegen);

    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = ");
    string_push_str(&mut line, op_name);
    string_push(&mut line, ' ');
    string_push_string(&mut line, &rAstType_to_llvm_name(ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, lhs);
    string_push(&mut line, ',');
    string_push_string(&mut line, rhs);

    codegen_emit_line(codegen, line);

    name
}

/// Emit an icmp instruction:
/// `name` = icmp `op` `ty` `lhs`,`rhs`
/// where `op` can be one of the following: `eq`, `ne`, `gt`, `lt`, `ge`, `le`
/// and return `name`.
fn codegen_emit_icmp(
    codegen: &mut Codegen,
    op: &RAstComparisonOp,
    ty: &RAstType,
    lhs: &String,
    rhs: &String,
) -> String {
    let op_name: &str = match op {
        RAstComparisonOp::Eq => "eq",
        RAstComparisonOp::Ne => "ne",
        RAstComparisonOp::Gt => "ugt",
        RAstComparisonOp::Lt => "ult",
        RAstComparisonOp::Ge => "uge",
        RAstComparisonOp::Le => "ule",
    };
    let name: String = codegen_next_register(codegen);

    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = icmp ");
    string_push_str(&mut line, op_name);
    string_push(&mut line, ' ');
    string_push_string(&mut line, &rAstType_to_llvm_name(ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, lhs);
    string_push(&mut line, ',');
    string_push_string(&mut line, rhs);

    codegen_emit_line(codegen, line);

    name
}

/// Emit a ret instruction:
/// ret `ty` `value`
fn codegen_emit_ret_value(codegen: &mut Codegen, ty: &RAstType, value: &String) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_str(&mut line, "ret ");
    string_push_string(&mut line, &rAstType_to_llvm_name(ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, value);

    codegen_emit_line(codegen, line);
}

/// Emit a ret void instruction:
/// ret void
fn codegen_emit_ret_void(codegen: &mut Codegen) {
    codegen_emit_line(codegen, string("  ret void"));
}

/// Emit a label:
/// `label`:
fn codegen_emit_label(codegen: &mut Codegen, label: &String) {
    let mut line: String = string_new();
    string_push(&mut line, '\n');
    string_push_string(&mut line, label);
    string_push(&mut line, ':');

    codegen_emit_line(codegen, line);
}

/// Emit an unconditional branch:
/// br label %`target_label`
fn codegen_emit_br(codegen: &mut Codegen, target_label: &String) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  br label %");
    string_push_string(&mut line, target_label);

    codegen_emit_line(codegen, line);
}

/// Emit a conditional branch:
/// br i1 `condition`, label %`then_label`, label %`else_label`
fn codegen_emit_br_conditional(
    codegen: &mut Codegen,
    condition: &String,
    then_label: &String,
    else_label: &String,
) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  br i1 ");
    string_push_string(&mut line, condition);
    string_push_str(&mut line, ", label %");
    string_push_string(&mut line, then_label);
    string_push_str(&mut line, ", label %");
    string_push_string(&mut line, else_label);

    codegen_emit_line(codegen, line);
}

/// Emit a cast instruction of the following form:
/// `name` = `op` `from_type` `value` to `to_type`
/// where `op` can be one of the following: `zext`, `trunc`
/// and return `name`.
fn codegen_emit_cast(
    codegen: &mut Codegen,
    op: &str,
    from_type: &RAstType,
    to_type: &RAstType,
    value: &String,
) -> String {
    let name: String = codegen_next_register(codegen);

    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = ");
    string_push_str(&mut line, op);
    string_push(&mut line, ' ');
    string_push_string(&mut line, &rAstType_to_llvm_name(from_type));
    string_push(&mut line, ' ');
    string_push_string(&mut line, value);
    string_push_str(&mut line, " to ");
    string_push_string(&mut line, &rAstType_to_llvm_name(to_type));
    string_push(&mut line, '\n');

    codegen_emit_line(codegen, line);

    name
}

/// Emit a zext instruction:
/// `name` = zext `from_type` `value` to `to_type`
/// and return `name`.
fn codegen_emit_zext(
    codegen: &mut Codegen,
    from_type: &RAstType,
    to_type: &RAstType,
    value: &String,
) -> String {
    codegen_emit_cast(codegen, "zext", from_type, to_type, value)
}

/// Emit a trunc instruction:
/// `name` = trunc `from_type` `value` to `to_type`
/// and return `name`.
fn codegen_emit_trunc(
    codegen: &mut Codegen,
    from_type: &RAstType,
    to_type: &RAstType,
    value: &String,
) -> String {
    codegen_emit_cast(codegen, "trunc", from_type, to_type, value)
}

/// Emit an inttoptr instruction:
/// `name` = inttoptr `from_type` `value` to `to_type`
/// and return `name`.
fn codegen_emit_inttoptr(
    codegen: &mut Codegen,
    from_type: &RAstType,
    to_type: &RAstType,
    value: &String,
) -> String {
    codegen_emit_cast(codegen, "inttoptr", from_type, to_type, value)
}

/// Emit a ptrtoint instruction:
/// `name` = ptrtoint `from_type` `value` to `to_type`
/// and return `name`.
fn codegen_emit_ptrtoint(
    codegen: &mut Codegen,
    from_type: &RAstType,
    to_type: &RAstType,
    value: &String,
) -> String {
    codegen_emit_cast(codegen, "ptrtoint", from_type, to_type, value)
}

/// Emit an alloca instruction:
/// `name` = alloca `ty`, i64 `num_elements`
/// and return `name`.
fn codegen_emit_alloca(codegen: &mut Codegen, ty: &RAstType, num_elements: usize) -> String {
    let name: String = codegen_next_register(codegen);

    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = alloca ");
    string_push_string(&mut line, &rAstType_to_llvm_name(ty));
    string_push_str(&mut line, ", i64 ");
    string_push_string(&mut line, &integer_to_string(num_elements));

    codegen_emit_line(codegen, line);

    name
}

/// Emit a store instruction:
/// store `ty` `value`, ptr `pointer`.
fn codegen_emit_store(codegen: &mut Codegen, ty: &RAstType, value: &String, pointer: &String) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  store ");
    string_push_string(&mut line, &rAstType_to_llvm_name(ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, value);
    string_push(&mut line, ',');
    string_push_str(&mut line, " ptr ");
    string_push_string(&mut line, pointer);

    codegen_emit_line(codegen, line);
}

/// Emit a load instruction:
/// `name` = load `ty`, `ptr` pointer`.
fn codegen_emit_load(codegen: &mut Codegen, ty: &RAstType, pointer: &String) -> String {
    let name: String = codegen_next_register(codegen);
    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = load ");
    string_push_string(&mut line, &rAstType_to_llvm_name(ty));
    string_push(&mut line, ',');
    string_push_str(&mut line, " ptr ");
    string_push_string(&mut line, pointer);

    codegen_emit_line(codegen, line);

    name
}

/// Emit a call instruction that returns a value.
fn codegen_emit_call_value(
    codegen: &mut Codegen,
    function_name: &String,
    return_type: &RAstType,
    argument_types: &Vec<RAstType>,
    argument_values: &Vec<String>,
) -> String {
    let name: String = codegen_next_register(codegen);

    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = call ");
    string_push_string(&mut line, &rAstType_to_llvm_name(return_type));
    string_push_str(&mut line, " @");
    string_push_string(&mut line, function_name);
    string_push(&mut line, '(');

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstType>(argument_types);
    while i < len {
        let argument_type: &RAstType = vec_at::<RAstType>(argument_types, i);
        let argument_value: &String = vec_at::<String>(argument_values, i);
        string_push_string(&mut line, &rAstType_to_llvm_name(argument_type));
        string_push(&mut line, ' ');
        string_push_string(&mut line, argument_value);

        i = i + 1;
        if i < len {
            string_push_str(&mut line, ", ");
        }
    }
    string_push_str(&mut line, ")");

    codegen_emit_line(codegen, line);

    name
}

/// Emit a call instruction that returns void.
fn codegen_emit_call_void(
    codegen: &mut Codegen,
    function_name: &String,
    argument_types: &Vec<RAstType>,
    argument_values: &Vec<String>,
) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  call void @");
    string_push_string(&mut line, function_name);
    string_push(&mut line, '(');

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstType>(argument_types);
    while i < len {
        let argument_type: &RAstType = vec_at::<RAstType>(argument_types, i);
        let argument_value: &String = vec_at::<String>(argument_values, i);
        string_push_string(&mut line, &rAstType_to_llvm_name(argument_type));
        string_push(&mut line, ' ');
        string_push_string(&mut line, argument_value);

        i = i + 1;
        if i < len {
            string_push_str(&mut line, ", ");
        }
    }
    string_push_str(&mut line, ")");

    codegen_emit_line(codegen, line);
}

/// Emit a function header.
fn codegen_emit_function_header(
    codegen: &mut Codegen,
    fn_name: &String,
    return_type_name: &String,
    parameters: &Vec<RAstVariable>,
) {
    let mut line: String = string_new();
    string_push_str(&mut line, "define ");
    string_push_string(&mut line, return_type_name);
    string_push_str(&mut line, " @");
    string_push_string(&mut line, fn_name);
    string_push_str(&mut line, "(");

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstVariable>(parameters);
    while i < len {
        let RAstVariable::Variable(pattern, parameter_type): &RAstVariable =
            vec_at::<RAstVariable>(parameters, i);

        // TODO: what if wildcards are used? Duplicate register names?
        let parameter_name: String = match pattern {
            RAstPattern::Identifier(_, name) => string_clone(name),
            _ => string("arg"),
        };

        string_push_string(&mut line, &rAstType_to_llvm_name(parameter_type));
        string_push_str(&mut line, " %");
        string_push_string(&mut line, &parameter_name);

        i = i + 1;
        if i < len {
            string_push_str(&mut line, ", ");
        }
    }
    string_push_str(&mut line, ") {\nentry:");

    codegen_emit_line(codegen, line);
}

/// Emit an LLVM declare for the given extern function.
fn codegen_emit_declare(
    codegen: &mut Codegen,
    fn_name: &String,
    parameters: &Vec<RAstVariable>,
    return_type: &RAstType,
) {
    let mut line: String = string_new();
    string_push_str(&mut line, "declare ");
    string_push_string(&mut line, &rAstType_to_llvm_name(return_type));
    string_push_str(&mut line, " @");
    string_push_string(&mut line, fn_name);
    string_push_str(&mut line, "(");

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstVariable>(parameters);
    while i < len {
        let RAstVariable::Variable(_, parameter_type): &RAstVariable =
            vec_at::<RAstVariable>(parameters, i);

        string_push_string(&mut line, &rAstType_to_llvm_name(parameter_type));

        i = i + 1;
        if i < len {
            string_push_str(&mut line, ", ");
        }
    }
    string_push_str(&mut line, ")");

    codegen_emit_line(codegen, line);
}

/// Fixup a previously emitted alloca instruction without changing the destination register.
// TODO: assumes a lot about the emitted LLVM-IR, make this more robust.
fn codegen_fixup_alloca(
    codegen: &mut Codegen,
    index: usize,
    new_type: &RAstType,
    new_count: usize,
) {
    let Code::Code(lines): &mut Code = codegen_code_mut(codegen);

    let old_alloca: &String = vec_at::<String>(lines, index);
    let mut new_alloca: String = string_new();

    let mut space_count: usize = 0;
    let mut i: usize = 0;

    // "  <register> = alloca " has 5 spaces.
    while space_count < 5 {
        let c: char = string_at(&old_alloca, i);

        if is_whitespace(c) {
            space_count = space_count + 1;
        }

        string_push(&mut new_alloca, c);
        i = i + 1;
    }

    string_push_string(&mut new_alloca, &rAstType_to_llvm_name(new_type));
    string_push_str(&mut new_alloca, ", i64 ");
    string_push_string(&mut new_alloca, &integer_to_string(new_count));

    codegen_fixup(codegen, index, new_alloca);
}

// -----------------------------------------------------------------
// -----------------------------------------------------------------
// ------------------------ LLVM Emulator -------------------------
// -----------------------------------------------------------------
// -----------------------------------------------------------------

// -----------------------------------------------------------------
// ---------------------- Lexical Analysis -------------------------
// -----------------------------------------------------------------

/// Tokens produced by the LLVM lexer.
enum LlvmToken {
    Define,          // "define"
    Declare,         // "declare"
    Ret,             // "ret"
    IntToPtr,        // "inttoptr"
    PtrToInt,        // "ptrtoint"
    Br,              // "br"
    Label,           // "label"
    Add,             // "add"
    Sub,             // "sub"
    Mul,             // "mul"
    Udiv,            // "udiv"
    Urem,            // "urem"
    Icmp,            // "icmp"
    Zext,            // "zext"
    Trunc,           // "trunc"
    Alloca,          // "alloca"
    Store,           // "store"
    Load,            // "load"
    To,              // "to"
    Call,            // "call"
    Gep,             // "getelementptr"
    Constant,        // "constant"
    Eq,              // "eq"
    Ne,              // "ne"
    Ugt,             // "ugt"
    Uge,             // "uge"
    Ult,             // "ult"
    Ule,             // "ule"
    Ptr,             // "ptr"
    I64,             // "i64"
    I8,              // "i8"
    I1,              // "i1"
    Void,            // "void"
    At,              // "@"
    Percent,         // "%"
    LParen,          // "("
    RParen,          // ")"
    LBrace,          // "{"
    RBrace,          // "}"
    LBracket,        // "["
    RBracket,        // "]"
    Comma,           // ","
    Assign,          // "="
    Colon,           // ":"
    CString(String), // c"..."
    Identifier(String),
    Integer(usize),
    Eof,
}

/// A type that encapsulates the state of the lexer for the LLVM-IR parser.
enum LlvmLexer {
    /// LLVM-IR human-readable source file, current token
    Lexer(SourceFile, LlvmToken),
}

/// Create a new LLVM lexer and scan the first token.
fn llvmLexer_new(source: String) -> LlvmLexer {
    let source_file: SourceFile = SourceFile::SourceFile(source, 0, 1, 0);
    let mut lexer: LlvmLexer = LlvmLexer::Lexer(source_file, LlvmToken::Eof);
    llvmLexer_next_token(&mut lexer);
    lexer
}

/// Get the lexer source file.
fn llvmLexer_sourcefile(LlvmLexer::Lexer(source, _): &LlvmLexer) -> &SourceFile {
    source
}

/// Get the lexer source file.
fn llvmLexer_sourcefile_mut(LlvmLexer::Lexer(source, _): &mut LlvmLexer) -> &mut SourceFile {
    source
}

/// Get the current lexer token.
fn llvmLexer_current_token(LlvmLexer::Lexer(_, token): &LlvmLexer) -> &LlvmToken {
    token
}

/// Set the current lexer token.
fn llvmLexer_set_current_token(LlvmLexer::Lexer(_, old_token): &mut LlvmLexer, token: LlvmToken) {
    *old_token = token;
}

/// Peek the current source character.
fn llvmLexer_peek_char(lexer: &LlvmLexer) -> Option<char> {
    let SourceFile::SourceFile(string, index, _, _): &SourceFile = llvmLexer_sourcefile(lexer);
    string_get(string, *index)
}

/// Peek the next source character after the current one and return true if it is the expected
/// character
fn llvmLexer_next_char_eq(lexer: &LlvmLexer, expected: char) -> bool {
    let SourceFile::SourceFile(content, index, _, _): &SourceFile = llvmLexer_sourcefile(lexer);
    match string_get(content, *index + 1) {
        Option::Some(character) => character == expected,
        _ => false,
    }
}

fn llvmLexer_expect_char(lexer: &mut LlvmLexer, expected: char) {
    match llvmLexer_consume_char(lexer) {
        Option::Some(c) => {
            if c != expected {
                panic!("unexpected character");
            }
        },
        _ => panic!("unexpected EOF"),
    }
}

/// Consume and return the current source character.
fn llvmLexer_consume_char(lexer: &mut LlvmLexer) -> Option<char> {
    let SourceFile::SourceFile(source, index, line, last_newline_idx): &mut SourceFile =
        llvmLexer_sourcefile_mut(lexer);

    let current: Option<char> = string_get(source, *index);
    *index = *index + 1;

    match current {
        Option::Some(character) => {
            if character == '\n' {
                *line = *line + 1;
                *last_newline_idx = *index;
            }
        },
        Option::None => {},
    }
    current
}

/// Consume and return the next token.
fn llvmLexer_next_token(lexer: &mut LlvmLexer) -> LlvmToken {
    llvmLexer_skip_whitespace_and_comments(lexer);

    let token: LlvmToken = match llvmLexer_peek_char(lexer) {
        Option::Some(ch) => {
            if and(ch == 'c', llvmLexer_next_char_eq(lexer, '"')) {
                let value: String = llvmLexer_scan_cstring(lexer);
                LlvmToken::CString(value)
            } else if or(is_alpha(ch), ch == '.') {
                let ident: String = llvmLexer_scan_identifier_or_keyword(lexer);
                llvm_identifier_to_token(ident)
            } else if is_digit(ch) {
                let value: usize = llvmLexer_scan_integer(lexer);
                LlvmToken::Integer(value)
            } else {
                llvmLexer_scan_symbol(lexer)
            }
        },
        Option::None => LlvmToken::Eof,
    };

    llvmLexer_set_current_token(lexer, llvmToken_clone(&token));
    token
}

/// Scan and return a c"..." string literal.
fn llvmLexer_scan_cstring(lexer: &mut LlvmLexer) -> String {
    let mut literal: String = string_new();
    llvmLexer_expect_char(lexer, 'c');
    llvmLexer_expect_char(lexer, '"');

    while true {
        match llvmLexer_consume_char(lexer) {
            Option::Some('"') => return literal,
            Option::Some('\\') => {
                let character: char = llvmLexer_scan_escape(lexer);
                string_push(&mut literal, character);
            },
            Option::Some(ch) => string_push(&mut literal, ch),
            Option::None => panic!("unterminated LLVM c-string"),
        }
    }
    literal // satisfy compiler
}

fn llvmLexer_scan_escape(lexer: &mut LlvmLexer) -> char {
    match llvmLexer_consume_char(lexer) {
        Option::Some(hex_digit) => {
            if is_hexadecimal_digit(hex_digit) {
                match llvmLexer_consume_char(lexer) {
                    Option::Some(second_hex_digit) => {
                        let mut char_byte: String = string_new();
                        string_push(&mut char_byte, hex_digit);
                        string_push(&mut char_byte, second_hex_digit);

                        unwrap::<usize>(string_to_integer(&char_byte, 16)) as u8 as char
                    },
                    _ => panic!("expected second digit for escaped character byte"),
                }
            } else {
                hex_digit
            }
        },
        Option::None => panic!("unterminated LLVM c-string"),
    }
}

fn llvmLexer_scan_identifier_or_keyword(lexer: &mut LlvmLexer) -> String {
    let mut identifier: String = string_new();
    while true {
        match llvmLexer_peek_char(lexer) {
            Option::Some(ch) => {
                if is_alphanumeric_or_dot(ch) {
                    llvmLexer_consume_char(lexer);
                    string_push(&mut identifier, ch);
                } else {
                    return identifier;
                }
            },
            Option::None => return identifier,
        }
    }
    identifier // satisfy compiler
}

fn llvm_identifier_to_token(identifier: String) -> LlvmToken {
    if string_eq(&identifier, &string("define")) {
        LlvmToken::Define
    } else if string_eq(&identifier, &string("declare")) {
        LlvmToken::Declare
    } else if string_eq(&identifier, &string("ret")) {
        LlvmToken::Ret
    } else if string_eq(&identifier, &string("inttoptr")) {
        LlvmToken::IntToPtr
    } else if string_eq(&identifier, &string("ptrtoint")) {
        LlvmToken::PtrToInt
    } else if string_eq(&identifier, &string("br")) {
        LlvmToken::Br
    } else if string_eq(&identifier, &string("label")) {
        LlvmToken::Label
    } else if string_eq(&identifier, &string("add")) {
        LlvmToken::Add
    } else if string_eq(&identifier, &string("sub")) {
        LlvmToken::Sub
    } else if string_eq(&identifier, &string("mul")) {
        LlvmToken::Mul
    } else if string_eq(&identifier, &string("udiv")) {
        LlvmToken::Udiv
    } else if string_eq(&identifier, &string("urem")) {
        LlvmToken::Urem
    } else if string_eq(&identifier, &string("icmp")) {
        LlvmToken::Icmp
    } else if string_eq(&identifier, &string("zext")) {
        LlvmToken::Zext
    } else if string_eq(&identifier, &string("trunc")) {
        LlvmToken::Trunc
    } else if string_eq(&identifier, &string("alloca")) {
        LlvmToken::Alloca
    } else if string_eq(&identifier, &string("store")) {
        LlvmToken::Store
    } else if string_eq(&identifier, &string("load")) {
        LlvmToken::Load
    } else if string_eq(&identifier, &string("to")) {
        LlvmToken::To
    } else if string_eq(&identifier, &string("call")) {
        LlvmToken::Call
    } else if string_eq(&identifier, &string("getelementptr")) {
        LlvmToken::Gep
    } else if string_eq(&identifier, &string("constant")) {
        LlvmToken::Constant
    } else if string_eq(&identifier, &string("eq")) {
        LlvmToken::Eq
    } else if string_eq(&identifier, &string("ne")) {
        LlvmToken::Ne
    } else if string_eq(&identifier, &string("ugt")) {
        LlvmToken::Ugt
    } else if string_eq(&identifier, &string("uge")) {
        LlvmToken::Uge
    } else if string_eq(&identifier, &string("ult")) {
        LlvmToken::Ult
    } else if string_eq(&identifier, &string("ule")) {
        LlvmToken::Ule
    } else if string_eq(&identifier, &string("ptr")) {
        LlvmToken::Ptr
    } else if string_eq(&identifier, &string("i64")) {
        LlvmToken::I64
    } else if string_eq(&identifier, &string("i8")) {
        LlvmToken::I8
    } else if string_eq(&identifier, &string("i1")) {
        LlvmToken::I1
    } else if string_eq(&identifier, &string("void")) {
        LlvmToken::Void
    } else {
        LlvmToken::Identifier(identifier)
    }
}

fn llvmLexer_scan_integer(lexer: &mut LlvmLexer) -> usize {
    let mut value: usize = 0;
    while true {
        match llvmLexer_peek_char(lexer) {
            Option::Some(ch) => {
                if is_digit(ch) {
                    let digit: usize = (ch as usize) - ('0' as usize);
                    value = value * 10 + digit;
                    llvmLexer_consume_char(lexer);
                } else {
                    return value;
                }
            },
            Option::None => return value,
        }
    }
    value
}

fn llvmLexer_scan_symbol(lexer: &mut LlvmLexer) -> LlvmToken {
    match unwrap::<char>(llvmLexer_consume_char(lexer)) {
        '@' => LlvmToken::At,
        '%' => LlvmToken::Percent,
        '(' => LlvmToken::LParen,
        ')' => LlvmToken::RParen,
        '{' => LlvmToken::LBrace,
        '}' => LlvmToken::RBrace,
        '[' => LlvmToken::LBracket,
        ']' => LlvmToken::RBracket,
        ',' => LlvmToken::Comma,
        '=' => LlvmToken::Assign,
        ':' => LlvmToken::Colon,
        _ => panic!("unsupported token in LLVM input"),
    }
}

fn llvmLexer_skip_whitespace_and_comments(lexer: &mut LlvmLexer) {
    while true {
        match llvmLexer_peek_char(lexer) {
            Option::Some(ch) => {
                if is_whitespace(ch) {
                    llvmLexer_consume_char(lexer);
                } else if ch == ';' {
                    llvmLexer_consume_char(lexer);
                    llvmLexer_skip_line(lexer);
                } else {
                    return;
                }
            },
            Option::None => return,
        }
    }
}

fn llvmLexer_skip_line(lexer: &mut LlvmLexer) {
    while true {
        match llvmLexer_consume_char(lexer) {
            Option::Some('\n') => return,
            Option::Some(_) => (),
            Option::None => return,
        }
    }
}

// -----------------------------------------------------------------
// ------------------------- Parser --------------------------------
// -----------------------------------------------------------------

/// The parser for LLVM-IR.
enum LParser {
    Parser(LlvmLexer, LlvmAst, LlvmLocalSymTable),
}

/// Create an LLVM parser and prime the first token.
fn llvmParser_new(source: String) -> LParser {
    LParser::Parser(
        llvmLexer_new(source),
        llvmAst_new(),
        llvmLocalSymTable_new(),
    )
}

/// Get immutable parser lexer access.
fn llvmParser_lexer(LParser::Parser(lexer, _, _): &LParser) -> &LlvmLexer {
    lexer
}

/// Get mutable parser lexer access.
fn llvmParser_lexer_mut(LParser::Parser(lexer, _, _): &mut LParser) -> &mut LlvmLexer {
    lexer
}

/// Get mutable parser AST access.
fn llvmParser_ast_mut(LParser::Parser(_, ast, _): &mut LParser) -> &mut LlvmAst {
    ast
}

fn llvmParser_local(LParser::Parser(_, _, local): &LParser) -> &LlvmLocalSymTable {
    local
}

fn llvmParser_local_mut(LParser::Parser(_, _, local): &mut LParser) -> &mut LlvmLocalSymTable {
    local
}

/// Parse LLVM source into LLVM AST.
fn llvmParser_parse_to_ast(source: String) -> LlvmAst {
    let mut parser: LParser = llvmParser_new(source);
    llvmParser_parse_language(&mut parser);
    let LParser::Parser(_, ast, _): LParser = parser;
    ast
}

/// Get current LLVM parser token.
fn llvmParser_current_token(parser: &LParser) -> &LlvmToken {
    llvmLexer_current_token(llvmParser_lexer(parser))
}

/// Consume and return the current LLVM parser token.
fn llvmParser_consume_current_token(parser: &mut LParser) -> LlvmToken {
    let lexer: &LlvmLexer = llvmParser_lexer(parser);
    let token: LlvmToken = llvmToken_clone(llvmLexer_current_token(lexer));
    llvmParser_next_token(parser);
    token
}

/// Advance and return next LLVM parser token.
fn llvmParser_next_token(parser: &mut LParser) -> LlvmToken {
    llvmLexer_next_token(llvmParser_lexer_mut(parser))
}

/// Check whether parser current token equals expected token.
fn llvmParser_current_token_eq(parser: &LParser, token: &LlvmToken) -> bool {
    llvmToken_eq(llvmParser_current_token(parser), token)
}

/// Try consuming one token and report success.
fn llvmParser_try_consume(parser: &mut LParser, token: &LlvmToken) -> bool {
    if llvmParser_current_token_eq(parser, token) {
        llvmParser_next_token(parser);
        true
    } else {
        false
    }
}

/// Require and consume one token.
fn llvmParser_expect_token(parser: &mut LParser, token: &LlvmToken) {
    if not(llvmParser_try_consume(parser, token)) {
        let message: String = llvmParser_expected_message(parser, &llvmToken_to_string(token));
        llvmParser_error(parser, &message);
    }
}

/// Read and consume one identifier token.
fn llvmParser_expect_identifier(parser: &mut LParser) -> String {
    match llvmParser_current_token(parser) {
        LlvmToken::Identifier(identifier) => {
            let value: String = string_clone(identifier);
            llvmParser_next_token(parser);
            value
        },
        _ => {
            let message: String = llvmParser_expected_message(parser, &string("LLVM identifier"));
            llvmParser_error(parser, &message)
        },
    }
}

fn llvmParser_expect_value_type(parser: &LParser, value: &LlvmValue, expected: &LlvmType) {
    if not(llvmParser_value_has_type(parser, value, expected)) {
        llvmParser_error(parser, &string("LLVM value does not match expected type"));
    }
}

fn llvmParser_value_has_type(parser: &LParser, value: &LlvmValue, expected: &LlvmType) -> bool {
    match value {
        LlvmValue::Register(name) => {
            match llvmLocalSymTable_lookup_register_type(llvmParser_local(parser), name) {
                Option::Some(actual) => llvmType_eq(actual, expected),
                Option::None => false,
            }
        },
        LlvmValue::Literal(_) => match expected {
            LlvmType::I1 | LlvmType::I8 | LlvmType::I64 => true, // allow overflows
            _ => false,
        },
        LlvmValue::Global(_) => match expected {
            LlvmType::Ptr => true,
            _ => false,
        },
    }
}

/// Return true if the current token indicates the start of a new instruction.
fn llvmParser_is_instruction_start(parser: &mut LParser) -> bool {
    match llvmParser_current_token(parser) {
        LlvmToken::RBrace | LlvmToken::Identifier(_) => false,
        _ => true,
    }
}

enum LlvmAst {
    AST(Vec<LlvmGlobal>, StringMap<LlvmFunction>),
}

/// Top-level LLVM global data.
enum LlvmGlobal {
    /// name, bytes
    String(String, String),
}

/// Create an empty LLVM AST.
fn llvmAst_new() -> LlvmAst {
    LlvmAst::AST(vec_new::<LlvmGlobal>(), stringMap_new::<LlvmFunction>())
}

/// Get immutable access to the top-level globals list.
fn llvmAst_globals(LlvmAst::AST(globals, _): &LlvmAst) -> &Vec<LlvmGlobal> {
    globals
}

/// Get mutable access to the top-level globals list.
fn llvmAst_globals_mut(LlvmAst::AST(globals, _): &mut LlvmAst) -> &mut Vec<LlvmGlobal> {
    globals
}

/// Insert a global entry into the AST. Returns false on duplicate name.
fn llvmAst_insert_global(ast: &mut LlvmAst, name: String, global: LlvmGlobal) -> bool {
    let globals: &Vec<LlvmGlobal> = llvmAst_globals(ast);

    let mut i: usize = 0;
    while i < vec_len::<LlvmGlobal>(globals) {
        let LlvmGlobal::String(existing_name, _): &LlvmGlobal = vec_at::<LlvmGlobal>(globals, i);
        if string_eq(existing_name, &name) {
            return false;
        }
        i = i + 1;
    }

    vec_push::<LlvmGlobal>(llvmAst_globals_mut(ast), global);
    true
}

/// Get immutable access to the top-level function map.
fn llvmAst_functions(LlvmAst::AST(_, functions): &LlvmAst) -> &StringMap<LlvmFunction> {
    functions
}

/// Get mutable access to the top-level function map.
fn llvmAst_functions_mut(LlvmAst::AST(_, functions): &mut LlvmAst) -> &mut StringMap<LlvmFunction> {
    functions
}

/// Insert a function entry into the AST. Returns false on duplicate name.
fn llvmAst_insert_function(ast: &mut LlvmAst, name: String, function: LlvmFunction) -> bool {
    if stringMap_contains::<LlvmFunction>(llvmAst_functions(ast), &name) {
        false
    } else {
        stringMap_insert::<LlvmFunction>(llvmAst_functions_mut(ast), name, function);
        true
    }
}

/// Lookup a function in the AST by name.
fn llvmAst_lookup_function(ast: &LlvmAst, name: String) -> &LlvmFunction {
    match stringMap_get::<LlvmFunction>(llvmAst_functions(ast), &name) {
        Option::Some(function) => function,
        Option::None => panic!("unknown LLVM function"),
    }
}

/// Local symbol table for LLVM to track virtual register
enum LlvmLocalSymTable {
    Registers(StringMap<LlvmType>),
}

/// Create an empty LLVM local symbol table.
fn llvmLocalSymTable_new() -> LlvmLocalSymTable {
    LlvmLocalSymTable::Registers(stringMap_new::<LlvmType>())
}

/// Clear local register table buckets.
fn llvmLocalSymTable_clear(symtable: &mut LlvmLocalSymTable) {
    match symtable {
        LlvmLocalSymTable::Registers(registers) => *registers = stringMap_new::<LlvmType>(),
    }
}

/// Insert register name. Returns false on duplicate.
fn llvmLocalSymTable_insert_register(
    LlvmLocalSymTable::Registers(registers): &mut LlvmLocalSymTable,
    name: String,
    ty: LlvmType,
) -> bool {
    // Check SSA
    if stringMap_contains::<LlvmType>(registers, &name) {
        false
    } else {
        stringMap_insert::<LlvmType>(registers, name, ty);
        true
    }
}

/// Lookup a register type in the local symbol table.
fn llvmLocalSymTable_lookup_register_type<'a>(
    LlvmLocalSymTable::Registers(registers): &'a LlvmLocalSymTable,
    name: &String,
) -> Option<&'a LlvmType> {
    stringMap_get::<LlvmType>(registers, name)
}

/// An executable LLVM-IR function.
enum LlvmFunction {
    /// return type, parameters, basic blocks
    // TODO: use StringMap for InstructionBlocks
    Function(LlvmType, Vec<LlvmParameter>, Vec<InstructionBlock>),
    /// return type, parameters, builtin
    BuiltIn(LlvmBuiltIn, LlvmType, Vec<LlvmParameter>),
}

/// Supported LLVM-IR declared functions.
enum LlvmBuiltIn {
    Exit,
    Malloc,
}

/// Represents a parameter of an LLVM function.
enum LlvmParameter {
    /// identifier, type
    Parameter(String, LlvmType),
}

/// Supported LLVM types in the subset.
#[derive(Debug)]
enum LlvmType {
    I1,
    I8,
    I64,
    Ptr,
    Array(usize, Box<LlvmType>),
    Void,
}

fn llvmType_bitwidth(ty: &LlvmType) -> usize {
    match ty {
        LlvmType::I1 => 1,
        LlvmType::I8 => 8,
        LlvmType::I64 => 64,
        LlvmType::Ptr => size_of::<usize>() * 8,
        LlvmType::Array(len, inner) => *len * llvmType_bitwidth(box_deref::<LlvmType>(inner)),
        LlvmType::Void => 0,
    }
}

/// Return the size of an LLVM type in bytes.
fn llvmType_size(ty: &LlvmType) -> usize {
    max(1, llvmType_bitwidth(ty) / 8)
}

/// Represents an instruction block.
enum InstructionBlock {
    /// label, instructions
    Block(String, Vec<Instruction>),
}

/// Get a shared reference to the label of an instruction block.
fn instructionBlock_label(InstructionBlock::Block(label, _): &InstructionBlock) -> &String {
    label
}

/// Get a shared reference to the instructions of an instruction block.
fn instructionBlock_instructions(
    InstructionBlock::Block(_, instructions): &InstructionBlock,
) -> &Vec<Instruction> {
    instructions
}

/// Get the instructions of the block labelled by the given label.
fn instructionBlock_fetch_instructions(
    blocks: &Vec<InstructionBlock>,
    label: String,
) -> &Vec<Instruction> {
    let mut i: usize = 0;
    while i < vec_len::<InstructionBlock>(blocks) {
        let block: &InstructionBlock = vec_at::<InstructionBlock>(blocks, i);

        let other_label: &String = instructionBlock_label(block);
        if string_eq(other_label, &label) {
            return instructionBlock_instructions(block);
        }

        i = i + 1;
    }
    panic!("unknown LLVM block label");
}

/// Represents an instruction inside an instruction block.
///
/// Assignment: Assigns the value of an instruction to a virtual register.
/// Store: Stores a value at the given ptr value.
/// Call: Calls a function for side effects, discarding the result.
/// Ret: Returns from the current function with an optional return value.
/// Br: Branches to another basic block.
enum Instruction {
    Assignment(AssignInstruction),
    /// stored type, value, address
    Store(LlvmType, LlvmValue, LlvmValue),
    Call(Call),
    /// return type, optional value
    Ret(LlvmType, Option<LlvmValue>),
    Br(Branch),
}

/// A `call` instruction.
enum Call {
    /// return type, callee, arguments
    Call(LlvmType, String, Vec<LlvmTypedValue>),
}

/// Represents "br", either a conditional or unconditional jump.
enum Branch {
    /// label
    Unconditional(String),
    /// condition, then label, else label
    Conditional(LlvmValue, String, String),
}

/// Represents an assignment instruction.
enum AssignInstruction {
    Assign(String, AssignOp),
}

/// Represents the right-hand-side of an assignment
enum AssignOp {
    /// operation, type, left operand, right operand
    Binary(BinaryOp, LlvmType, LlvmValue, LlvmValue),
    /// operation, operand type, left operand, right operand
    Icmp(IcmpOp, LlvmType, LlvmValue, LlvmValue),
    /// operation, target type, value
    Cast(CastOp, LlvmType, LlvmValue),
    /// allocated type, number of elements
    Alloca(LlvmType, usize),
    /// loaded type, address
    Load(LlvmType, LlvmValue),
    Call(Call),
    /// type, pointer, indexes
    Gep(LlvmType, LlvmValue, Vec<LlvmTypedValue>),
}

/// Binary operations that can only appear in assignments.
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Udiv,
    Urem,
}

/// Unsigned integer comparison operations for icmp.
enum IcmpOp {
    Eq,
    Ne,
    Ugt,
    Uge,
    Ult,
    Ule,
}

/// Cast operations that can only appear in assignments.
enum CastOp {
    Zext,
    Trunc,
    IntToPtr,
    PtrToInt,
}

fn assignOp_get_type(operation: &AssignOp) -> LlvmType {
    match operation {
        AssignOp::Binary(_, ty, _, _) => llvmType_clone(ty),
        AssignOp::Icmp(_, _, _, _) => LlvmType::I1,
        AssignOp::Call(Call::Call(ty, _, _)) => llvmType_clone(ty),
        AssignOp::Cast(_, ty, _) => llvmType_clone(ty),
        AssignOp::Alloca(_, _) => LlvmType::Ptr,
        AssignOp::Load(ty, _) => llvmType_clone(ty),
        AssignOp::Gep(_, _, _) => LlvmType::Ptr,
    }
}

/// Represents an LLVM value operand.
#[derive(Debug)]
enum LlvmValue {
    /// identifier
    Register(String),
    /// integer value
    Literal(usize),
    /// identifier
    Global(String),
}

/// Represents a value with a specified type.
// TODO: drop this: the AST does not need to know about types. Parser ensures type safety.
enum LlvmTypedValue {
    Pair(LlvmType, LlvmValue),
}

fn llvmParser_parse_language(parser: &mut LParser) {
    while not(llvmParser_current_token_eq(parser, &LlvmToken::Eof)) {
        match llvmParser_current_token(parser) {
            LlvmToken::At => llvmParser_parse_string(parser),
            LlvmToken::Define => llvmParser_parse_function(parser),
            LlvmToken::Declare => llvmParser_parse_declare(parser),
            _ => {
                let message: String =
                    llvmParser_expected_message(parser, &string("LLVM top-level item"));
                llvmParser_error(parser, &message)
            },
        }
    }
}

fn llvmParser_parse_string(parser: &mut LParser) {
    let name: String = llvmParser_parse_global_name(parser);
    llvmParser_expect_token(parser, &LlvmToken::Assign);
    llvmParser_expect_token(parser, &LlvmToken::Constant);
    llvmParser_parse_type(parser);

    match llvmParser_current_token(parser) {
        LlvmToken::CString(value) => {
            let string_value: String = string_clone(value);
            llvmParser_next_token(parser);
            if not(llvmAst_insert_global(
                llvmParser_ast_mut(parser),
                string_clone(&name),
                LlvmGlobal::String(name, string_value),
            )) {
                llvmParser_error(parser, &string("duplicate LLVM global string"));
            }
        },
        _ => {
            let message: String =
                llvmParser_expected_message(parser, &string("LLVM c-string literal"));
            llvmParser_error(parser, &message)
        },
    }
}

fn llvmParser_parse_function(parser: &mut LParser) {
    llvmParser_expect_token(parser, &LlvmToken::Define);
    let return_type: LlvmType = llvmParser_parse_type(parser);
    let function_name: String = llvmParser_parse_global_name(parser);

    llvmLocalSymTable_clear(llvmParser_local_mut(parser));

    let parameters: Vec<LlvmParameter> = llvmParser_parse_parameters(parser, true);

    llvmParser_expect_token(parser, &LlvmToken::LBrace);
    let blocks: Vec<InstructionBlock> = llvmParser_parse_blocks(parser);
    llvmParser_expect_token(parser, &LlvmToken::RBrace);

    let function: LlvmFunction = LlvmFunction::Function(return_type, parameters, blocks);
    if not(llvmAst_insert_function(
        llvmParser_ast_mut(parser),
        function_name,
        function,
    )) {
        llvmParser_error(parser, &string("duplicate LLVM function definition"));
    }
}

fn llvmParser_parse_declare(parser: &mut LParser) {
    llvmParser_expect_token(parser, &LlvmToken::Declare);
    let return_type: LlvmType = llvmParser_parse_type(parser);
    let function_name: String = llvmParser_parse_global_name(parser);

    llvmLocalSymTable_clear(llvmParser_local_mut(parser));
    let parameters: Vec<LlvmParameter> = llvmParser_parse_parameters(parser, false);

    let builtin: LlvmBuiltIn = if string_eq(&function_name, &string("malloc")) {
        LlvmBuiltIn::Malloc
    } else if string_eq(&function_name, &string("exit")) {
        LlvmBuiltIn::Exit
    } else {
        llvmParser_error(parser, &string("unknown declared function"));
    };

    let function: LlvmFunction = LlvmFunction::BuiltIn(builtin, return_type, parameters);
    if not(llvmAst_insert_function(
        llvmParser_ast_mut(parser),
        function_name,
        function,
    )) {
        llvmParser_error(parser, &string("duplicate LLVM function declaration"));
    }
}

/// Parse parameters of a function.
///
/// * `parser`: The parser state
/// * `require_names`: True, if the parameters are named (function definition). False, if they are
/// not (function declaration).
fn llvmParser_parse_parameters(parser: &mut LParser, named: bool) -> Vec<LlvmParameter> {
    let mut parameters: Vec<LlvmParameter> = vec_new::<LlvmParameter>();

    llvmParser_expect_token(parser, &LlvmToken::LParen);

    if not(llvmParser_current_token_eq(parser, &LlvmToken::RParen)) {
        let parameter_type: LlvmType = llvmParser_parse_type(parser);
        let param_name: String = llvmParser_parse_parameter_name(parser, 0);
        llvmLocalSymTable_insert_register(
            llvmParser_local_mut(parser),
            string_clone(&param_name),
            llvmType_clone(&parameter_type),
        );

        let parameter: LlvmParameter = LlvmParameter::Parameter(param_name, parameter_type);
        vec_push::<LlvmParameter>(&mut parameters, parameter);

        while llvmParser_current_token_eq(parser, &LlvmToken::Comma) {
            llvmParser_next_token(parser);

            let parameter_type: LlvmType = llvmParser_parse_type(parser);
            let param_name: String =
                llvmParser_parse_parameter_name(parser, vec_len::<LlvmParameter>(&parameters));

            if named {
                if not(llvmLocalSymTable_insert_register(
                    llvmParser_local_mut(parser),
                    string_clone(&param_name),
                    llvmType_clone(&parameter_type),
                )) {
                    llvmParser_error(parser, &string("duplicate parameters in LLVM function"));
                }
            }

            let parameter: LlvmParameter = LlvmParameter::Parameter(param_name, parameter_type);
            vec_push::<LlvmParameter>(&mut parameters, parameter);
        }
    }
    llvmParser_expect_token(parser, &LlvmToken::RParen);
    parameters
}

fn llvmParser_parse_parameter_name(parser: &mut LParser, index: usize) -> String {
    if llvmParser_current_token_eq(parser, &LlvmToken::Percent) {
        llvmParser_parse_register(parser)
    } else {
        let mut name: String = string("arg");
        string_push_string(&mut name, &integer_to_string(index));
        name
    }
}

fn llvmParser_parse_global_name(parser: &mut LParser) -> String {
    llvmParser_expect_token(parser, &LlvmToken::At);
    llvmParser_expect_identifier(parser)
}

fn llvmParser_parse_blocks(parser: &mut LParser) -> Vec<InstructionBlock> {
    let mut blocks: Vec<InstructionBlock> = vec_new::<InstructionBlock>();
    while not(llvmParser_current_token_eq(parser, &LlvmToken::RBrace)) {
        let block: InstructionBlock = llvmParser_parse_block(parser);
        vec_push::<InstructionBlock>(&mut blocks, block);
    }
    blocks
}

fn llvmParser_parse_block(parser: &mut LParser) -> InstructionBlock {
    let label: String = llvmParser_expect_identifier(parser);
    llvmParser_expect_token(parser, &LlvmToken::Colon);
    // TODO: insert into symbol table

    let mut instructions: Vec<Instruction> = vec_new::<Instruction>();
    while llvmParser_is_instruction_start(parser) {
        let instruction: Instruction = llvmParser_parse_instruction(parser);
        vec_push::<Instruction>(&mut instructions, instruction);
    }

    InstructionBlock::Block(label, instructions)
}

fn llvmParser_parse_register(parser: &mut LParser) -> String {
    llvmParser_expect_token(parser, &LlvmToken::Percent);
    llvmParser_expect_identifier(parser)
}

fn llvmParser_parse_instruction(parser: &mut LParser) -> Instruction {
    match llvmParser_current_token(parser) {
        LlvmToken::Ret => llvmParser_parse_return(parser),
        LlvmToken::Br => llvmParser_parse_branch(parser),
        LlvmToken::Percent => Instruction::Assignment(llvmParser_parse_assignment(parser)),
        LlvmToken::Store => llvmParser_parse_store(parser),
        LlvmToken::Call => {
            llvmParser_next_token(parser);
            Instruction::Call(llvmParser_parse_call(parser))
        },
        _ => {
            let message: String = llvmParser_expected_message(parser, &string("LLVM instruction"));
            llvmParser_error(parser, &message)
        },
    }
}

fn llvmParser_parse_return(parser: &mut LParser) -> Instruction {
    llvmParser_expect_token(parser, &LlvmToken::Ret);
    let returned_type: LlvmType = llvmParser_parse_type(parser);
    let return_value: Option<LlvmValue> = if llvmType_eq(&returned_type, &LlvmType::Void) {
        Option::None
    } else {
        Option::Some(llvmParser_parse_value(parser))
    };
    Instruction::Ret(returned_type, return_value)
}

fn llvmParser_parse_branch(parser: &mut LParser) -> Instruction {
    llvmParser_expect_token(parser, &LlvmToken::Br);
    let branch: Branch = if llvmParser_try_consume(parser, &LlvmToken::Label) {
        let target_label: String = llvmParser_parse_register(parser);
        Branch::Unconditional(target_label)
    } else {
        llvmParser_expect_token(parser, &LlvmToken::I1);
        let condition: LlvmValue = llvmParser_parse_value(parser);
        llvmParser_expect_token(parser, &LlvmToken::Comma);

        llvmParser_expect_token(parser, &LlvmToken::Label);
        let then_label: String = llvmParser_parse_register(parser);
        llvmParser_expect_token(parser, &LlvmToken::Comma);

        llvmParser_expect_token(parser, &LlvmToken::Label);
        let else_label: String = llvmParser_parse_register(parser);

        Branch::Conditional(condition, then_label, else_label)
    };
    Instruction::Br(branch)
}

fn llvmParser_parse_assignment(parser: &mut LParser) -> AssignInstruction {
    let target_register: String = llvmParser_parse_register(parser);

    llvmParser_expect_token(parser, &LlvmToken::Assign);
    let operation: AssignOp = match llvmParser_consume_current_token(parser) {
        LlvmToken::Add => llvmParser_parse_binary_assign(parser, BinaryOp::Add),
        LlvmToken::Sub => llvmParser_parse_binary_assign(parser, BinaryOp::Sub),
        LlvmToken::Mul => llvmParser_parse_binary_assign(parser, BinaryOp::Mul),
        LlvmToken::Udiv => llvmParser_parse_binary_assign(parser, BinaryOp::Udiv),
        LlvmToken::Urem => llvmParser_parse_binary_assign(parser, BinaryOp::Urem),
        LlvmToken::Icmp => llvmParser_parse_icmp_assign(parser),
        LlvmToken::Zext => llvmParser_parse_cast_assign(parser, CastOp::Zext),
        LlvmToken::Trunc => llvmParser_parse_cast_assign(parser, CastOp::Trunc),
        LlvmToken::IntToPtr => llvmParser_parse_cast_assign(parser, CastOp::IntToPtr),
        LlvmToken::PtrToInt => llvmParser_parse_cast_assign(parser, CastOp::PtrToInt),
        LlvmToken::Alloca => llvmParser_parse_alloca_assign(parser),
        LlvmToken::Load => llvmParser_parse_load_assign(parser),
        LlvmToken::Call => llvmParser_parse_call_assign(parser),
        LlvmToken::Gep => llvmParser_parse_gep_assign(parser),
        _ => {
            let message: String =
                llvmParser_expected_message(parser, &string("LLVM assignment operation"));
            llvmParser_error(parser, &message)
        },
    };

    if not(llvmLocalSymTable_insert_register(
        llvmParser_local_mut(parser),
        string_clone(&target_register),
        assignOp_get_type(&operation),
    )) {
        llvmParser_error(
            parser,
            &string("SSA violation: duplicate virtual register assignment"),
        );
    }

    AssignInstruction::Assign(target_register, operation)
}

fn llvmParser_parse_binary_assign(parser: &mut LParser, operator: BinaryOp) -> AssignOp {
    let ty: LlvmType = llvmParser_parse_type(parser);
    let left: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_token(parser, &LlvmToken::Comma);
    let right: LlvmValue = llvmParser_parse_value(parser);
    AssignOp::Binary(operator, ty, left, right)
}

fn llvmParser_parse_icmp_assign(parser: &mut LParser) -> AssignOp {
    let predicate: IcmpOp = match llvmParser_consume_current_token(parser) {
        LlvmToken::Eq => IcmpOp::Eq,
        LlvmToken::Ne => IcmpOp::Ne,
        LlvmToken::Ugt => IcmpOp::Ugt,
        LlvmToken::Uge => IcmpOp::Uge,
        LlvmToken::Ult => IcmpOp::Ult,
        LlvmToken::Ule => IcmpOp::Ule,
        _ => {
            let message: String =
                llvmParser_expected_message(parser, &string("LLVM icmp operator"));
            llvmParser_error(parser, &message)
        },
    };

    let ty: LlvmType = llvmParser_parse_type(parser);
    let left: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_value_type(parser, &left, &ty);

    llvmParser_expect_token(parser, &LlvmToken::Comma);
    let right: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_value_type(parser, &right, &ty);

    AssignOp::Icmp(predicate, ty, left, right)
}

fn llvmParser_parse_call_assign(parser: &mut LParser) -> AssignOp {
    let call: Call = llvmParser_parse_call(parser);

    let Call::Call(return_type, _, _): &Call = &call;
    if llvmType_eq(return_type, &LlvmType::Void) {
        llvmParser_error(parser, &string("cannot assign void to a register"));
    }

    AssignOp::Call(call)
}

fn llvmParser_parse_cast_assign(parser: &mut LParser, operator: CastOp) -> AssignOp {
    let from_type: LlvmType = llvmParser_parse_type(parser);

    let value: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_value_type(parser, &value, &from_type);

    llvmParser_expect_token(parser, &LlvmToken::To);
    let to_type: LlvmType = llvmParser_parse_type(parser);

    match &operator {
        CastOp::Zext => {
            let from_bits: usize = llvmType_bitwidth(&from_type);
            let to_bits: usize = llvmType_bitwidth(&to_type);
            if not(from_bits < to_bits) {
                llvmParser_error(
                    parser,
                    &string("invalid LLVM zext: source type must be smaller than target type"),
                );
            }
        },
        CastOp::Trunc => {
            let from_bits: usize = llvmType_bitwidth(&from_type);
            let to_bits: usize = llvmType_bitwidth(&to_type);
            if not(from_bits > to_bits) {
                llvmParser_error(
                    parser,
                    &string("invalid LLVM trunc: source type must be larger than target type"),
                );
            }
        },
        CastOp::IntToPtr => {
            if not(llvmType_eq(&from_type, &LlvmType::I64)) {
                llvmParser_error(
                    parser,
                    &string("invalid LLVM inttoptr: source type must be i64"),
                );
            }
            if not(llvmType_eq(&to_type, &LlvmType::Ptr)) {
                llvmParser_error(
                    parser,
                    &string("invalid LLVM inttoptr: target type must be ptr"),
                );
            }
        },
        CastOp::PtrToInt => {
            if not(llvmType_eq(&from_type, &LlvmType::Ptr)) {
                llvmParser_error(
                    parser,
                    &string("invalid LLVM ptrtoint: source type must be ptr"),
                );
            }
            if not(llvmType_eq(&to_type, &LlvmType::I64)) {
                llvmParser_error(
                    parser,
                    &string("invalid LLVM ptrtoint: target type must be i64"),
                );
            }
        },
    }

    AssignOp::Cast(operator, to_type, value)
}

fn llvmParser_parse_alloca_assign(parser: &mut LParser) -> AssignOp {
    let allocated_type: LlvmType = llvmParser_parse_type(parser);
    llvmParser_expect_token(parser, &LlvmToken::Comma);

    llvmParser_expect_token(parser, &LlvmToken::I64);
    let num_elements: usize = llvmParser_parse_integer(parser);

    AssignOp::Alloca(allocated_type, num_elements)
}

fn llvmParser_parse_load_assign(parser: &mut LParser) -> AssignOp {
    let loaded_type: LlvmType = llvmParser_parse_type(parser);
    llvmParser_expect_token(parser, &LlvmToken::Comma);

    llvmParser_expect_token(parser, &LlvmToken::Ptr);
    let address: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_value_type(parser, &address, &LlvmType::Ptr);

    AssignOp::Load(loaded_type, address)
}

fn llvmParser_parse_gep_assign(parser: &mut LParser) -> AssignOp {
    let base_type: LlvmType = llvmParser_parse_type(parser);
    llvmParser_expect_token(parser, &LlvmToken::Comma);
    llvmParser_expect_token(parser, &LlvmToken::Ptr);
    let pointer_value: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_token(parser, &LlvmToken::Comma);

    let mut indexes: Vec<LlvmTypedValue> = vec_new::<LlvmTypedValue>();
    let first_index_type: LlvmType = llvmParser_parse_type(parser);
    let first_index_value: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_value_type(parser, &first_index_value, &first_index_type);
    let first_index: LlvmTypedValue = LlvmTypedValue::Pair(first_index_type, first_index_value);
    vec_push::<LlvmTypedValue>(&mut indexes, first_index);
    while llvmParser_try_consume(parser, &LlvmToken::Comma) {
        let index_type: LlvmType = llvmParser_parse_type(parser);
        let index_value: LlvmValue = llvmParser_parse_value(parser);
        llvmParser_expect_value_type(parser, &index_value, &index_type);
        let index: LlvmTypedValue = LlvmTypedValue::Pair(index_type, index_value);
        vec_push::<LlvmTypedValue>(&mut indexes, index);
    }

    AssignOp::Gep(base_type, pointer_value, indexes)
}

fn llvmParser_parse_store(parser: &mut LParser) -> Instruction {
    llvmParser_expect_token(parser, &LlvmToken::Store);

    let store_type: LlvmType = llvmParser_parse_type(parser);
    let value: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_value_type(parser, &value, &store_type);

    llvmParser_expect_token(parser, &LlvmToken::Comma);
    llvmParser_expect_token(parser, &LlvmToken::Ptr);

    let address: LlvmValue = llvmParser_parse_value(parser);
    llvmParser_expect_value_type(parser, &address, &LlvmType::Ptr);

    Instruction::Store(store_type, value, address)
}

fn llvmParser_parse_call(parser: &mut LParser) -> Call {
    let return_type: LlvmType = llvmParser_parse_type(parser);
    let callee: String = llvmParser_parse_global_name(parser);

    llvmParser_expect_token(parser, &LlvmToken::LParen);
    let mut arguments: Vec<LlvmTypedValue> = vec_new::<LlvmTypedValue>();
    if not(llvmParser_current_token_eq(parser, &LlvmToken::RParen)) {
        let arg_type: LlvmType = llvmParser_parse_type(parser);
        let arg_value: LlvmValue = llvmParser_parse_value(parser);
        llvmParser_expect_value_type(parser, &arg_value, &arg_type);
        vec_push::<LlvmTypedValue>(&mut arguments, LlvmTypedValue::Pair(arg_type, arg_value));

        while llvmParser_current_token_eq(parser, &LlvmToken::Comma) {
            llvmParser_next_token(parser);

            let arg_type: LlvmType = llvmParser_parse_type(parser);
            let arg_value: LlvmValue = llvmParser_parse_value(parser);
            llvmParser_expect_value_type(parser, &arg_value, &arg_type);
            vec_push::<LlvmTypedValue>(&mut arguments, LlvmTypedValue::Pair(arg_type, arg_value));
        }
    }
    llvmParser_expect_token(parser, &LlvmToken::RParen);

    Call::Call(return_type, callee, arguments)
}

fn llvmParser_parse_type(parser: &mut LParser) -> LlvmType {
    match llvmParser_consume_current_token(parser) {
        LlvmToken::I1 => LlvmType::I1,
        LlvmToken::I8 => LlvmType::I8,
        LlvmToken::I64 => LlvmType::I64,
        LlvmToken::Void => LlvmType::Void,
        LlvmToken::Ptr => LlvmType::Ptr,
        LlvmToken::LBracket => {
            let len: usize = llvmParser_parse_integer(parser);
            match llvmParser_current_token(parser) {
                LlvmToken::Identifier(separator) => {
                    if not(string_eq(separator, &string("x"))) {
                        let message: String =
                            llvmParser_expected_message(parser, &string("x in LLVM array type"));
                        llvmParser_error(parser, &message);
                    }
                    llvmParser_next_token(parser);
                },
                _ => {
                    let message: String =
                        llvmParser_expected_message(parser, &string("x in LLVM array type"));
                    llvmParser_error(parser, &message)
                },
            }
            let inner: LlvmType = llvmParser_parse_type(parser);
            llvmParser_expect_token(parser, &LlvmToken::RBracket);
            LlvmType::Array(len, box_new::<LlvmType>(inner))
        },
        _ => {
            let message: String = llvmParser_expected_message(parser, &string("LLVM type"));
            llvmParser_error(parser, &message)
        },
    }
}

fn llvmParser_parse_value(parser: &mut LParser) -> LlvmValue {
    match llvmParser_current_token(parser) {
        LlvmToken::Percent => LlvmValue::Register(llvmParser_parse_register(parser)),
        LlvmToken::At => LlvmValue::Global(llvmParser_parse_global_name(parser)),
        LlvmToken::Integer(_) => LlvmValue::Literal(llvmParser_parse_integer(parser)),
        _ => {
            let message: String = llvmParser_expected_message(parser, &string("LLVM value"));
            llvmParser_error(parser, &message)
        },
    }
}

fn llvmParser_parse_integer(parser: &mut LParser) -> usize {
    match llvmParser_consume_current_token(parser) {
        LlvmToken::Integer(value) => value,
        _ => {
            let message: String =
                llvmParser_expected_message(parser, &string("LLVM integer literal"));
            llvmParser_error(parser, &message)
        },
    }
}

// ------------------------- Interpreter -----------------------------

/// Execution control flow after one instruction.
enum LlvmExecFlow {
    Continue,
    /// label
    Jump(String),
    /// return value
    Return(usize),
}

/// Type that encapsulates the state of the LLVM emulator.
enum Emu {
    Emu(
        /// map of global names to their addresses
        StringMap<usize>,
        /// byte-addressed, double-word-aligned and big-endian memory (data, heap, stack)
        Vec<u8>,
        /// stack pointer
        usize,
        /// current frame size,
        usize,
        /// global pointer (end of data segment, start of heap)
        usize,
        /// exit code, if the program exited
        Option<usize>,
    ),
}

/// Create a new emulator state with `memory_size` bytes of main memory.
fn emu_new(memory_size: usize) -> Emu {
    let stack_pointer: usize = memory_size;
    let global_pointer: usize = 0;
    Emu::Emu(
        stringMap_new::<usize>(),
        vec_with_len::<u8>(memory_size),
        stack_pointer,
        0,
        global_pointer,
        Option::None,
    )
}

/// Get a shared reference to the global values.
fn emu_globals(Emu::Emu(globals, _, _, _, _, _): &Emu) -> &StringMap<usize> {
    globals
}

/// Get mutable access to the global values.
fn emu_globals_mut(Emu::Emu(globals, _, _, _, _, _): &mut Emu) -> &mut StringMap<usize> {
    globals
}

/// Get the current value of the stack pointer.
fn emu_get_sp(Emu::Emu(_, _, stack_pointer, _, _, _): &Emu) -> usize {
    *stack_pointer
}

/// Set the value of the stack pointer.
fn emu_set_sp(Emu::Emu(_, _, stack_pointer, _, _, _): &mut Emu, value: usize) {
    *stack_pointer = value;
}

/// Get the size of the active stack frame in bytes.
fn emu_get_frame_size(Emu::Emu(_, _, _, frame_size, _, _): &Emu) -> usize {
    *frame_size
}

/// Set the size of the active stack frame.
fn emu_set_frame_size(Emu::Emu(_, _, _, frame_size, _, _): &mut Emu, value: usize) {
    *frame_size = value;
}

/// Get the global pointer (end of data segment).
fn emu_get_gp(Emu::Emu(_, _, _, _, gp, _): &Emu) -> usize {
    *gp
}

/// Set the global pointer (end of data segment).
fn emu_set_gp(Emu::Emu(_, _, _, _, gp, _): &mut Emu, value: usize) {
    *gp = value;
}

/// Get the address of the top of the heap.
fn emu_get_heap_pointer(emulator: &Emu) -> usize {
    let gp: usize = emu_get_gp(emulator);
    match emu_load_bytes(emulator, gp - size_of::<usize>(), size_of::<usize>()) {
        Option::Some(value) => value,
        Option::None => gp,
    }
}

/// Allocate and set the heap pointer (which is used by the bump allocator).
fn emu_set_heap_pointer(emulator: &mut Emu, value: usize) {
    let gp: usize = emu_get_gp(emulator);
    emu_store_bytes(emulator, gp, value, size_of::<usize>());
    emu_set_gp(emulator, gp + size_of::<usize>());
}

/// Return true if exit was requested and return the code.
fn emu_exit_code(Emu::Emu(_, _, _, _, _, exit_code): &Emu) -> Option<usize> {
    match exit_code {
        Option::Some(code) => Option::Some(*code),
        Option::None => Option::None,
    }
}

/// Set the exit code and mark the program as exited.
fn emu_set_exit_code(Emu::Emu(_, _, _, _, _, exit_code): &mut Emu, code: usize) {
    *exit_code = Option::Some(code);
}

/// Align the given address or size to a double-word boundary.
fn emu_align_to_double(value: usize) -> usize {
    let align: usize = size_of::<usize>();
    if value % align == 0 {
        value
    } else {
        value + (align - (value % align))
    }
}

/// Allocate `size` many double words on the stack and return the address.
fn emu_allocate_stack(emulator: &mut Emu, size: usize) -> Option<usize> {
    let bytes: usize = size * size_of::<usize>();
    let stack_pointer: usize = emu_get_sp(emulator);
    let frame_size: usize = emu_get_frame_size(emulator);

    let new_sp: usize = stack_pointer - size * size_of::<usize>();
    emu_set_sp(emulator, new_sp);
    emu_set_frame_size(emulator, frame_size + bytes);
    Option::Some(new_sp)
}

/// Allocate `size` bytes on the heap and return the address.
fn emu_allocate_heap(emulator: &mut Emu, size: usize) -> Option<usize> {
    let size: usize = max(size, size_of::<usize>());
    let aligned_size: usize = emu_align_to_double(size);
    let heap_pointer: usize = emu_get_heap_pointer(emulator);
    let new_heap_pointer: usize = heap_pointer + aligned_size;
    let stack_pointer: usize = emu_get_sp(emulator);

    if new_heap_pointer >= stack_pointer {
        Option::None
    } else {
        emu_set_heap_pointer(emulator, new_heap_pointer);
        Option::Some(heap_pointer)
    }
}

/// Load top-level LLVM string globals into the data segment.
fn emu_load_globals(emulator: &mut Emu, ast: &LlvmAst) {
    let mut data_pointer: usize = emu_get_gp(emulator);

    let mut i: usize = 0;
    while i < vec_len::<LlvmGlobal>(llvmAst_globals(ast)) {
        let LlvmGlobal::String(name, value): &LlvmGlobal =
            vec_at::<LlvmGlobal>(llvmAst_globals(ast), i);

        let alloc_size: usize = emu_align_to_double(string_len(value));
        let address: usize = data_pointer;

        let mut j: usize = 0;
        while j < string_len(value) {
            let character: usize = unwrap::<char>(string_get(value, j)) as usize;
            emu_store_bytes(emulator, address + j, character, 1);
            j = j + 1;
        }

        stringMap_insert::<usize>(emu_globals_mut(emulator), string_clone(name), address);
        data_pointer = data_pointer + alloc_size;
        i = i + 1;
    }

    emu_set_gp(emulator, data_pointer);
    emu_set_heap_pointer(emulator, data_pointer);
}

/// Deallocates the top stack frame by resetting the frame size to 0 and moving the stack pointer up
/// by the frame size.
fn emu_deallocate_stack_frame(emulator: &mut Emu) {
    let stack_pointer: usize = emu_get_sp(emulator);
    let frame_size: usize = emu_get_frame_size(emulator);
    emu_set_sp(emulator, stack_pointer + frame_size);
    emu_set_frame_size(emulator, 0);
}

/// Store a little-endian integer value at `address` using `byte_count` bytes.
fn emu_store_bytes(emulator: &mut Emu, address: usize, value: usize, byte_count: usize) -> bool {
    let Emu::Emu(_, memory, _, _, _, _): &mut Emu = emulator;

    let mut remaining: usize = value;
    let mut i: usize = 0;
    while i < byte_count {
        let byte: u8 = (remaining % 256) as u8;

        if not(vec_set::<u8>(memory, address + i, byte)) {
            return false;
        }

        remaining = remaining / 256;
        i = i + 1;
    }
    true
}

/// Load a little-endian integer value from `address` using `byte_count` bytes.
fn emu_load_bytes(emulator: &Emu, address: usize, byte_count: usize) -> Option<usize> {
    let Emu::Emu(_, memory, _, _, _, _): &Emu = emulator;

    let mut value: usize = 0;
    let mut factor: usize = 1;
    let mut i: usize = 0;
    while i < byte_count {
        match vec_get::<u8>(memory, address + i) {
            Option::Some(byte) => {
                value = value + (*byte as usize) * factor;
                if i + 1 < byte_count {
                    factor = factor * 256;
                }
            },
            _ => return Option::None,
        }
        i = i + 1;
    }
    Option::Some(value as usize)
}

/// Parse and emulate LLVM source and return the return value of @main.
fn emu_execute_llvm(source: String) -> usize {
    let ast: LlvmAst = llvmParser_parse_to_ast(source);

    let main_name: String = string("main");
    let empty_args: Vec<usize> = vec_new::<usize>();

    // TODO: parameterise memory size
    let mut emulator: Emu = emu_new(3000000);
    emu_load_globals(&mut emulator, &ast);
    emu_execute_function_named(&mut emulator, &ast, &main_name, &empty_args)
}

/// Lookup a function by name and execute it.
fn emu_execute_function_named(
    emulator: &mut Emu,
    ast: &LlvmAst,
    function_name: &String,
    arguments: &Vec<usize>,
) -> usize {
    let function: &LlvmFunction = llvmAst_lookup_function(ast, string_clone(function_name));
    emu_execute_function(emulator, ast, function, arguments)
}

/// Execute the given function's body.
fn emu_execute_function(
    emulator: &mut Emu,
    ast: &LlvmAst,
    function: &LlvmFunction,
    arguments: &Vec<usize>,
) -> usize {
    let previous_frame_size: usize = emu_get_frame_size(emulator);
    emu_set_frame_size(emulator, 0);

    match function {
        LlvmFunction::BuiltIn(builtin, _, _) => {
            let value: usize = emu_execute_builtin(emulator, builtin, arguments);
            emu_set_frame_size(emulator, previous_frame_size);
            return value;
        },
        LlvmFunction::Function(_, parameters, blocks) => {
            let mut virtual_registers: StringMap<usize> = stringMap_new::<usize>();

            let mut i: usize = 0;
            while i < vec_len::<LlvmParameter>(parameters) {
                let parameter: &LlvmParameter = vec_at::<LlvmParameter>(parameters, i);
                let LlvmParameter::Parameter(name, _): &LlvmParameter = parameter;

                let value: &usize = vec_at::<usize>(arguments, i);
                stringMap_insert::<usize>(&mut virtual_registers, string_clone(name), *value);

                i = i + 1;
            }

            let mut current_label: String =
                string_clone(instructionBlock_label(vec_at::<InstructionBlock>(
                    blocks, 0,
                )));
            while true {
                let instructions: &Vec<Instruction> =
                    instructionBlock_fetch_instructions(blocks, string_clone(&current_label));

                let flow: LlvmExecFlow =
                    emu_execute_instructions(emulator, ast, &mut virtual_registers, instructions);

                match flow {
                    LlvmExecFlow::Continue => panic!("LLVM block did not terminate"),
                    LlvmExecFlow::Jump(next_label) => current_label = next_label,
                    LlvmExecFlow::Return(value) => {
                        emu_deallocate_stack_frame(emulator);
                        emu_set_frame_size(emulator, previous_frame_size);
                        return value;
                    },
                }
            }
            0 // satisfy compiler
        },
    }
}

/// Execute one builtin function and return its value.
fn emu_execute_builtin(emulator: &mut Emu, builtin: &LlvmBuiltIn, arguments: &Vec<usize>) -> usize {
    match builtin {
        LlvmBuiltIn::Malloc => {
            let value: usize = *vec_at::<usize>(arguments, 0);
            match emu_allocate_heap(emulator, value) {
                Option::Some(address) => address,
                Option::None => panic!("heap overflow of emu"),
            }
        },
        LlvmBuiltIn::Exit => {
            let value: usize = *vec_at::<usize>(arguments, 0);
            emu_set_exit_code(emulator, value);
            value
        },
    }
}

/// Execute a given list of instructions.
fn emu_execute_instructions(
    emulator: &mut Emu,
    ast: &LlvmAst,
    registers: &mut StringMap<usize>,
    instructions: &Vec<Instruction>,
) -> LlvmExecFlow {
    let mut i: usize = 0;
    while i < vec_len::<Instruction>(instructions) {
        let instruction: &Instruction = vec_at::<Instruction>(instructions, i);

        match instruction {
            Instruction::Assignment(assign_instruction) => {
                emu_execute_assignment(emulator, ast, registers, assign_instruction);
            },
            Instruction::Store(ty, value, address) => {
                emu_execute_store(emulator, registers, ty, value, address);
            },
            Instruction::Call(Call::Call(call_type, callee, arguments)) => {
                let _ = emu_execute_call(emulator, ast, registers, call_type, callee, arguments);
            },

            Instruction::Ret(return_type, return_value) => {
                return LlvmExecFlow::Return(match return_value {
                    Option::Some(value) => {
                        let value: usize = llvm_eval_value(emulator, registers, value);
                        llvm_overflow_value(value, &return_type)
                    },
                    Option::None => 0,
                });
            },
            Instruction::Br(branch) => {
                return match branch {
                    Branch::Unconditional(target_label) => {
                        LlvmExecFlow::Jump(string_clone(target_label))
                    },
                    Branch::Conditional(condition, then_label, else_label) => {
                        let condition_value: usize =
                            llvm_eval_value(emulator, registers, condition);

                        if condition_value == 1 {
                            LlvmExecFlow::Jump(string_clone(then_label))
                        } else {
                            LlvmExecFlow::Jump(string_clone(else_label))
                        }
                    },
                };
            },
        }

        match emu_exit_code(emulator) {
            Option::Some(code) => return LlvmExecFlow::Return(code),
            Option::None => {},
        }

        i = i + 1;
    }
    LlvmExecFlow::Continue
}

/// Execute the given assignment instruction.
fn emu_execute_assignment(
    emulator: &mut Emu,
    ast: &LlvmAst,
    registers: &mut StringMap<usize>,
    AssignInstruction::Assign(target, operation): &AssignInstruction,
) {
    let value: usize = emu_evaluate_assign_op(emulator, ast, registers, operation);
    stringMap_insert::<usize>(registers, string_clone(target), value);
}

/// Evaluate the value of the assignment operation.
fn emu_evaluate_assign_op(
    emulator: &mut Emu,
    ast: &LlvmAst,
    registers: &StringMap<usize>,
    operation: &AssignOp,
) -> usize {
    match operation {
        AssignOp::Binary(operator, result_type, left, right) => {
            let lhs: usize = llvm_eval_value(emulator, registers, left);
            let rhs: usize = llvm_eval_value(emulator, registers, right);
            let lhs: usize = llvm_overflow_value(lhs, result_type);
            let rhs: usize = llvm_overflow_value(rhs, result_type);

            match operator {
                BinaryOp::Add => lhs + rhs,
                BinaryOp::Sub => lhs - rhs,
                BinaryOp::Mul => lhs * rhs,
                BinaryOp::Udiv => lhs / rhs,
                BinaryOp::Urem => lhs % rhs,
            }
        },

        AssignOp::Icmp(predicate, operand_type, left, right) => {
            let mut lhs: usize = llvm_eval_value(emulator, registers, left);
            let mut rhs: usize = llvm_eval_value(emulator, registers, right);

            // handle overflowing literals
            lhs = llvm_overflow_value(lhs, operand_type);
            rhs = llvm_overflow_value(rhs, operand_type);

            let result: bool = match predicate {
                IcmpOp::Eq => lhs == rhs,
                IcmpOp::Ne => lhs != rhs,
                IcmpOp::Ugt => lhs > rhs,
                IcmpOp::Uge => lhs >= rhs,
                IcmpOp::Ult => lhs < rhs,
                IcmpOp::Ule => lhs <= rhs,
            };
            result as usize
        },

        AssignOp::Cast(cast_op, to_type, value) => {
            let evaluated_value: usize = llvm_eval_value(emulator, registers, value);
            match cast_op {
                CastOp::Zext => llvm_overflow_value(evaluated_value, to_type),
                CastOp::Trunc => llvm_overflow_value(evaluated_value, to_type),
                CastOp::IntToPtr => evaluated_value, // only interpretation changes
                CastOp::PtrToInt => evaluated_value, // only interpretation changes
            }
        },

        AssignOp::Alloca(allocated_type, num_elements) => {
            let space: usize = *num_elements * llvmType_size(allocated_type);
            match emu_allocate_stack(emulator, space) {
                Option::Some(address) => address,
                Option::None => panic!("Stack overflow of emu"),
            }
        },

        AssignOp::Load(loaded_type, address_value) => {
            let address: usize = llvm_eval_value(emulator, registers, address_value);
            match emu_load_bytes(emulator, address, llvmType_size(loaded_type)) {
                Option::Some(value) => llvm_overflow_value(value, loaded_type),
                Option::None => panic!("invalid LLVM load address"),
            }
        },

        AssignOp::Call(Call::Call(call_type, callee, arguments)) => {
            emu_execute_call(emulator, ast, registers, call_type, callee, arguments)
        },

        AssignOp::Gep(base_type, pointer, indexes) => {
            let mut address: usize = llvm_eval_value(emulator, registers, pointer);
            let mut current_type: LlvmType = llvmType_clone(base_type);

            let mut i: usize = 0;
            while i < vec_len::<LlvmTypedValue>(indexes) {
                let LlvmTypedValue::Pair(_, index_value): &LlvmTypedValue =
                    vec_at::<LlvmTypedValue>(indexes, i);
                let index: usize = llvm_eval_value(emulator, registers, index_value);

                address = address + index * llvmType_size(&current_type);
                current_type = match current_type {
                    LlvmType::Array(_, inner) => llvmType_clone(box_deref::<LlvmType>(&inner)),
                    other => other,
                };
                i = i + 1;
            }
            address
        },
    }
}

/// Execute an LLVM call and return the raw result value.
fn emu_execute_call(
    emulator: &mut Emu,
    ast: &LlvmAst,
    registers: &StringMap<usize>,
    call_type: &LlvmType,
    callee: &String,
    arguments: &Vec<LlvmTypedValue>,
) -> usize {
    let mut arg_values: Vec<usize> = vec_new::<usize>();
    let mut i: usize = 0;
    while i < vec_len::<LlvmTypedValue>(arguments) {
        let argument: &LlvmTypedValue = vec_at::<LlvmTypedValue>(arguments, i);
        let LlvmTypedValue::Pair(ty, argument_value): &LlvmTypedValue = argument;

        let value: usize = llvm_eval_value(emulator, registers, argument_value);
        let wrapped_value: usize = llvm_overflow_value(value, ty);
        vec_push::<usize>(&mut arg_values, wrapped_value);

        i = i + 1;
    }

    let value: usize = emu_execute_function_named(emulator, ast, callee, &arg_values);
    llvm_overflow_value(value, call_type)
}

/// Normalize a value so it wraps around according to the given type.
fn llvm_overflow_value(value: usize, ty: &LlvmType) -> usize {
    match ty {
        LlvmType::I1 => value % 2,
        LlvmType::I8 => value % 256,
        _ => value,
    }
}

/// Execute the given store instruction.
fn emu_execute_store(
    emulator: &mut Emu,
    registers: &StringMap<usize>,
    store_type: &LlvmType,
    value: &LlvmValue,
    address: &LlvmValue,
) {
    let raw_value: usize = llvm_eval_value(emulator, registers, value);
    let stored_value: usize = llvm_overflow_value(raw_value, store_type);
    let target_address: usize = llvm_eval_value(emulator, registers, address);
    let byte_count: usize = llvmType_size(store_type);

    if not(emu_store_bytes(
        emulator,
        target_address,
        stored_value,
        byte_count,
    )) {
        panic!("invalid LLVM store address");
    }
}

/// Evaluate the value of a virtual register, global name or literal.
fn llvm_eval_value(emulator: &Emu, registers: &StringMap<usize>, value: &LlvmValue) -> usize {
    match value {
        LlvmValue::Literal(number) => *number,
        LlvmValue::Register(name) => match stringMap_get::<usize>(registers, name) {
            Option::Some(register_value) => *register_value,
            Option::None => panic!("unknown LLVM register"),
        },
        LlvmValue::Global(name) => match stringMap_get::<usize>(emu_globals(emulator), name) {
            Option::Some(value) => *value,
            Option::None => panic!("unknown LLVM global value"),
        },
    }
}

// -----------------------------------------------------------------
// -----------------------------------------------------------------
// ------------------------- Library -------------------------------
// -----------------------------------------------------------------
// -----------------------------------------------------------------

// -------------------------- Math ---------------------------------

/// Return the maximum of two values.
fn max(n: usize, m: usize) -> usize {
    if n > m { n } else { m }
}

/// Return true if the number is negative (Two's Complement).
fn is_negative(number: usize) -> bool {
    number >= 9223372036854775808
}

// -------------------------- Error --------------------------------

/// Report an error message with source location and exit.
/// TODO: not subset-conform
fn report_error(file: &SourceFile, message: &String) -> ! {
    let line: usize = sourceFile_current_line(file);
    let col: usize = sourceFile_current_column(file);

    eprintln!("ERROR at {}:{}:", line, col);

    let mut start: usize = sourceFile_current_line_start(file);
    let mut reached_end: bool = false;
    while not(reached_end) {
        match sourceFile_get_char(file, start) {
            Option::Some('\n') => reached_end = true,
            Option::Some(c) => eprint!("{}", c),
            Option::None => reached_end = true,
        }
        start = start + 1;
    }
    eprintln!();

    let mut i: usize = 1;
    while i < col {
        eprint!(" ");
        i = i + 1;
    }
    eprint!("^ ");
    eprint_string(message);
    eprintln!();

    exit_process(1);
}

fn lexer_error(lexer: &Lexer, message: &String) -> ! {
    report_error(lexer_sourcefile(lexer), message)
}

/// Emit an error at the parser current location and abort.
fn parse_error(lexer: &Lexer, message: &String) -> ! {
    lexer_error(lexer, message)
}

fn codegen_error(message: &str) -> ! {
    panic!("Codegeneration error: {}", message)
}

fn semantic_check_error(message: &str) -> ! {
    panic!("Semantic error: {}", message);
}

/// Emit an LLVM parser error and panic.
fn llvmParser_error(parser: &LParser, message: &String) -> ! {
    let file: &SourceFile = llvmLexer_sourcefile(llvmParser_lexer(parser));
    report_error(file, message)
}

fn llvmParser_expected_message(parser: &LParser, expected: &String) -> String {
    let mut message: String = string("expected ");
    string_push_string(&mut message, expected);
    let token: &LlvmToken = llvmParser_current_token(parser);
    string_push_str(&mut message, ", but got: ");
    string_push_string(&mut message, &llvmToken_to_string(token));
    message
}

// -----------------------------------------------------------------
// -------------------------- bool ---------------------------------
// -----------------------------------------------------------------

/// Logical AND of two booleans.
fn and(a: bool, b: bool) -> bool {
    a as u8 + b as u8 == 2
}

/// Logical OR of two booleans.
fn or(a: bool, b: bool) -> bool {
    a as u8 + b as u8 > 0
}

/// Logical NOT of one boolean.
fn not(a: bool) -> bool {
    a as u8 == 0
}

// -----------------------------------------------------------------
// -------------------------- char ---------------------------------
// -----------------------------------------------------------------

/// Check whether a character is whitespace.
fn is_whitespace(c: char) -> bool {
    or(or(c == ' ', c == '\t'), or(c == '\n', c == '\r'))
}

/// Check whether a character is a decimal digit.
fn is_digit(c: char) -> bool {
    and(c >= '0', c <= '9')
}

/// Check whether a character is a hexadecimal digit.
/// Both upper and lowercase hexadecimal digits are considered valid.
fn is_hexadecimal_digit(c: char) -> bool {
    let upper: char = to_uppercase(c);
    or(is_digit(c), and(upper >= 'A', upper <= 'F'))
}

/// Check whether a character is a lowercase letter
fn is_lowercase(c: char) -> bool {
    and(c >= 'a', c <= 'z')
}

/// Check whether a character is an uppercase letter
fn is_uppercase(c: char) -> bool {
    and(c >= 'A', c <= 'Z')
}

/// Check whether a character is an alphabetical letter
fn is_letter(c: char) -> bool {
    or(is_lowercase(c), is_uppercase(c))
}

/// Check whether a character is alphabetic or underscore.
fn is_alpha(c: char) -> bool {
    or(is_letter(c), c == '_')
}

/// Check whether a character is alphanumeric.
fn is_alphanumeric(c: char) -> bool {
    or(is_alpha(c), is_digit(c))
}

/// Check whether a character is alphanumeric or '.'.
fn is_alphanumeric_or_dot(ch: char) -> bool {
    or(is_alphanumeric(ch), ch == '.')
}

/// Convert an ASCII character to uppercase.
/// If the character is not a letter, it is returned as is.
fn to_uppercase(c: char) -> char {
    if or(not(is_letter(c)), is_uppercase(c)) {
        c
    } else {
        (c as u8 - ('a' as u8 - 'A' as u8)) as char
    }
}

// -----------------------------------------------------------------
// ------------------------ Option<T> ------------------------------
// -----------------------------------------------------------------

/// Optional type that can contain some value with type T or no value.
enum Option<T> {
    Some(T),
    None,
}

/// Check whether an Option value is the None variant.
fn option_is_none<T>(opt: &Option<T>) -> bool {
    match opt {
        Option::Some(_) => false,
        Option::None => true,
    }
}

/// Returns the value wrapped in Some.
/// If the Option is None, end the program.
fn unwrap<T>(opt: Option<T>) -> T {
    match opt {
        Option::Some(value) => value,
        Option::None => panic!("tried to unwrap None variant of Option<T>"),
    }
}

// -----------------------------------------------------------------
// -------------------------- List --------------------------------
// -----------------------------------------------------------------

/// Generic cons list.
enum List<T> {
    /// head, tail
    Cons(T, Box<List<T>>),
    Nil,
}

// ----------------------------------------------------------------
// --------------------------- Box --------------------------------
// ----------------------------------------------------------------

/// Pointer to heap that owns its value.
#[derive(Debug)]
enum Box<T> {
    Ptr(*mut T),
}

/// Allocate and box a value on the heap.
fn box_new<T>(value: T) -> Box<T> {
    let ptr: *mut T = alloc::<T>(1);
    unsafe { *ptr = value };
    Box::Ptr(ptr)
}

/// Dereference a box.
fn box_deref<T>(Box::Ptr(ptr): &Box<T>) -> &T {
    unsafe { &**ptr }
}

// ----------------------------------------------------------------
// --------------------------- Vec --------------------------------
// ----------------------------------------------------------------

/// Generic contiguous growable buffer.
#[derive(Debug)]
enum Vec<T> {
    /// start, length, capacity
    Vec(*mut T, usize, usize),
}

/// Create an empty vector.
fn vec_new<T>() -> Vec<T> {
    vec_with_capacity::<T>(10)
}

/// Create a vector with fixed starting capacity.
fn vec_with_capacity<T>(initial_capacity: usize) -> Vec<T> {
    let capacity: usize = max(initial_capacity, 1);
    let ptr: *mut T = alloc::<T>(capacity);
    Vec::Vec(ptr, 0, capacity)
}

/// Create a vector with a fixed initial length.
fn vec_with_len<T>(len: usize) -> Vec<T> {
    let Vec::Vec(ptr, _, capacity) = vec_with_capacity(len);
    Vec::Vec(ptr, len, capacity)
}

/// Get the backing pointer.
fn vec_ptr<T>(Vec::Vec(ptr, _, _): &Vec<T>) -> *mut T {
    *ptr
}

/// Get the logical length.
fn vec_len<T>(Vec::Vec(_, len, _): &Vec<T>) -> usize {
    *len
}

/// Get the capacity.
fn vec_capacity<T>(Vec::Vec(_, _, capacity): &Vec<T>) -> usize {
    *capacity
}

/// Ensure capacity for extra elements.
fn vec_accomodate_extra_space<T>(vec: &mut Vec<T>, space: usize) {
    let len: usize = vec_len::<T>(vec);
    let capacity: usize = vec_capacity::<T>(vec);
    if capacity < len + space {
        let Vec::Vec(ptr, len_ref, capacity_ref): &mut Vec<T> = vec;
        *capacity_ref = *capacity_ref * 2;

        let new_ptr: *mut T = alloc::<T>(*capacity_ref) as *mut T;
        unsafe { memcopy::<T>(new_ptr, *ptr, *len_ref) };
        *ptr = new_ptr;
        // TODO: change this
        vec_accomodate_extra_space::<T>(vec, space); // if doubling was not enough, double again,
    }
}

/// Append one element.
fn vec_push<T>(vec: &mut Vec<T>, value: T) {
    vec_accomodate_extra_space::<T>(vec, 1);
    let Vec::Vec(ptr, len, _): &mut Vec<T> = vec;
    unsafe { *ptr_add::<T>(*ptr, *len) = value }
    *len = *len + 1;
}

/// Set vector length after writing raw bytes/elements.
fn vec_set_len<T>(Vec::Vec(_, old_len, _): &mut Vec<T>, len: usize) {
    *old_len = len;
}

/// Get an immutable reference to an element by index.
fn vec_get<'a, T>(vec: &'a Vec<T>, index: usize) -> Option<&'a T> {
    if index >= vec_len::<T>(vec) {
        Option::None
    } else {
        let ptr: *mut T = ptr_add::<T>(vec_ptr::<T>(vec), index);
        unsafe { Option::Some(&*ptr) }
    }
}

/// Get a mutable reference to an element by index.
fn vec_get_mut<'a, T>(vec: &'a mut Vec<T>, index: usize) -> Option<&'a mut T> {
    if index >= vec_len::<T>(vec) {
        Option::None
    } else {
        let ptr: *mut T = ptr_add::<T>(vec_ptr::<T>(vec), index);
        unsafe { Option::Some(&mut *ptr) }
    }
}

/// Get an immutable reference to an element by index.
/// Panics, if the index is out of bounds.
fn vec_at<T>(vec: &Vec<T>, index: usize) -> &T {
    unwrap::<&T>(vec_get::<T>(vec, index))
}

/// Set a value at the given index. Return false if the index is out of bounds.
fn vec_set<T>(vec: &mut Vec<T>, index: usize, value: T) -> bool {
    if index >= vec_len::<T>(vec) {
        false
    } else {
        let ptr: *mut T = vec_ptr::<T>(vec);
        let ptr: *mut T = ptr_add::<T>(ptr, index);
        unsafe {
            *ptr = value;
        }
        true
    }
}

/// Append all elements from one vector to another.
fn vec_extend<T>(vec: &mut Vec<T>, other: &Vec<T>) {
    let other_len: usize = vec_len::<T>(other);
    vec_accomodate_extra_space::<T>(vec, other_len);

    let len: usize = vec_len::<T>(vec);
    let dest: *mut T = ptr_add::<T>(vec_ptr::<T>(vec), len);
    let src: *mut T = vec_ptr::<T>(other);
    unsafe { memcopy::<T>(dest, src, other_len) };
    vec_set_len::<T>(vec, len + other_len);
}

/// Compare two vectors for equality using an element equality function.
fn vec_eq<T>(left: &Vec<T>, right: &Vec<T>, item_eq: fn(&T, &T) -> bool) -> bool {
    let left_len: usize = vec_len::<T>(left);
    let right_len: usize = vec_len::<T>(right);
    if left_len != right_len {
        return false;
    }

    let mut i: usize = 0;
    while i < left_len {
        let l: &T = vec_at::<T>(left, i);
        let r: &T = vec_at::<T>(right, i);
        if not(item_eq(l, r)) {
            return false;
        }

        i = i + 1;
    }
    true
}

// ----------------------------------------------------------------
// ------------------------ StringMap -----------------------------
// ----------------------------------------------------------------

/// Bucket entry for StringMap.
enum StringMapEntry<T> {
    Entry(String, T),
}

/// Get the key stored in one StringMapEntry.
fn stringMapEntry_get_key<T>(StringMapEntry::Entry(key, _): &StringMapEntry<T>) -> &String {
    key
}

/// Get the value stored in one StringMapEntry.
fn stringMapEntry_get_value<T>(StringMapEntry::Entry(_, value): &StringMapEntry<T>) -> &T {
    value
}

/// Hash map from String keys to generic values.
enum StringMap<T> {
    Map(Vec<List<StringMapEntry<T>>>),
}

/// Create a map with default len.
fn stringMap_new<T>() -> StringMap<T> {
    stringMap_with_len::<T>(1024)
}

/// Create a map with explicit len.
fn stringMap_with_len<T>(len: usize) -> StringMap<T> {
    let bucket_len: usize = if len == 0 { 1 } else { len };
    let mut buckets: Vec<List<StringMapEntry<T>>> =
        vec_with_capacity::<List<StringMapEntry<T>>>(bucket_len);
    let mut i: usize = 0;
    while i < bucket_len {
        vec_push::<List<StringMapEntry<T>>>(&mut buckets, List::Nil);
        i = i + 1;
    }
    StringMap::Map(buckets)
}

/// Insert a key/value pair by prepending it to the bucket list.
fn stringMap_insert<T>(StringMap::Map(buckets): &mut StringMap<T>, key: String, value: T) {
    let bucket_index: usize = { string_hash(&key, vec_len::<List<StringMapEntry<T>>>(buckets)) };

    let bucket: &mut List<StringMapEntry<T>> = unwrap::<&mut List<StringMapEntry<T>>>(
        vec_get_mut::<List<StringMapEntry<T>>>(buckets, bucket_index),
    );

    let mut old_bucket: List<StringMapEntry<T>> = List::Nil;
    unsafe {
        memcopy::<List<StringMapEntry<T>>>(
            &mut old_bucket as *mut List<StringMapEntry<T>>,
            bucket as *mut List<StringMapEntry<T>>,
            1,
        );
    }

    *bucket = List::Cons(
        StringMapEntry::Entry(key, value),
        box_new::<List<StringMapEntry<T>>>(old_bucket),
    );
}

/// Get a shared reference to the value for a key.
fn stringMap_get<'a, T>(map: &'a StringMap<T>, key: &String) -> Option<&'a T> {
    let StringMap::Map(buckets): &'a StringMap<T> = map;
    let bucket_index: usize = string_hash(key, vec_len::<List<StringMapEntry<T>>>(buckets));

    let maybe_bucket: Option<&List<StringMapEntry<T>>> =
        vec_get::<List<StringMapEntry<T>>>(buckets, bucket_index);
    if option_is_none::<&List<StringMapEntry<T>>>(&maybe_bucket) {
        return Option::None;
    }
    let mut bucket: &List<StringMapEntry<T>> = unwrap::<&List<StringMapEntry<T>>>(maybe_bucket);

    while true {
        match bucket {
            List::Cons(entry, tail) => {
                let other_key: &String = stringMapEntry_get_key::<T>(entry);
                if string_eq(other_key, key) {
                    return Option::Some(stringMapEntry_get_value::<T>(entry));
                }

                // repeat with next bucket
                bucket = box_deref::<List<StringMapEntry<T>>>(tail);
            },

            List::Nil => return Option::None,
        }
    }
    Option::None
}

/// Check whether a key exists.
fn stringMap_contains<T>(map: &StringMap<T>, key: &String) -> bool {
    match stringMap_get::<T>(map, key) {
        Option::Some(_) => true,
        Option::None => false,
    }
}

// ----------------------------------------------------------------
// ---------------------- StringMapStack --------------------------
// ----------------------------------------------------------------
// A stack of StringMap<T> which inserts/looks-up by stack order.

/// Stack of StringMap scopes.
enum StringMapStack<T> {
    Stack(Vec<StringMap<T>>, usize),
}

/// Create an empty StringMap stack.
fn stringMapStack_new<T>() -> StringMapStack<T> {
    StringMapStack::Stack(vec_new::<StringMap<T>>(), 0)
}

/// Push a new empty scope.
fn stringMapStack_push_empty<T>(StringMapStack::Stack(scopes, top): &mut StringMapStack<T>) {
    let new_scope: StringMap<T> = stringMap_new::<T>();
    if *top == vec_len::<StringMap<T>>(scopes) {
        vec_push::<StringMap<T>>(scopes, new_scope);
    } else {
        vec_set::<StringMap<T>>(scopes, *top, new_scope);
    }
    *top = *top + 1;
}

/// Pop the top scope.
fn stringMapStack_pop<T>(StringMapStack::Stack(_, top): &mut StringMapStack<T>) -> bool {
    if *top == 0 {
        false
    } else {
        *top = *top - 1;
        true
    }
}

/// Insert into the current scope and return whether the name already existed there.
fn stringMapStack_insert<T>(stack: &mut StringMapStack<T>, name: String, value: T) -> bool {
    let StringMapStack::Stack(scopes, top): &mut StringMapStack<T> = stack;
    if *top == 0 {
        return true;
    }

    let idx: usize = *top - 1;
    let scope: &mut StringMap<T> =
        unwrap::<&mut StringMap<T>>(vec_get_mut::<StringMap<T>>(scopes, idx));
    let already_used: bool = stringMap_contains::<T>(scope, &name);
    stringMap_insert::<T>(scope, name, value);
    already_used
}

/// Look up a value in any visible scope.
fn stringMapStack_lookup<'a, T>(stack: &'a StringMapStack<T>, name: &String) -> Option<&'a T> {
    let StringMapStack::Stack(scopes, top) = stack;
    let mut index: usize = *top;
    while index > 0 {
        index = index - 1;
        let scope: &StringMap<T> = unwrap::<&StringMap<T>>(vec_get::<StringMap<T>>(scopes, index));
        match stringMap_get::<T>(scope, name) {
            Option::Some(value) => return Option::Some(value),
            Option::None => {},
        }
    }
    Option::None
}

// ----------------------------------------------------------------
// --------------------------- Eq ---------------------------------
// ----------------------------------------------------------------

fn llvmType_eq(left: &LlvmType, right: &LlvmType) -> bool {
    match left {
        LlvmType::I1 => match right {
            LlvmType::I1 => true,
            _ => false,
        },
        LlvmType::I8 => match right {
            LlvmType::I8 => true,
            _ => false,
        },
        LlvmType::I64 => match right {
            LlvmType::I64 => true,
            _ => false,
        },
        LlvmType::Ptr => match right {
            LlvmType::Ptr => true,
            _ => false,
        },
        LlvmType::Array(left_len, left_inner) => match right {
            LlvmType::Array(right_len, right_inner) => {
                *left_len == *right_len
                    && llvmType_eq(
                        box_deref::<LlvmType>(left_inner),
                        box_deref::<LlvmType>(right_inner),
                    )
            },
            _ => false,
        },
        LlvmType::Void => match right {
            LlvmType::Void => true,
            _ => false,
        },
    }
}

/// Check if two tokens are equal.
fn token_eq(a: &Token, b: &Token) -> bool {
    match a {
        Token::Unsafe => match b {
            Token::Unsafe => true,
            _ => false,
        },
        Token::Fn => match b {
            Token::Fn => true,
            _ => false,
        },
        Token::Enum => match b {
            Token::Enum => true,
            _ => false,
        },
        Token::Extern => match b {
            Token::Extern => true,
            _ => false,
        },
        Token::Let => match b {
            Token::Let => true,
            _ => false,
        },
        Token::If => match b {
            Token::If => true,
            _ => false,
        },
        Token::Else => match b {
            Token::Else => true,
            _ => false,
        },
        Token::While => match b {
            Token::While => true,
            _ => false,
        },
        Token::Return => match b {
            Token::Return => true,
            _ => false,
        },
        Token::Match => match b {
            Token::Match => true,
            _ => false,
        },
        Token::As => match b {
            Token::As => true,
            _ => false,
        },
        Token::Mut => match b {
            Token::Mut => true,
            _ => false,
        },
        Token::Pipe => match b {
            Token::Pipe => true,
            _ => false,
        },
        Token::Ampersand => match b {
            Token::Ampersand => true,
            _ => false,
        },
        Token::LBrace => match b {
            Token::LBrace => true,
            _ => false,
        },
        Token::RBrace => match b {
            Token::RBrace => true,
            _ => false,
        },
        Token::LParen => match b {
            Token::LParen => true,
            _ => false,
        },
        Token::RParen => match b {
            Token::RParen => true,
            _ => false,
        },
        Token::Colon => match b {
            Token::Colon => true,
            _ => false,
        },
        Token::DoubleColon => match b {
            Token::DoubleColon => true,
            _ => false,
        },
        Token::SemiColon => match b {
            Token::SemiColon => true,
            _ => false,
        },
        Token::Comma => match b {
            Token::Comma => true,
            _ => false,
        },
        Token::Assign => match b {
            Token::Assign => true,
            _ => false,
        },
        Token::Bang => match b {
            Token::Bang => true,
            _ => false,
        },
        Token::Cmp(left_comparison) => match b {
            Token::Cmp(right_comparison) => comparison_eq(left_comparison, right_comparison),
            _ => false,
        },
        Token::FatArrow => match b {
            Token::FatArrow => true,
            _ => false,
        },
        Token::Plus => match b {
            Token::Plus => true,
            _ => false,
        },
        Token::Minus => match b {
            Token::Minus => true,
            _ => false,
        },
        Token::Star => match b {
            Token::Star => true,
            _ => false,
        },
        Token::Slash => match b {
            Token::Slash => true,
            _ => false,
        },
        Token::Remainder => match b {
            Token::Remainder => true,
            _ => false,
        },
        Token::Usize => match b {
            Token::Usize => true,
            _ => false,
        },
        Token::U8 => match b {
            Token::U8 => true,
            _ => false,
        },
        Token::Bool => match b {
            Token::Bool => true,
            _ => false,
        },
        Token::Char => match b {
            Token::Char => true,
            _ => false,
        },
        Token::Str => match b {
            Token::Str => true,
            _ => false,
        },
        Token::Arrow => match b {
            Token::Arrow => true,
            _ => false,
        },
        Token::Literal(left_literal) => match b {
            Token::Literal(right_literal) => literalToken_eq(left_literal, right_literal),
            _ => false,
        },
        Token::Identifier(left) => match b {
            Token::Identifier(right) => string_eq(left, right),
            _ => false,
        },
        Token::Eof => match b {
            Token::Eof => true,
            _ => false,
        },
    }
}

/// Check if two comparison tokens are equal.
fn comparison_eq(left: &Comparison, right: &Comparison) -> bool {
    match left {
        Comparison::Eq => match right {
            Comparison::Eq => true,
            _ => false,
        },
        Comparison::Ne => match right {
            Comparison::Ne => true,
            _ => false,
        },
        Comparison::Gt => match right {
            Comparison::Gt => true,
            _ => false,
        },
        Comparison::Lt => match right {
            Comparison::Lt => true,
            _ => false,
        },
        Comparison::Geq => match right {
            Comparison::Geq => true,
            _ => false,
        },
        Comparison::Leq => match right {
            Comparison::Leq => true,
            _ => false,
        },
    }
}

/// Check if two literal tokens are equal.
fn literalToken_eq(left: &Literal, right: &Literal) -> bool {
    match left {
        Literal::Int(left_value) => match right {
            Literal::Int(right_value) => left_value == right_value,
            _ => false,
        },
        Literal::String(left_value) => match right {
            Literal::String(right_value) => string_eq(left_value, right_value),
            _ => false,
        },
        Literal::Char(left_value) => match right {
            Literal::Char(right_value) => left_value == right_value,
            _ => false,
        },
        Literal::Bool(left_value) => match right {
            Literal::Bool(right_value) => left_value == right_value,
            _ => false,
        },
    }
}

/// Check two Rust AST types for equality.
fn rAstType_eq(a: &RAstType, b: &RAstType) -> bool {
    match a {
        RAstType::U8 => match b {
            RAstType::U8 => true,
            _ => false,
        },
        RAstType::Usize => match b {
            RAstType::Usize => true,
            _ => false,
        },
        RAstType::Bool => match b {
            RAstType::Bool => true,
            _ => false,
        },
        RAstType::Char => match b {
            RAstType::Char => true,
            _ => false,
        },
        RAstType::Unit => match b {
            RAstType::Unit => true,
            _ => false,
        },
        RAstType::Never => match b {
            RAstType::Never => true,
            _ => false,
        },
        RAstType::Custom(left) => match b {
            RAstType::Custom(right) => string_eq(left, right),
            _ => false,
        },
        RAstType::Reference(left, left_mut) => match b {
            RAstType::Reference(right, right_mut) => and(
                *left_mut == *right_mut,
                rAstType_eq(box_deref::<RAstType>(left), box_deref::<RAstType>(right)),
            ),
            _ => false,
        },
        RAstType::RawPointerMut(left) => match b {
            RAstType::RawPointerMut(right) => {
                rAstType_eq(box_deref::<RAstType>(left), box_deref::<RAstType>(right))
            },
            _ => false,
        },
    }
}

/// Check two LLVM tokens for equality.
fn llvmToken_eq(left: &LlvmToken, right: &LlvmToken) -> bool {
    match left {
        LlvmToken::Define => match right {
            LlvmToken::Define => true,
            _ => false,
        },
        LlvmToken::Declare => match right {
            LlvmToken::Declare => true,
            _ => false,
        },
        LlvmToken::Ret => match right {
            LlvmToken::Ret => true,
            _ => false,
        },
        LlvmToken::IntToPtr => match right {
            LlvmToken::IntToPtr => true,
            _ => false,
        },
        LlvmToken::PtrToInt => match right {
            LlvmToken::PtrToInt => true,
            _ => false,
        },
        LlvmToken::Br => match right {
            LlvmToken::Br => true,
            _ => false,
        },
        LlvmToken::Label => match right {
            LlvmToken::Label => true,
            _ => false,
        },
        LlvmToken::Add => match right {
            LlvmToken::Add => true,
            _ => false,
        },
        LlvmToken::Sub => match right {
            LlvmToken::Sub => true,
            _ => false,
        },
        LlvmToken::Mul => match right {
            LlvmToken::Mul => true,
            _ => false,
        },
        LlvmToken::Udiv => match right {
            LlvmToken::Udiv => true,
            _ => false,
        },
        LlvmToken::Urem => match right {
            LlvmToken::Urem => true,
            _ => false,
        },
        LlvmToken::Icmp => match right {
            LlvmToken::Icmp => true,
            _ => false,
        },
        LlvmToken::Zext => match right {
            LlvmToken::Zext => true,
            _ => false,
        },
        LlvmToken::Trunc => match right {
            LlvmToken::Trunc => true,
            _ => false,
        },
        LlvmToken::Alloca => match right {
            LlvmToken::Alloca => true,
            _ => false,
        },
        LlvmToken::Store => match right {
            LlvmToken::Store => true,
            _ => false,
        },
        LlvmToken::Load => match right {
            LlvmToken::Load => true,
            _ => false,
        },
        LlvmToken::To => match right {
            LlvmToken::To => true,
            _ => false,
        },
        LlvmToken::Call => match right {
            LlvmToken::Call => true,
            _ => false,
        },
        LlvmToken::Gep => match right {
            LlvmToken::Gep => true,
            _ => false,
        },
        LlvmToken::Constant => match right {
            LlvmToken::Constant => true,
            _ => false,
        },
        LlvmToken::Eq => match right {
            LlvmToken::Eq => true,
            _ => false,
        },
        LlvmToken::Ne => match right {
            LlvmToken::Ne => true,
            _ => false,
        },
        LlvmToken::Ugt => match right {
            LlvmToken::Ugt => true,
            _ => false,
        },
        LlvmToken::Uge => match right {
            LlvmToken::Uge => true,
            _ => false,
        },
        LlvmToken::Ult => match right {
            LlvmToken::Ult => true,
            _ => false,
        },
        LlvmToken::Ule => match right {
            LlvmToken::Ule => true,
            _ => false,
        },
        LlvmToken::Ptr => match right {
            LlvmToken::Ptr => true,
            _ => false,
        },
        LlvmToken::I64 => match right {
            LlvmToken::I64 => true,
            _ => false,
        },
        LlvmToken::I8 => match right {
            LlvmToken::I8 => true,
            _ => false,
        },
        LlvmToken::I1 => match right {
            LlvmToken::I1 => true,
            _ => false,
        },
        LlvmToken::Void => match right {
            LlvmToken::Void => true,
            _ => false,
        },
        LlvmToken::At => match right {
            LlvmToken::At => true,
            _ => false,
        },
        LlvmToken::Percent => match right {
            LlvmToken::Percent => true,
            _ => false,
        },
        LlvmToken::LParen => match right {
            LlvmToken::LParen => true,
            _ => false,
        },
        LlvmToken::RParen => match right {
            LlvmToken::RParen => true,
            _ => false,
        },
        LlvmToken::LBrace => match right {
            LlvmToken::LBrace => true,
            _ => false,
        },
        LlvmToken::RBrace => match right {
            LlvmToken::RBrace => true,
            _ => false,
        },
        LlvmToken::LBracket => match right {
            LlvmToken::LBracket => true,
            _ => false,
        },
        LlvmToken::RBracket => match right {
            LlvmToken::RBracket => true,
            _ => false,
        },
        LlvmToken::Comma => match right {
            LlvmToken::Comma => true,
            _ => false,
        },
        LlvmToken::Assign => match right {
            LlvmToken::Assign => true,
            _ => false,
        },
        LlvmToken::Colon => match right {
            LlvmToken::Colon => true,
            _ => false,
        },
        LlvmToken::CString(left_value) => match right {
            LlvmToken::CString(right_value) => string_eq(left_value, right_value),
            _ => false,
        },
        LlvmToken::Identifier(left_name) => match right {
            LlvmToken::Identifier(right_name) => string_eq(left_name, right_name),
            _ => false,
        },
        LlvmToken::Integer(left_value) => match right {
            LlvmToken::Integer(right_value) => *left_value == *right_value,
            _ => false,
        },
        LlvmToken::Eof => match right {
            LlvmToken::Eof => true,
            _ => false,
        },
    }
}

/// Check if two strings are equal.
fn string_eq(s1: &String, s2: &String) -> bool {
    let len: usize = string_len(s1);
    if len != string_len(s2) {
        return false;
    }

    let mut i: usize = 0;
    while i < len {
        let c1: char = unwrap::<char>(string_get(s1, i));
        let c2: char = unwrap::<char>(string_get(s2, i));
        if c1 != c2 {
            return false;
        }

        i = i + 1;
    }

    true
}

// ----------------------------------------------------------------
// ------------------------- Clone --------------------------------
// ----------------------------------------------------------------

/// Clone a token value.
fn token_clone(token: &Token) -> Token {
    match token {
        Token::Unsafe => Token::Unsafe,
        Token::Fn => Token::Fn,
        Token::Enum => Token::Enum,
        Token::Extern => Token::Extern,
        Token::Let => Token::Let,
        Token::If => Token::If,
        Token::Else => Token::Else,
        Token::While => Token::While,
        Token::Return => Token::Return,
        Token::Match => Token::Match,
        Token::As => Token::As,
        Token::Mut => Token::Mut,
        Token::Ampersand => Token::Ampersand,
        Token::LBrace => Token::LBrace,
        Token::RBrace => Token::RBrace,
        Token::LParen => Token::LParen,
        Token::RParen => Token::RParen,
        Token::Colon => Token::Colon,
        Token::DoubleColon => Token::DoubleColon,
        Token::SemiColon => Token::SemiColon,
        Token::Comma => Token::Comma,
        Token::Pipe => Token::Pipe,
        Token::Assign => Token::Assign,
        Token::Bang => Token::Bang,
        Token::Cmp(comparison) => Token::Cmp(comparison_clone(comparison)),
        Token::FatArrow => Token::FatArrow,
        Token::Plus => Token::Plus,
        Token::Minus => Token::Minus,
        Token::Star => Token::Star,
        Token::Slash => Token::Slash,
        Token::Remainder => Token::Remainder,
        Token::Usize => Token::Usize,
        Token::U8 => Token::U8,
        Token::Bool => Token::Bool,
        Token::Char => Token::Char,
        Token::Str => Token::Str,
        Token::Arrow => Token::Arrow,
        Token::Literal(literal) => Token::Literal(literalToken_clone(literal)),
        Token::Identifier(value) => Token::Identifier(string_clone(value)),
        Token::Eof => Token::Eof,
    }
}

/// Clone a comparison operator.
fn comparison_clone(comparison: &Comparison) -> Comparison {
    match comparison {
        Comparison::Eq => Comparison::Eq,
        Comparison::Ne => Comparison::Ne,
        Comparison::Gt => Comparison::Gt,
        Comparison::Lt => Comparison::Lt,
        Comparison::Geq => Comparison::Geq,
        Comparison::Leq => Comparison::Leq,
    }
}

/// Clone a literal token payload.
fn literalToken_clone(literal: &Literal) -> Literal {
    match literal {
        Literal::Int(value) => Literal::Int(*value),
        Literal::String(value) => Literal::String(string_clone(value)),
        Literal::Char(value) => Literal::Char(*value),
        Literal::Bool(value) => Literal::Bool(*value),
    }
}

/// Clone a function signature.
fn fnSignature_clone(signature: &FnSignature) -> FnSignature {
    match signature {
        FnSignature::Fn(parameter_types, return_type, is_unsafe) => {
            let mut cloned_params: Vec<RAstType> = vec_new::<RAstType>();
            let mut i: usize = 0;
            while i < vec_len::<RAstType>(parameter_types) {
                let param: &RAstType = vec_at::<RAstType>(parameter_types, i);
                vec_push::<RAstType>(&mut cloned_params, rAstType_clone(param));
                i = i + 1;
            }
            FnSignature::Fn(cloned_params, rAstType_clone(return_type), *is_unsafe)
        },
    }
}

/// Clone a Rust AST type value.
fn rAstType_clone(t: &RAstType) -> RAstType {
    match t {
        RAstType::U8 => RAstType::U8,
        RAstType::Usize => RAstType::Usize,
        RAstType::Bool => RAstType::Bool,
        RAstType::Char => RAstType::Char,
        RAstType::Unit => RAstType::Unit,
        RAstType::Never => RAstType::Never,
        RAstType::Custom(name) => RAstType::Custom(string_clone(name)),
        RAstType::Reference(inner, mutable) => RAstType::Reference(
            box_new::<RAstType>(rAstType_clone(box_deref::<RAstType>(inner))),
            *mutable,
        ),
        RAstType::RawPointerMut(inner) => RAstType::RawPointerMut(box_new::<RAstType>(
            rAstType_clone(box_deref::<RAstType>(inner)),
        )),
    }
}

/// Clone a STPair
fn stPair_clone(STPair::ST(string, ty): &STPair) -> STPair {
    STPair::ST(string_clone(string), rAstType_clone(ty))
}

/// Clone an LLVM token.
fn llvmToken_clone(token: &LlvmToken) -> LlvmToken {
    match token {
        LlvmToken::Define => LlvmToken::Define,
        LlvmToken::Declare => LlvmToken::Declare,
        LlvmToken::Ret => LlvmToken::Ret,
        LlvmToken::IntToPtr => LlvmToken::IntToPtr,
        LlvmToken::PtrToInt => LlvmToken::PtrToInt,
        LlvmToken::Br => LlvmToken::Br,
        LlvmToken::Label => LlvmToken::Label,
        LlvmToken::Add => LlvmToken::Add,
        LlvmToken::Sub => LlvmToken::Sub,
        LlvmToken::Mul => LlvmToken::Mul,
        LlvmToken::Udiv => LlvmToken::Udiv,
        LlvmToken::Urem => LlvmToken::Urem,
        LlvmToken::Icmp => LlvmToken::Icmp,
        LlvmToken::Zext => LlvmToken::Zext,
        LlvmToken::Trunc => LlvmToken::Trunc,
        LlvmToken::Alloca => LlvmToken::Alloca,
        LlvmToken::Store => LlvmToken::Store,
        LlvmToken::Load => LlvmToken::Load,
        LlvmToken::To => LlvmToken::To,
        LlvmToken::Call => LlvmToken::Call,
        LlvmToken::Gep => LlvmToken::Gep,
        LlvmToken::Constant => LlvmToken::Constant,
        LlvmToken::Eq => LlvmToken::Eq,
        LlvmToken::Ne => LlvmToken::Ne,
        LlvmToken::Ugt => LlvmToken::Ugt,
        LlvmToken::Uge => LlvmToken::Uge,
        LlvmToken::Ult => LlvmToken::Ult,
        LlvmToken::Ule => LlvmToken::Ule,
        LlvmToken::Ptr => LlvmToken::Ptr,
        LlvmToken::I64 => LlvmToken::I64,
        LlvmToken::I8 => LlvmToken::I8,
        LlvmToken::I1 => LlvmToken::I1,
        LlvmToken::Void => LlvmToken::Void,
        LlvmToken::At => LlvmToken::At,
        LlvmToken::Percent => LlvmToken::Percent,
        LlvmToken::LParen => LlvmToken::LParen,
        LlvmToken::RParen => LlvmToken::RParen,
        LlvmToken::LBrace => LlvmToken::LBrace,
        LlvmToken::RBrace => LlvmToken::RBrace,
        LlvmToken::LBracket => LlvmToken::LBracket,
        LlvmToken::RBracket => LlvmToken::RBracket,
        LlvmToken::Comma => LlvmToken::Comma,
        LlvmToken::Assign => LlvmToken::Assign,
        LlvmToken::Colon => LlvmToken::Colon,
        LlvmToken::CString(value) => LlvmToken::CString(string_clone(value)),
        LlvmToken::Identifier(name) => LlvmToken::Identifier(string_clone(name)),
        LlvmToken::Integer(value) => LlvmToken::Integer(*value),
        LlvmToken::Eof => LlvmToken::Eof,
    }
}

/// Clone an LLVM type.
fn llvmType_clone(ty: &LlvmType) -> LlvmType {
    match ty {
        LlvmType::I1 => LlvmType::I1,
        LlvmType::I8 => LlvmType::I8,
        LlvmType::I64 => LlvmType::I64,
        LlvmType::Ptr => LlvmType::Ptr,
        LlvmType::Array(len, inner) => LlvmType::Array(
            *len,
            box_new::<LlvmType>(llvmType_clone(box_deref::<LlvmType>(inner))),
        ),
        LlvmType::Void => LlvmType::Void,
    }
}

/// Clone a string.
fn string_clone(string: &String) -> String {
    let len: usize = string_len(string);

    let mut clone: String = string_with_capacity(len);
    let mut i: usize = 0;
    while i < len {
        let character: char = unwrap::<char>(string_get(string, i));
        string_push(&mut clone, character);
        i = i + 1;
    }
    clone
}

// ------------------------- String -------------------------------

/// A growable ASCII string.
#[derive(Debug)]
enum String {
    Inner(Vec<u8>),
}

/// Create a new empty string.
fn string_new() -> String {
    string_with_capacity(10)
}

/// Create a new string with the specified capacity
fn string_with_capacity(initial_capacity: usize) -> String {
    String::Inner(vec_with_capacity::<u8>(initial_capacity))
}

/// Create an owned string from a string slice.
fn string(str: &str) -> String {
    let mut s: String = string_new();
    string_push_str(&mut s, str);
    s
}

/// Get the length of the string.
fn string_len(String::Inner(bytes): &String) -> usize {
    vec_len::<u8>(bytes)
}

/// Get the character at the given index.
fn string_get(String::Inner(bytes): &String, index: usize) -> Option<char> {
    match vec_get::<u8>(bytes, index) {
        Option::Some(value) => Option::Some(*value as char),
        Option::None => Option::None,
    }
}

/// Get the character at the given index and panic if the index is out of bounds.
fn string_at(String::Inner(bytes): &String, index: usize) -> char {
    *vec_at::<u8>(bytes, index) as char
}

/// Set a character in a string. Return false if the index is out of bounds.
fn string_set(String::Inner(vec): &mut String, index: usize, character: char) -> bool {
    vec_set::<u8>(vec, index, character as u8)
}

/// Append a character to the string.
fn string_push(String::Inner(bytes): &mut String, character: char) {
    vec_push::<u8>(bytes, character as u8);
}

/// Append a string slice to the string.
fn string_push_str(String::Inner(bytes): &mut String, str: &str) {
    let str_len: usize = str::len(str);
    vec_accomodate_extra_space::<u8>(bytes, str_len);

    let str_ptr: *mut u8 = str::as_ptr(str) as *mut u8;
    let len: usize = vec_len::<u8>(bytes);
    let dest: *mut u8 = ptr_add::<u8>(vec_ptr::<u8>(bytes), len);

    unsafe { memcopy::<u8>(dest, str_ptr, str_len) }
    vec_set_len::<u8>(bytes, len + str_len);
}

/// Push a string onto another string.
fn string_push_string(String::Inner(bytes): &mut String, String::Inner(other_bytes): &String) {
    vec_extend::<u8>(bytes, other_bytes);
}

/// Converts a string into an integer given the base.
/// Returns None if the integer contained in the string is invalid for 64-bit integers.
fn string_to_integer(string: &String, base: usize) -> Option<usize> {
    let mut value: usize = 0;

    let mut i: usize = 0;
    while i < string_len(string) {
        let digit: char = unwrap::<char>(string_get(string, i));

        let digit_value: usize = if is_digit(digit) {
            digit as usize - '0' as usize
        } else {
            digit as usize - 'A' as usize + 10
        };

        let max: usize = 18446744073709551615; // 2^64 - 1

        if or(digit_value > base - 1, value > max / base) {
            return Option::None;
        }

        value = value * base + digit_value;

        i = i + 1;
    }
    Option::Some(value)
}

/// Reverse a String in place.
fn string_reverse(string: &mut String) {
    let len: usize = string_len(string);
    let mut i: usize = 0;
    while i < len / 2 {
        let a: char = unwrap::<char>(string_get(string, i));
        let b: char = unwrap::<char>(string_get(string, len - 1 - i));
        string_set(string, i, b);
        string_set(string, len - 1 - i, a);
        i = i + 1;
    }
}

/// Hash a String.
fn string_hash(string: &String, bucket_count: usize) -> usize {
    if bucket_count == 0 {
        return 0;
    }

    let mut hash: usize = 0;
    let mut i: usize = 0;
    while i < string_len(string) {
        let character: usize = unwrap::<char>(string_get(string, i)) as usize;
        hash = hash * 67 + character;
        i = i + 1;
    }
    hash % bucket_count
}

// -------------------- Display (to_string()) ----------------------

/// Convert an integer into a string.
fn integer_to_string(mut integer: usize) -> String {
    let mut string: String = string_new();

    if integer == 0 {
        string_push(&mut string, '0');
        return string;
    }

    while integer > 0 {
        let digit: u8 = (integer % 10) as u8;
        let character: char = ('0' as u8 + digit) as char;
        string_push(&mut string, character);
        integer = integer / 10;
    }

    string_reverse(&mut string);
    string
}

/// Convert a token into a string.
fn token_to_string(token: &Token) -> String {
    match token {
        Token::Fn => string("fn"),
        Token::Enum => string("enum"),
        Token::Extern => string("extern"),
        Token::Let => string("let"),
        Token::If => string("if"),
        Token::Else => string("else"),
        Token::While => string("while"),
        Token::Return => string("return"),
        Token::Match => string("match"),
        Token::As => string("as"),
        Token::Unsafe => string("unsafe"),
        Token::Mut => string("mut"),
        Token::Ampersand => string("&"),
        Token::LBrace => string("{"),
        Token::RBrace => string("}"),
        Token::LParen => string("("),
        Token::RParen => string(")"),
        Token::Colon => string(":"),
        Token::DoubleColon => string("::"),
        Token::SemiColon => string(";"),
        Token::Comma => string(","),
        Token::Pipe => string("|"),
        Token::Assign => string("="),
        Token::Bang => string("!"),
        Token::Cmp(comparison) => comparison_to_string(comparison),
        Token::FatArrow => string("=>"),
        Token::Plus => string("+"),
        Token::Minus => string("-"),
        Token::Star => string("*"),
        Token::Slash => string("/"),
        Token::Remainder => string("%"),
        Token::Usize => string("usize"),
        Token::U8 => string("u8"),
        Token::Bool => string("bool"),
        Token::Char => string("char"),
        Token::Str => string("str"),
        Token::Arrow => string("->"),
        Token::Literal(literal) => literal_to_string(literal),
        Token::Identifier(name) => string_clone(name),
        Token::Eof => string("<eof>"),
    }
}

/// Convert an LLVM token into a string.
fn llvmToken_to_string(token: &LlvmToken) -> String {
    match token {
        LlvmToken::Define => string("define"),
        LlvmToken::Declare => string("declare"),
        LlvmToken::Ret => string("ret"),
        LlvmToken::IntToPtr => string("inttoptr"),
        LlvmToken::PtrToInt => string("ptrtoint"),
        LlvmToken::Br => string("br"),
        LlvmToken::Label => string("label"),
        LlvmToken::Add => string("add"),
        LlvmToken::Sub => string("sub"),
        LlvmToken::Mul => string("mul"),
        LlvmToken::Udiv => string("udiv"),
        LlvmToken::Urem => string("urem"),
        LlvmToken::Icmp => string("icmp"),
        LlvmToken::Zext => string("zext"),
        LlvmToken::Trunc => string("trunc"),
        LlvmToken::Alloca => string("alloca"),
        LlvmToken::Store => string("store"),
        LlvmToken::Load => string("load"),
        LlvmToken::To => string("to"),
        LlvmToken::Call => string("call"),
        LlvmToken::Gep => string("getelementptr"),
        LlvmToken::Constant => string("constant"),
        LlvmToken::Eq => string("eq"),
        LlvmToken::Ne => string("ne"),
        LlvmToken::Ugt => string("ugt"),
        LlvmToken::Uge => string("uge"),
        LlvmToken::Ult => string("ult"),
        LlvmToken::Ule => string("ule"),
        LlvmToken::Ptr => string("ptr"),
        LlvmToken::I64 => string("i64"),
        LlvmToken::I8 => string("i8"),
        LlvmToken::I1 => string("i1"),
        LlvmToken::Void => string("void"),
        LlvmToken::At => string("@"),
        LlvmToken::Percent => string("%"),
        LlvmToken::LParen => string("("),
        LlvmToken::RParen => string(")"),
        LlvmToken::LBrace => string("{"),
        LlvmToken::RBrace => string("}"),
        LlvmToken::LBracket => string("["),
        LlvmToken::RBracket => string("]"),
        LlvmToken::Comma => string(","),
        LlvmToken::Assign => string("="),
        LlvmToken::Colon => string(":"),
        LlvmToken::CString(value) => {
            let mut string: String = string_new();
            string_push_str(&mut string, "c\"");
            string_push_string(&mut string, value);
            string_push(&mut string, '"');
            string
        },
        LlvmToken::Identifier(name) => string_clone(name),
        LlvmToken::Integer(value) => integer_to_string(*value),
        LlvmToken::Eof => string("<eof>"),
    }
}

/// Convert a comparison token into a string.
fn comparison_to_string(comparison: &Comparison) -> String {
    match comparison {
        Comparison::Eq => string("=="),
        Comparison::Ne => string("!="),
        Comparison::Gt => string(">"),
        Comparison::Lt => string("<"),
        Comparison::Geq => string(">="),
        Comparison::Leq => string("<="),
    }
}

/// Convert a literal token into a string.
fn literal_to_string(literal: &Literal) -> String {
    match literal {
        Literal::Int(value) => integer_to_string(*value),
        Literal::Bool(value) => {
            if *value {
                string("true")
            } else {
                string("false")
            }
        },
        Literal::Char(value) => {
            let mut string: String = string_new();
            string_push(&mut string, '\'');
            string_push(&mut string, *value);
            string_push(&mut string, '\'');
            string
        },
        Literal::String(value) => {
            let mut string: String = string_new();
            string_push(&mut string, '"');
            string_push_string(&mut string, value);
            string_push(&mut string, '"');
            string
        },
    }
}

// --------------------------- I/O ---------------------------------

enum IOResult {
    Success,
    OpenFailure,
    WriteFailure,
}

/// Print a string to stdout.
fn print_string(String::Inner(bytes): &String) {
    let len: usize = vec_len::<u8>(bytes);
    let ptr: *mut u8 = vec_ptr::<u8>(bytes);
    unsafe { io_write_report_error("/dev/stdout\0", ptr, len) }
}

/// Print a string to stderr.
fn eprint_string(String::Inner(bytes): &String) {
    let len: usize = vec_len::<u8>(bytes);
    let ptr: *mut u8 = vec_ptr::<u8>(bytes);
    unsafe { io_write_report_error("/dev/stderr\0", ptr, len) }
}

/// Print a string slice to stdout.
fn print_str(text: &str) {
    let len: usize = str::len(text);
    let ptr: *mut u8 = str::as_ptr(text) as *mut u8;
    unsafe { io_write_report_error("/dev/stdout\0", ptr, len) }
}

/// Print a string slice to stderr.
fn eprint_str(text: &str) {
    let len: usize = str::len(text);
    let ptr: *mut u8 = str::as_ptr(text) as *mut u8;
    unsafe { io_write_report_error("/dev/stderr\0", ptr, len) }
}

/// Write the given `buffer` to `path` and report an error if there was one.
/// `path` must be a NULL-terminated string.
unsafe fn io_write_report_error(path: &str, buffer_ptr: *mut u8, len: usize) {
    let path_ptr: *mut u8 = str::as_ptr(path) as *mut u8;
    match unsafe { io_write(path_ptr, buffer_ptr, len) } {
        IOResult::OpenFailure => eprint_str("Could not open \n"),
        IOResult::WriteFailure => eprint_str("Could not write to \n"),
        _ => return,
    };
    eprint_str(path);
    eprint_str("\n");
    exit_process(1);
}

/// Write the given `buffer` to `path` and return an IOResult.
/// `path` must be a NULL-terminated string.
unsafe fn io_write(path: *mut u8, buffer: *mut u8, len: usize) -> IOResult {
    let fd: usize = unsafe { open(path, 1) };
    if is_negative(fd) {
        return IOResult::OpenFailure;
    }

    let mut offset: usize = 0;
    while offset < len {
        let remaining: usize = len - offset;
        let written: usize = unsafe { write(fd, ptr_add::<u8>(buffer, offset), remaining) };
        if or(is_negative(written), written == 0) {
            return IOResult::WriteFailure;
        }
        offset = offset + written;
    }

    IOResult::Success
}

// ------------------------- Memory -------------------------------

/// Copy n bytes from src to dest.
///
/// It must hold: forall 0 <= i < n, dest[i] can be written
/// and src[i] can be read safely.
unsafe fn memcopy<T>(dest: *mut T, src: *mut T, n: usize) {
    let byte_count: usize = n * size_of::<T>();
    let dest_u8: *mut u8 = dest as *mut u8;
    let src_u8: *mut u8 = src as *mut u8;
    let mut i: usize = 0;
    while i < byte_count {
        unsafe {
            *ptr_add::<u8>(dest_u8, i) = *ptr_add::<u8>(src_u8, i);
        }
        i = i + 1;
    }
}

/// Increment a pointer by n elements.
fn ptr_add<T>(ptr: *mut T, n: usize) -> *mut T {
    (ptr as usize + n * size_of::<T>()) as *mut T
}

/// Heap-allocate memory for `count` T and return a pointer to the beginning of the memory block.
/// The returned pointer is never null and the memory is always zeroed.
/// The caller should cast the returned pointer to the desired type.
fn alloc<T>(count: usize) -> *mut T {
    unsafe {
        let p: *mut u8 = malloc(size_of::<T>() * count);

        if p as usize == 0 {
            eprintln!("Heap Memory Allocation Error!");
            exit(1);
        }

        let mut i = 0;
        while i < size_of::<T>() * count {
            *ptr_add(p, i) = 0;
            i = i + 1;
        }

        p as *mut T
    }
}

/// Exit the current process.
fn exit_process(code: usize) -> ! {
    unsafe { exit(code) }
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut u8;
    fn exit(code: usize) -> !;
    fn open(path: *mut u8, flags: usize) -> usize;
    fn write(fd: usize, buf: *mut u8, count: usize) -> usize;
}

// -----------------------------------------------------------------
// -------------------------- Tests --------------------------------
// -----------------------------------------------------------------

include!("tests.rs");
