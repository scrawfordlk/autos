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
    let mut lexer: RLexer = rLexer_new(string(source));
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
enum RToken {
    Fn,                 // "fn"
    Enum,               // "enum"
    Extern,             // "extern"
    Let,                // "let"
    If,                 // "if"
    Else,               // "else"
    While,              // "while"
    Return,             // "return"
    Match,              // "match"
    As,                 // "as"
    Unsafe,             // "unsafe"
    Mut,                // "mut"
    Ampersand,          // "&"
    LBrace,             // "{"
    RBrace,             // "}"
    LParen,             // "("
    RParen,             // ")"
    Colon,              // ":"
    DoubleColon,        // "::"
    SemiColon,          // ";"
    Comma,              // ","
    Pipe,               // "|"
    Assign,             // "="
    Bang,               // "!"
    Cmp(RComparisonOp), // ==, !=, <, <=, >, >=
    FatArrow,           // "=>"
    Plus,               // "+"
    Minus,              // "-"
    Star,               // "*"
    Slash,              // "/"
    Remainder,          // "%"
    Usize,              // "usize"
    U8,                 // "u8"
    Bool,               // "bool"
    Char,               // "char"
    Str,                // "str"
    Arrow,              // "->"
    Literal(RLiteral),
    Identifier(String),
    Eof,
}

/// Comparison tokens
#[derive(Debug)]
enum RComparisonOp {
    Eq,
    Ne,
    Gt,
    Lt,
    Geq,
    Leq,
}

/// Literal tokens.
#[derive(Debug)]
enum RLiteral {
    Int(usize),
    String(String),
    Char(char),
    Bool(bool),
}

/// The lexer state of the scanned Rust program.
enum RLexer {
    /// source file, current token
    Lexer(SourceFile, RToken),
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
fn rLexer_new(source: String) -> RLexer {
    let source_file: SourceFile = SourceFile::SourceFile(source, 0, 0, 0);
    let mut lexer: RLexer = RLexer::Lexer(source_file, RToken::Eof);
    rLexer_next_token(&mut lexer);
    lexer
}

/// Get immutable access to the lexer source file state.
fn rLexer_sourcefile(RLexer::Lexer(source, _): &RLexer) -> &SourceFile {
    source
}

/// Get mutable access to the lexer source file state.
fn rLexer_sourcefile_mut(RLexer::Lexer(source, _): &mut RLexer) -> &mut SourceFile {
    source
}

/// Get the current token from the lexer.
fn rLexer_current_token(RLexer::Lexer(_, token): &RLexer) -> &RToken {
    token
}

/// Get mutable access to the current lexer token slot.
fn rLexer_set_current_token(RLexer::Lexer(_, old_token): &mut RLexer, token: RToken) {
    *old_token = token;
}

/// Check whether the current token equals `token`.
fn rLexer_current_token_eq(lexer: &RLexer, token: &RToken) -> bool {
    token_eq(rLexer_current_token(lexer), token)
}

/// Peek at the next character without consuming it.
fn rLexer_peek_char(lexer: &RLexer) -> Option<char> {
    let SourceFile::SourceFile(string, index, _, _): &SourceFile = rLexer_sourcefile(lexer);
    string_get(string, *index)
}

/// Consume and return the next character.
fn rLexer_consume_char(lexer: &mut RLexer) -> Option<char> {
    let SourceFile::SourceFile(source, index, line, last_newline_idx): &mut SourceFile =
        rLexer_sourcefile_mut(lexer);

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
fn rLexer_try_consume(lexer: &mut RLexer, token: &RToken) -> bool {
    if rLexer_current_token_eq(lexer, token) {
        rLexer_next_token(lexer);
        true
    } else {
        false
    }
}

/// Consume the next character, erroring if it doesn't match expected.
fn rLexer_expect_char(lexer: &mut RLexer, expected: char) {
    match rLexer_consume_char(lexer) {
        Option::Some(c) => {
            if c != expected {
                let mut message: String = string("unexpected character: ");
                string_push_string(&mut message, &rLiteral_to_string(&RLiteral::Char(c)));
                lexer_error(lexer, &message);
            }
        },
        Option::None => lexer_error(lexer, &string("unexpected end of input")),
    }
}

// ---------------------- Lexer ----------------------

/// Consume and return the next token.
fn rLexer_next_token(lexer: &mut RLexer) -> RToken {
    rLexer_skip_attributes(lexer);
    rLexer_skip_whitespace(lexer);

    let token: RToken = match rLexer_peek_char(lexer) {
        Option::Some(c) => {
            if is_alpha(c) {
                let ident: String = rLexer_scan_identifier(lexer);
                rust_identifier_to_token(ident)
            } else if is_digit(c) {
                let value: usize = rLexer_scan_integer(lexer);
                RToken::Literal(RLiteral::Int(value))
            } else if c == '\'' {
                let ch: char = rLexer_scan_char_literal(lexer);
                RToken::Literal(RLiteral::Char(ch))
            } else if c == '"' {
                let s: String = rLexer_scan_string_literal(lexer);
                RToken::Literal(RLiteral::String(s))
            } else {
                rLexer_scan_symbol(lexer)
            }
        },
        Option::None => RToken::Eof,
    };

    rLexer_set_current_token(lexer, token_clone(&token));
    token
}

/// Scan an identifier or keyword.
fn rLexer_scan_identifier(lexer: &mut RLexer) -> String {
    let mut ident: String = string_new();
    while true {
        match rLexer_peek_char(lexer) {
            Option::Some(c) => {
                if is_alphanumeric(c) {
                    rLexer_consume_char(lexer);
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
fn rust_identifier_to_token(ident: String) -> RToken {
    if string_eq(&ident, &string("fn")) {
        RToken::Fn
    } else if string_eq(&ident, &string("enum")) {
        RToken::Enum
    } else if string_eq(&ident, &string("extern")) {
        RToken::Extern
    } else if string_eq(&ident, &string("let")) {
        RToken::Let
    } else if string_eq(&ident, &string("if")) {
        RToken::If
    } else if string_eq(&ident, &string("else")) {
        RToken::Else
    } else if string_eq(&ident, &string("while")) {
        RToken::While
    } else if string_eq(&ident, &string("return")) {
        RToken::Return
    } else if string_eq(&ident, &string("match")) {
        RToken::Match
    } else if string_eq(&ident, &string("as")) {
        RToken::As
    } else if string_eq(&ident, &string("unsafe")) {
        RToken::Unsafe
    } else if string_eq(&ident, &string("mut")) {
        RToken::Mut
    } else if string_eq(&ident, &string("usize")) {
        RToken::Usize
    } else if string_eq(&ident, &string("u8")) {
        RToken::U8
    } else if string_eq(&ident, &string("bool")) {
        RToken::Bool
    } else if string_eq(&ident, &string("char")) {
        RToken::Char
    } else if string_eq(&ident, &string("str")) {
        RToken::Str
    } else if string_eq(&ident, &string("true")) {
        RToken::Literal(RLiteral::Bool(true))
    } else if string_eq(&ident, &string("false")) {
        RToken::Literal(RLiteral::Bool(false))
    } else {
        RToken::Identifier(ident)
    }
}

fn rLexer_scan_integer(lexer: &mut RLexer) -> usize {
    let mut value: String = string_new();

    let mut done: bool = false;
    while not(done) {
        match rLexer_peek_char(lexer) {
            Option::Some(c) => {
                if is_digit(c) {
                    string_push(&mut value, c);
                    rLexer_consume_char(lexer);
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

fn rLexer_scan_char_literal(lexer: &mut RLexer) -> char {
    rLexer_expect_char(lexer, '\'');
    let c: char = match rLexer_consume_char(lexer) {
        Option::Some('\\') => rLexer_scan_escape_char(lexer),
        Option::Some(ch) => ch,
        Option::None => lexer_error(lexer, &string("unexpected end of file")),
    };
    rLexer_expect_char(lexer, '\'');
    c
}

fn rLexer_scan_string_literal(lexer: &mut RLexer) -> String {
    rLexer_expect_char(lexer, '"');
    let mut s: String = string_new();
    while true {
        match rLexer_consume_char(lexer) {
            Option::Some('"') => return s,
            Option::Some('\\') => string_push(&mut s, rLexer_scan_escape_char(lexer)),
            Option::Some(c) => string_push(&mut s, c),
            Option::None => lexer_error(lexer, &string("unexpected end of string literal")),
        }
    }
    s // satisfy compiler
}

/// Scan an escape sequence after backslash.
fn rLexer_scan_escape_char(lexer: &mut RLexer) -> char {
    match rLexer_consume_char(lexer) {
        Option::Some('n') => '\n',
        Option::Some('t') => '\t',
        Option::Some('r') => '\r',
        Option::Some('0') => '\0',
        Option::Some(c) => c,
        Option::None => lexer_error(lexer, &string("unexpected end of escape sequence")),
    }
}

fn rLexer_scan_symbol(lexer: &mut RLexer) -> RToken {
    match unwrap::<char>(rLexer_consume_char(lexer)) {
        '{' => RToken::LBrace,
        '}' => RToken::RBrace,
        '(' => RToken::LParen,
        ')' => RToken::RParen,
        ';' => RToken::SemiColon,
        ',' => RToken::Comma,
        '|' => RToken::Pipe,
        '+' => RToken::Plus,
        '*' => RToken::Star,
        '/' => rLexer_scan_slash(lexer),
        '%' => RToken::Remainder,
        '&' => RToken::Ampersand,
        ':' => rLexer_scan_colon(lexer),
        '=' => rLexer_scan_equals(lexer),
        '-' => rLexer_scan_minus(lexer),
        '!' => rLexer_scan_bang(lexer),
        '<' => rLexer_scan_less(lexer),
        '>' => rLexer_scan_greater(lexer),
        c => {
            let mut message: String = string("unexpected character: ");
            string_push_string(&mut message, &rLiteral_to_string(&RLiteral::Char(c)));
            lexer_error(lexer, &message);
        },
    }
}

fn rLexer_scan_slash(lexer: &mut RLexer) -> RToken {
    match rLexer_peek_char(lexer) {
        Option::Some('/') => {
            rLexer_consume_char(lexer);
            rLexer_skip_line_comment(lexer);
            rLexer_next_token(lexer)
        },
        _ => RToken::Slash,
    }
}

fn rLexer_scan_colon(lexer: &mut RLexer) -> RToken {
    match rLexer_peek_char(lexer) {
        Option::Some(':') => {
            rLexer_consume_char(lexer);
            RToken::DoubleColon
        },
        _ => RToken::Colon,
    }
}

fn rLexer_scan_equals(lexer: &mut RLexer) -> RToken {
    match rLexer_peek_char(lexer) {
        Option::Some('=') => {
            rLexer_consume_char(lexer);
            RToken::Cmp(RComparisonOp::Eq)
        },
        Option::Some('>') => {
            rLexer_consume_char(lexer);
            RToken::FatArrow
        },
        _ => RToken::Assign,
    }
}

fn rLexer_scan_minus(lexer: &mut RLexer) -> RToken {
    match rLexer_peek_char(lexer) {
        Option::Some('>') => {
            rLexer_consume_char(lexer);
            RToken::Arrow
        },
        _ => RToken::Minus,
    }
}

fn rLexer_scan_bang(lexer: &mut RLexer) -> RToken {
    match rLexer_peek_char(lexer) {
        Option::Some('=') => {
            rLexer_consume_char(lexer);
            RToken::Cmp(RComparisonOp::Ne)
        },
        _ => RToken::Bang,
    }
}

fn rLexer_scan_less(lexer: &mut RLexer) -> RToken {
    match rLexer_peek_char(lexer) {
        Option::Some('=') => {
            rLexer_consume_char(lexer);
            RToken::Cmp(RComparisonOp::Leq)
        },
        _ => RToken::Cmp(RComparisonOp::Lt),
    }
}

fn rLexer_scan_greater(lexer: &mut RLexer) -> RToken {
    match rLexer_peek_char(lexer) {
        Option::Some('=') => {
            rLexer_consume_char(lexer);
            RToken::Cmp(RComparisonOp::Geq)
        },
        _ => RToken::Cmp(RComparisonOp::Gt),
    }
}

fn rLexer_skip_whitespace(lexer: &mut RLexer) {
    while true {
        match rLexer_peek_char(lexer) {
            Option::Some(c) => {
                if is_whitespace(c) {
                    rLexer_consume_char(lexer);
                } else {
                    return;
                }
            },
            Option::None => return,
        }
    }
}

fn rLexer_skip_line_comment(lexer: &mut RLexer) {
    while true {
        match rLexer_consume_char(lexer) {
            Option::Some('\n') => return,
            Option::Some(_) => (),
            Option::None => return,
        }
    }
}

/// Skips attributes which are useful in Rust, but unsupported.
fn rLexer_skip_attributes(lexer: &mut RLexer) {
    rLexer_skip_whitespace(lexer);
    while true {
        match rLexer_peek_char(lexer) {
            Option::Some('#') => {
                rLexer_consume_char(lexer);
                rLexer_skip_whitespace(lexer);

                match rLexer_consume_char(lexer) {
                    Option::Some('[') => {
                        let mut skipping: bool = true;
                        while skipping {
                            match rLexer_consume_char(lexer) {
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
    Function(bool, String, Vec<RAstVariable>, RType, RAstBlock),
}

/// Enum definition.
enum RAstEnum {
    /// name, variants
    Enum(String, Vec<RAstVariant>),
}

/// Extern function declaration.
enum RAstExternFunction {
    /// name, parameters, return type
    ExternFunction(String, Vec<RAstVariable>, RType),
}

/// Enum variant.
enum RAstVariant {
    /// name, field types (empty vec for unit-like variants)
    Variant(String, Vec<RType>),
}

/// Typed variable (`pattern: type`).
enum RAstVariable {
    Variable(RAstPattern, RType),
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
enum RType {
    U8,
    Usize,
    Bool,
    Char,
    Unit,
    Never,
    Custom(String),
    /// inner, mutable
    Reference(Box<RType>, bool),
    /// `*mut T`
    RawPointerMut(Box<RType>),
}

/// A Rust expression.
enum RAstExpr {
    Return(Option<Box<RAstExpr>>),
    Assign(Box<RAstExpr>, Box<RAstExpr>),
    Binary(RAstBinaryOp, Box<RAstExpr>, Box<RAstExpr>),
    Cast(Box<RAstExpr>, RType),
    Unary(RAstUnaryOp, Box<RAstExpr>),
    Literal(RLiteral),
    VariableUse(String),
    Path(Vec<String>, Vec<RAstExpr>), // either function call or enum instantiaton
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
fn rAstPath_to_string(segments: &Vec<String>) -> String {
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
fn rAstLiteral_type(literal: &RLiteral) -> RType {
    match literal {
        RLiteral::Int(_) => RType::Usize,
        RLiteral::Char(_) => RType::Char,
        RLiteral::Bool(_) => RType::Bool,
        RLiteral::String(_) => {
            RType::Reference(box_new::<RType>(RType::Custom(string("str"))), false)
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

/// Return the size of the given type in bytes.
fn rType_size(ty: &RType) -> usize {
    match ty {
        RType::U8 | RType::Char | RType::Bool => 1,
        RType::Usize | RType::Reference(_, _) | RType::RawPointerMut(_) => size_of::<usize>(),
        RType::Unit | RType::Never => 0,
        RType::Custom(_) => 0, // TODO:
    }
}

/// Convert Rust AST type into a simple LLVM-IR type name.
fn rType_to_llvm_name(ty: &RType) -> String {
    match ty {
        RType::U8 => string("i8"),
        RType::Usize => string("i64"), // assume 64-bit for now
        RType::Bool => string("i1"),
        RType::Char => string("i8"),
        RType::Unit => string("void"),
        RType::Never => string("void"),
        RType::Custom(_) => string("i64"),
        RType::Reference(_, _) => string("ptr"),
        RType::RawPointerMut(_) => string("ptr"),
    }
}

fn rType_is_numeric(ty: &RType) -> bool {
    match ty {
        RType::U8 => true,
        RType::Usize => true,
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
fn rType_coerce(left: RType, right: RType) -> RType {
    if rType_eq(&left, &RType::Never) {
        right
    } else {
        left
    }
}

/// Checks for equality between two types.
/// If one of the arguments is Never, return true, since Never matches every type.
fn type_matches(left: &RType, right: &RType) -> bool {
    or(
        // Never is a special type that indicates the value is unreachable, so it matches every
        // type
        or(
            rType_eq(left, &RType::Never),
            rType_eq(right, &RType::Never),
        ),
        rType_eq(left, right),
    )
}

/// Return true if the type has a value.
/// This is true for all types, other than Unit and Never.
fn type_has_value(ty: &RType) -> bool {
    not(type_matches(ty, &RType::Unit))
}

/// Require and consume the given token.
fn expect_token(lexer: &mut RLexer, token: &RToken) {
    if not(rLexer_try_consume(lexer, token)) {
        let bad_token: &RToken = rLexer_current_token(lexer);
        let mut message: String = string("expected ");
        string_push_string(&mut message, &token_to_string(token));
        string_push_str(&mut message, ", but got: ");
        string_push_string(&mut message, &token_to_string(bad_token));
        parse_error(lexer, &message);
    }
}

/// Read and consume the current identifier token.
fn expect_identifier(lexer: &mut RLexer) -> String {
    match rLexer_current_token(lexer) {
        RToken::Identifier(name) => {
            let name: String = string_clone(name);
            rLexer_next_token(lexer);
            name
        },
        token => {
            let mut message: String = string("expected identifier, but got: ");
            string_push_string(&mut message, &token_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

fn parse_language(lexer: &mut RLexer) -> RAst {
    let mut items: Vec<RAstItem> = vec_new::<RAstItem>();

    while not(rLexer_current_token_eq(lexer, &RToken::Eof)) {
        match rLexer_current_token(lexer) {
            RToken::Unsafe => match rLexer_next_token(lexer) {
                RToken::Extern => {
                    let extern_block: RAstItem = RAstItem::ExternBlock(parse_extern_block(lexer));
                    vec_push::<RAstItem>(&mut items, extern_block);
                },
                RToken::Fn => {
                    let function: RAstItem = RAstItem::Function(parse_function(lexer, true));
                    vec_push::<RAstItem>(&mut items, function);
                },
                token => {
                    let mut message: String = string("expected fn or extern, but got: ");
                    string_push_string(&mut message, &token_to_string(&token));
                    parse_error(lexer, &message);
                },
            },
            RToken::Fn => {
                let function: RAstItem = RAstItem::Function(parse_function(lexer, false));
                vec_push::<RAstItem>(&mut items, function);
            },
            RToken::Enum => {
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

fn parse_extern_block(lexer: &mut RLexer) -> Vec<RAstExternFunction> {
    expect_token(lexer, &RToken::Extern);

    match rLexer_current_token(lexer) {
        RToken::Literal(RLiteral::String(value)) => {
            if not(string_eq(value, &string("C"))) {
                let mut message: String = string("expected \"C\", but got: ");
                string_push_string(&mut message, &token_to_string(rLexer_current_token(lexer)));
                parse_error(lexer, &message);
            }
            rLexer_next_token(lexer);
        },
        _ => {
            let mut message: String = string("expected \"C\", but got: ");
            string_push_string(&mut message, &token_to_string(rLexer_current_token(lexer)));
            parse_error(lexer, &message);
        },
    }

    expect_token(lexer, &RToken::LBrace);

    let mut functions: Vec<RAstExternFunction> = vec_new::<RAstExternFunction>();
    while not(rLexer_current_token_eq(lexer, &RToken::RBrace)) {
        let function: RAstExternFunction = parse_function_declaration(lexer);
        vec_push::<RAstExternFunction>(&mut functions, function);
    }
    expect_token(lexer, &RToken::RBrace);

    functions
}

fn parse_function_declaration(lexer: &mut RLexer) -> RAstExternFunction {
    expect_token(lexer, &RToken::Fn);
    let name: String = expect_identifier(lexer);
    expect_token(lexer, &RToken::LParen);

    let mut parameters: Vec<RAstVariable> = vec_new::<RAstVariable>();
    if not(rLexer_current_token_eq(lexer, &RToken::RParen)) {
        let variable: RAstVariable = parse_variable(lexer);
        vec_push::<RAstVariable>(&mut parameters, variable);

        while and(
            rLexer_try_consume(lexer, &RToken::Comma),
            not(rLexer_current_token_eq(lexer, &RToken::RParen)),
        ) {
            let variable: RAstVariable = parse_variable(lexer);
            vec_push::<RAstVariable>(&mut parameters, variable);
        }
    }
    expect_token(lexer, &RToken::RParen);

    let return_type: RType = if rLexer_try_consume(lexer, &RToken::Arrow) {
        parse_type(lexer)
    } else {
        RType::Unit
    };

    expect_token(lexer, &RToken::SemiColon);
    RAstExternFunction::ExternFunction(name, parameters, return_type)
}

fn parse_function(lexer: &mut RLexer, is_unsafe: bool) -> RAstFunction {
    expect_token(lexer, &RToken::Fn);

    let name: String = expect_identifier(lexer);
    expect_token(lexer, &RToken::LParen);

    let mut parameters: Vec<RAstVariable> = vec_new::<RAstVariable>();
    if not(rLexer_current_token_eq(lexer, &RToken::RParen)) {
        let variable: RAstVariable = parse_variable(lexer);
        vec_push::<RAstVariable>(&mut parameters, variable);

        while and(
            rLexer_try_consume(lexer, &RToken::Comma),
            not(rLexer_current_token_eq(lexer, &RToken::RParen)),
        ) {
            let variable: RAstVariable = parse_variable(lexer);
            vec_push::<RAstVariable>(&mut parameters, variable);
        }
    }
    expect_token(lexer, &RToken::RParen);

    let return_type: RType = if rLexer_try_consume(lexer, &RToken::Arrow) {
        parse_type(lexer)
    } else {
        RType::Unit
    };

    let body: RAstBlock = parse_block(lexer);

    RAstFunction::Function(is_unsafe, name, parameters, return_type, body)
}

fn parse_enum(lexer: &mut RLexer) -> RAstEnum {
    expect_token(lexer, &RToken::Enum);
    let name: String = expect_identifier(lexer);
    expect_token(lexer, &RToken::LBrace);

    let mut variants: Vec<RAstVariant> = vec_new::<RAstVariant>();
    let first_variant: RAstVariant = parse_variant(lexer);
    vec_push::<RAstVariant>(&mut variants, first_variant);
    expect_token(lexer, &RToken::Comma);

    while not(rLexer_current_token_eq(lexer, &RToken::RBrace)) {
        let variant: RAstVariant = parse_variant(lexer);
        vec_push::<RAstVariant>(&mut variants, variant);
        expect_token(lexer, &RToken::Comma);
    }
    expect_token(lexer, &RToken::RBrace);

    RAstEnum::Enum(name, variants)
}

fn parse_variant(lexer: &mut RLexer) -> RAstVariant {
    let name: String = expect_identifier(lexer);

    let mut field_types: Vec<RType> = vec_new::<RType>();
    if rLexer_try_consume(lexer, &RToken::LParen) {
        vec_push::<RType>(&mut field_types, parse_type(lexer));

        while rLexer_try_consume(lexer, &RToken::Comma) {
            vec_push::<RType>(&mut field_types, parse_type(lexer));
        }
        expect_token(lexer, &RToken::RParen);
    }

    RAstVariant::Variant(name, field_types)
}

fn parse_block(lexer: &mut RLexer) -> RAstBlock {
    expect_token(lexer, &RToken::LBrace);
    let mut statements: Vec<RAstStatement> = vec_new::<RAstStatement>();
    let mut tail: Option<Box<RAstExpr>> = Option::None;

    while not(rLexer_current_token_eq(lexer, &RToken::RBrace)) {
        if rLexer_current_token_eq(lexer, &RToken::Let) {
            let let_binding: RAstStatement = parse_binding(lexer);
            vec_push::<RAstStatement>(&mut statements, let_binding);
            expect_token(lexer, &RToken::SemiColon);
        } else {
            let expression: RAstExpr = parse_expression(lexer);

            if rLexer_current_token_eq(lexer, &RToken::RBrace) {
                // end of block with expression as return value
                rLexer_next_token(lexer);
                tail = Option::Some(box_new::<RAstExpr>(expression));
                return RAstBlock::Block(statements, tail);
            } else {
                rLexer_try_consume(lexer, &RToken::SemiColon); // optional semi-colon
                let expr_statement = RAstStatement::Expression(box_new::<RAstExpr>(expression));
                vec_push::<RAstStatement>(&mut statements, expr_statement);
            }
        }
    }
    expect_token(lexer, &RToken::RBrace);

    RAstBlock::Block(statements, tail)
}

fn parse_binding(lexer: &mut RLexer) -> RAstStatement {
    expect_token(lexer, &RToken::Let);
    let variable: RAstVariable = parse_variable(lexer);
    expect_token(lexer, &RToken::Assign);
    let value: RAstExpr = parse_expression(lexer);
    RAstStatement::Let(variable, box_new::<RAstExpr>(value))
}

fn parse_variable(lexer: &mut RLexer) -> RAstVariable {
    let pattern: RAstPattern = parse_pattern(lexer);
    expect_token(lexer, &RToken::Colon);
    let ty: RType = parse_type(lexer);
    RAstVariable::Variable(pattern, ty)
}

fn parse_type(lexer: &mut RLexer) -> RType {
    match rLexer_current_token(lexer) {
        RToken::U8 => {
            rLexer_next_token(lexer);
            RType::U8
        },
        RToken::Usize => {
            rLexer_next_token(lexer);
            RType::Usize
        },
        RToken::Char => {
            rLexer_next_token(lexer);
            RType::Char
        },
        RToken::Bool => {
            rLexer_next_token(lexer);
            RType::Bool
        },
        RToken::LParen => {
            rLexer_next_token(lexer);
            expect_token(lexer, &RToken::RParen);
            RType::Unit
        },
        RToken::Bang => {
            rLexer_next_token(lexer);
            RType::Never
        },
        RToken::Ampersand => {
            rLexer_next_token(lexer);

            if rLexer_try_consume(lexer, &RToken::Str) {
                return RType::Custom(string("&str"));
            }

            let mutable: bool = rLexer_try_consume(lexer, &RToken::Mut);
            let inner: RType = parse_type(lexer);
            RType::Reference(box_new::<RType>(inner), mutable)
        },
        RToken::Star => {
            rLexer_next_token(lexer);
            expect_token(lexer, &RToken::Mut);
            let inner: RType = parse_type(lexer);
            RType::RawPointerMut(box_new::<RType>(inner))
        },
        RToken::Identifier(_) => {
            let enum_name: String = expect_identifier(lexer);
            RType::Custom(enum_name)
        },
        token => {
            let mut message: String = string("expected a type, but got: ");
            string_push_string(&mut message, &token_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

fn parse_expression(lexer: &mut RLexer) -> RAstExpr {
    match rLexer_current_token(lexer) {
        RToken::Return => {
            rLexer_next_token(lexer);
            match rLexer_current_token(lexer) {
                RToken::SemiColon | RToken::RBrace => RAstExpr::Return(Option::None),
                _ => {
                    let expression: RAstExpr = parse_expression(lexer);
                    RAstExpr::Return(Option::Some(box_new::<RAstExpr>(expression)))
                },
            }
        },
        _ => parse_assignment(lexer),
    }
}

fn parse_assignment(lexer: &mut RLexer) -> RAstExpr {
    let left: RAstExpr = parse_comparison(lexer);
    if rLexer_try_consume(lexer, &RToken::Assign) {
        let right: RAstExpr = parse_assignment(lexer);
        RAstExpr::Assign(box_new::<RAstExpr>(left), box_new::<RAstExpr>(right))
    } else {
        left
    }
}

fn parse_comparison(lexer: &mut RLexer) -> RAstExpr {
    let left: RAstExpr = parse_arithmetic(lexer);

    match rLexer_current_token(lexer) {
        RToken::Cmp(comparison) => {
            let comparison: RComparisonOp = comparison_clone(comparison);
            rLexer_next_token(lexer);

            let right: RAstExpr = parse_arithmetic(lexer);

            let operator: RAstComparisonOp = match comparison {
                RComparisonOp::Eq => RAstComparisonOp::Eq,
                RComparisonOp::Ne => RAstComparisonOp::Ne,
                RComparisonOp::Gt => RAstComparisonOp::Gt,
                RComparisonOp::Lt => RAstComparisonOp::Lt,
                RComparisonOp::Geq => RAstComparisonOp::Ge,
                RComparisonOp::Leq => RAstComparisonOp::Le,
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

fn parse_arithmetic(lexer: &mut RLexer) -> RAstExpr {
    let mut left: RAstExpr = parse_term(lexer);

    while or(
        rLexer_current_token_eq(lexer, &RToken::Plus),
        rLexer_current_token_eq(lexer, &RToken::Minus),
    ) {
        let operator: RAstArithmeticOp = match rLexer_current_token(lexer) {
            RToken::Plus => RAstArithmeticOp::Add,
            RToken::Minus => RAstArithmeticOp::Sub,
            _ => panic("unreachable"),
        };
        rLexer_next_token(lexer);

        let right: RAstExpr = parse_term(lexer);

        left = RAstExpr::Binary(
            RAstBinaryOp::Arithmetic(operator),
            box_new::<RAstExpr>(left),
            box_new::<RAstExpr>(right),
        );
    }
    left
}

fn parse_term(lexer: &mut RLexer) -> RAstExpr {
    let mut left: RAstExpr = parse_cast(lexer);

    while or(
        rLexer_current_token_eq(lexer, &RToken::Star),
        or(
            rLexer_current_token_eq(lexer, &RToken::Slash),
            rLexer_current_token_eq(lexer, &RToken::Remainder),
        ),
    ) {
        let operator: RAstArithmeticOp = match rLexer_current_token(lexer) {
            RToken::Star => RAstArithmeticOp::Mul,
            RToken::Slash => RAstArithmeticOp::Div,
            RToken::Remainder => RAstArithmeticOp::Rem,
            _ => panic("unreachable"),
        };
        rLexer_next_token(lexer);

        let right: RAstExpr = parse_cast(lexer);

        left = RAstExpr::Binary(
            RAstBinaryOp::Arithmetic(operator),
            box_new::<RAstExpr>(left),
            box_new::<RAstExpr>(right),
        );
    }
    left
}

fn parse_cast(lexer: &mut RLexer) -> RAstExpr {
    let mut expression: RAstExpr = parse_unary(lexer);

    while rLexer_try_consume(lexer, &RToken::As) {
        let cast_type: RType = parse_type(lexer);
        expression = RAstExpr::Cast(box_new::<RAstExpr>(expression), cast_type);
    }
    expression
}

fn parse_unary(lexer: &mut RLexer) -> RAstExpr {
    match rLexer_current_token(lexer) {
        RToken::Ampersand => {
            rLexer_next_token(lexer);
            let mutable: bool = rLexer_try_consume(lexer, &RToken::Mut);
            let inner: RAstExpr = parse_unary(lexer);
            RAstExpr::Unary(RAstUnaryOp::Reference(mutable), box_new::<RAstExpr>(inner))
        },
        RToken::Star => {
            rLexer_next_token(lexer);
            let inner: RAstExpr = parse_unary(lexer);
            RAstExpr::Unary(RAstUnaryOp::Dereference, box_new::<RAstExpr>(inner))
        },
        _ => parse_factor(lexer),
    }
}

fn parse_factor(lexer: &mut RLexer) -> RAstExpr {
    match rLexer_current_token(lexer) {
        RToken::Literal(literal) => {
            let literal: RLiteral = rLiteral_clone(literal);
            rLexer_next_token(lexer);
            RAstExpr::Literal(literal)
        },
        RToken::Identifier(_) => parse_identifier_expression(lexer),
        RToken::LParen => {
            rLexer_next_token(lexer);
            let expression: RAstExpr = parse_expression(lexer);
            expect_token(lexer, &RToken::RParen);
            expression
        },
        RToken::Unsafe => {
            rLexer_next_token(lexer);
            RAstExpr::Block(true, parse_block(lexer))
        },
        RToken::LBrace => RAstExpr::Block(false, parse_block(lexer)),
        RToken::If => RAstExpr::If(parse_if(lexer)),
        RToken::While => parse_while(lexer),
        RToken::Match => parse_match(lexer),
        token => {
            let mut message: String = string("unexpected token: ");
            string_push_string(&mut message, &token_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

/// Parses either a variable, a function call or an instantiation of an enum.
fn parse_identifier_expression(lexer: &mut RLexer) -> RAstExpr {
    let first_identifier: String = expect_identifier(lexer);

    if rLexer_try_consume(lexer, &RToken::DoubleColon) {
        let second_identifier: String = expect_identifier(lexer);

        let mut segments: Vec<String> = vec_new::<String>();
        vec_push::<String>(&mut segments, first_identifier);
        vec_push::<String>(&mut segments, second_identifier);

        parse_path_values(lexer, segments)
    } else if rLexer_current_token_eq(lexer, &RToken::LParen) {
        let mut segments: Vec<String> = vec_new::<String>();
        vec_push::<String>(&mut segments, first_identifier);
        parse_path_values(lexer, segments)
    } else {
        RAstExpr::VariableUse(first_identifier)
    }
}

fn parse_if(lexer: &mut RLexer) -> RAstIf {
    expect_token(lexer, &RToken::If);
    let condition: RAstExpr = parse_expression(lexer);
    let then_block: RAstBlock = parse_block(lexer);

    let else_branch: Option<RAstElse> = if rLexer_try_consume(lexer, &RToken::Else) {
        if rLexer_current_token_eq(lexer, &RToken::If) {
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

fn parse_while(lexer: &mut RLexer) -> RAstExpr {
    expect_token(lexer, &RToken::While);
    let condition: RAstExpr = parse_expression(lexer);
    let body: RAstBlock = parse_block(lexer);
    RAstExpr::While(box_new::<RAstExpr>(condition), body)
}

fn parse_match(lexer: &mut RLexer) -> RAstExpr {
    expect_token(lexer, &RToken::Match);
    let value: RAstExpr = parse_expression(lexer);
    expect_token(lexer, &RToken::LBrace);

    let mut arms: Vec<RAstArm> = vec_new::<RAstArm>();
    while not(rLexer_current_token_eq(lexer, &RToken::RBrace)) {
        let arm: RAstArm = parse_arm(lexer);
        vec_push::<RAstArm>(&mut arms, arm);
    }
    expect_token(lexer, &RToken::RBrace);

    RAstExpr::Match(box_new::<RAstExpr>(value), arms)
}

fn parse_arm(lexer: &mut RLexer) -> RAstArm {
    let mut patterns: Vec<RAstPattern> = vec_new::<RAstPattern>();
    let pattern: RAstPattern = parse_pattern(lexer);
    vec_push::<RAstPattern>(&mut patterns, pattern);

    while rLexer_try_consume(lexer, &RToken::Pipe) {
        let pattern: RAstPattern = parse_pattern(lexer);
        vec_push::<RAstPattern>(&mut patterns, pattern);
    }

    expect_token(lexer, &RToken::FatArrow);

    let expression: RAstExpr = parse_expression(lexer);
    expect_token(lexer, &RToken::Comma);
    RAstArm::Arm(patterns, expression)
}

fn parse_pattern(lexer: &mut RLexer) -> RAstPattern {
    match rLexer_current_token(lexer) {
        RToken::Literal(literal) => {
            let pattern: RAstPattern = RAstPattern::Literal(match literal {
                RLiteral::Int(value) => RAstPatternLiteral::Int(*value),
                RLiteral::Char(value) => RAstPatternLiteral::Char(*value),
                RLiteral::Bool(value) => RAstPatternLiteral::Bool(*value),
                RLiteral::String(_) => {
                    parse_error(lexer, &string("matching on string literals is unsupported"))
                },
            });
            rLexer_next_token(lexer);
            pattern
        },
        RToken::Mut => {
            rLexer_next_token(lexer);
            let identifier: String = expect_identifier(lexer);
            RAstPattern::Identifier(true, identifier)
        },
        RToken::Identifier(_) => {
            let identifier: String = expect_identifier(lexer);

            if string_eq(&identifier, &string("_")) {
                RAstPattern::Wildcard
            } else if rLexer_try_consume(lexer, &RToken::DoubleColon) {
                let variant_name: String = expect_identifier(lexer);

                let mut fields: Vec<RAstPattern> = vec_new::<RAstPattern>();
                if rLexer_try_consume(lexer, &RToken::LParen) {
                    if not(rLexer_current_token_eq(lexer, &RToken::RParen)) {
                        let pattern: RAstPattern = parse_pattern(lexer);
                        vec_push::<RAstPattern>(&mut fields, pattern);

                        while and(
                            rLexer_try_consume(lexer, &RToken::Comma),
                            not(rLexer_current_token_eq(lexer, &RToken::RParen)),
                        ) {
                            let pattern: RAstPattern = parse_pattern(lexer);
                            vec_push::<RAstPattern>(&mut fields, pattern);
                        }
                    }
                    expect_token(lexer, &RToken::RParen);
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

fn parse_path_values(lexer: &mut RLexer, path: Vec<String>) -> RAstExpr {
    expect_token(lexer, &RToken::LParen);

    let mut values: Vec<RAstExpr> = vec_new::<RAstExpr>();
    if not(rLexer_current_token_eq(lexer, &RToken::RParen)) {
        let first_value: RAstExpr = parse_expression(lexer);
        vec_push::<RAstExpr>(&mut values, first_value);

        while and(
            rLexer_try_consume(lexer, &RToken::Comma),
            not(rLexer_current_token_eq(lexer, &RToken::RParen)),
        ) {
            let value: RAstExpr = parse_expression(lexer);
            vec_push::<RAstExpr>(&mut values, value);
        }
    }
    expect_token(lexer, &RToken::RParen);

    RAstExpr::Path(path, values)
}

// TODO: This should be shorter. E.g. add a name to FnSignature, which can be used in the AST.
fn collect_items(ast: &RAst) -> StringMap<Item> {
    let RAst::Language(ast_items): &RAst = ast;
    let mut items: StringMap<Item> = stringMap_new::<Item>();

    let mut i: usize = 0;
    while i < vec_len::<RAstItem>(ast_items) {
        match vec_at::<RAstItem>(ast_items, i) {
            RAstItem::Function(RAstFunction::Function(is_unsafe, name, params, return_type, _)) => {
                let mut param_types: Vec<RType> = vec_new::<RType>();
                let mut param_index: usize = 0;
                while param_index < vec_len::<RAstVariable>(params) {
                    let RAstVariable::Variable(_, parameter_type): &RAstVariable =
                        vec_at::<RAstVariable>(params, param_index);
                    vec_push::<RType>(&mut param_types, rType_clone(parameter_type));
                    param_index = param_index + 1;
                }

                let signature: FnSignature =
                    FnSignature::Fn(param_types, rType_clone(return_type), *is_unsafe);
                stringMap_insert::<Item>(&mut items, string_clone(name), Item::Function(signature));
            },
            RAstItem::Enum(enum_item) => {
                let RAstEnum::Enum(name, variants): &RAstEnum = enum_item;

                let mut cloned_variants: Vec<RAstVariant> = vec_new::<RAstVariant>();
                let mut i: usize = 0;
                while i < vec_len::<RAstVariant>(variants) {
                    let variant: &RAstVariant = vec_at::<RAstVariant>(variants, i);
                    let RAstVariant::Variant(variant_name, fields): &RAstVariant = variant;

                    let mut cloned_fields: Vec<RType> = vec_new::<RType>();
                    let mut field_index: usize = 0;
                    while field_index < vec_len::<RType>(fields) {
                        let field_type: &RType = vec_at::<RType>(fields, field_index);
                        vec_push::<RType>(&mut cloned_fields, rType_clone(field_type));
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

                    let mut param_types: Vec<RType> = vec_new::<RType>();
                    let mut param_index: usize = 0;
                    while param_index < vec_len::<RAstVariable>(params) {
                        let RAstVariable::Variable(_, parameter_type): &RAstVariable =
                            vec_at::<RAstVariable>(params, param_index);
                        vec_push::<RType>(&mut param_types, rType_clone(parameter_type));
                        param_index = param_index + 1;
                    }

                    let signature: FnSignature =
                        FnSignature::Fn(param_types, rType_clone(return_type), true);
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
    Semantic(StringMap<Item>, StringMapStack<Variable>, RType, usize),
}

fn semantic_new(items: StringMap<Item>) -> Semantic {
    Semantic::Semantic(items, stringMapStack_new::<Variable>(), RType::Unit, 0)
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

fn semantic_current_fn_return_type(semantic: &Semantic) -> &RType {
    let Semantic::Semantic(_, _, return_type, _): &Semantic = semantic;
    return_type
}

fn semantic_set_current_fn_return_type(semantic: &mut Semantic, ty: RType) {
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
fn semantic_expect_type_match(left: &RType, right: &RType) {
    if not(rType_eq(left, right)) {
        semantic_check_error("types do not match perfectly");
    }
}

/// Return true if the given types match.
///
/// Two types a, b match if:
/// 1. a == b
/// 2. a == Never
/// 3. b == Never
fn semantic_expect_coerced_type_match(left: &RType, right: &RType) {
    if not(type_matches(left, right)) {
        semantic_check_error("type mismatch");
    }
}

fn semantic_expect_numeric_type(ty: &RType) {
    if not(rType_is_numeric(ty)) {
        semantic_check_error("expected numeric type");
    }
}

fn semantic_expect_bool_type(ty: &RType) {
    if not(rType_eq(ty, &RType::Bool)) {
        semantic_check_error("expected bool type");
    }
}

/// Lookup a variable in local scopes.
fn semantic_lookup_variable(semantic: &Semantic, name: &String) -> Option<Variable> {
    match stringMapStack_lookup::<Variable>(semantic_locals(semantic), name) {
        Option::Some(entry) => {
            let Variable::Variable(variable_type, mutable) = entry;
            Option::Some(Variable::Variable(rType_clone(variable_type), *mutable))
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
    variable_type: RType,
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
    Variable(RType, bool),
}

/// A global item, i.e. either a function or an enum.
enum Item {
    Function(FnSignature),
    Enum(RAstEnum),
}

/// A type that represents the (type) signature of a function.
enum FnSignature {
    /// parameter types, return type, is unsafe
    Fn(Vec<RType>, RType, bool),
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
fn castOperation_get_cast_operation(left_type: &RType, right_type: &RType) -> CastOperation {
    if rType_eq(left_type, right_type) {
        return CastOperation::None;
    }

    match left_type {
        RType::U8 => match right_type {
            RType::Usize => CastOperation::ZeroExtend,
            RType::Char => CastOperation::None,
            _ => CastOperation::Invalid,
        },
        RType::Usize => match right_type {
            RType::U8 => CastOperation::Truncate,
            RType::RawPointerMut(_) => CastOperation::IntToPtr,
            _ => CastOperation::Invalid,
        },
        RType::Bool => match right_type {
            RType::U8 | RType::Usize => CastOperation::ZeroExtend,
            _ => CastOperation::Invalid,
        },
        RType::Char => match right_type {
            RType::Usize => CastOperation::ZeroExtend,
            RType::U8 => CastOperation::None,
            _ => CastOperation::Invalid,
        },
        RType::Reference(left_inner, mutable) => match right_type {
            RType::RawPointerMut(right_inner) => {
                if and(
                    rType_eq(
                        box_deref::<RType>(left_inner),
                        box_deref::<RType>(right_inner),
                    ),
                    *mutable,
                ) {
                    CastOperation::None
                } else {
                    CastOperation::Invalid
                }
            },
            _ => CastOperation::Invalid,
        },
        RType::RawPointerMut(_) => match right_type {
            RType::RawPointerMut(_) => CastOperation::None,
            RType::Usize => CastOperation::PtrToInt,
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

    semantic_set_current_fn_return_type(semantic, rType_clone(return_type));
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
                    rType_clone(parameter_type),
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

    let block_type: RType = semantic_check_block(semantic, body, *is_unsafe);
    semantic_expect_coerced_type_match(&block_type, return_type);

    semantic_leave_scope(semantic);
    semantic_set_current_fn_return_type(semantic, RType::Unit);
}

/// Analyze one block and return its resulting type.
fn semantic_check_block(semantic: &mut Semantic, block: &RAstBlock, is_unsafe: bool) -> RType {
    let RAstBlock::Block(statements, tail): &RAstBlock = block;
    if is_unsafe {
        semantic_push_unsafe_context(semantic);
    }
    semantic_enter_scope(semantic);

    let mut statement_flow_type: RType = RType::Unit;
    let mut i: usize = 0;
    let len: usize = vec_len::<RAstStatement>(statements);
    while i < len {
        let statement: &RAstStatement = vec_at::<RAstStatement>(statements, i);
        match statement {
            RAstStatement::Let(variable, value) => {
                semantic_check_binding(semantic, variable, box_deref::<RAstExpr>(value));
            },
            RAstStatement::Expression(expression) => {
                let ty: RType =
                    semantic_check_expression(semantic, box_deref::<RAstExpr>(expression));
                if rType_eq(&ty, &RType::Never) {
                    statement_flow_type = RType::Never;
                }
            },
        }
        i = i + 1;
    }

    let mut block_type: RType = match tail {
        Option::Some(expression) => {
            semantic_check_expression(semantic, box_deref::<RAstExpr>(expression))
        },
        Option::None => RType::Unit,
    };

    if rType_eq(&statement_flow_type, &RType::Never) {
        block_type = RType::Never;
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
    let actual_type: RType = semantic_check_expression(semantic, value);
    semantic_expect_type_match(binding_type, &actual_type);

    match pattern {
        RAstPattern::Identifier(is_mutable, lvalue_name) => {
            // allow shadowing of variables
            let _ = semantic_insert_variable(
                semantic,
                string_clone(lvalue_name),
                rType_clone(binding_type),
                *is_mutable,
            );
        },
        _ => {},
    }
}

/// Analyze one expression and return its type.
fn semantic_check_expression(semantic: &mut Semantic, expression: &RAstExpr) -> RType {
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
        RAstExpr::Literal(literal) => rAstLiteral_type(literal),
        RAstExpr::VariableUse(name) => semantic_check_variable_use(semantic, name),
        RAstExpr::Path(path, values) => semantic_check_path(semantic, path, values),
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

fn semantic_check_return(semantic: &mut Semantic, returned: &Option<Box<RAstExpr>>) -> RType {
    match returned {
        Option::Some(expression) => {
            let ty: RType = semantic_check_expression(semantic, box_deref::<RAstExpr>(expression));
            semantic_expect_type_match(&ty, semantic_current_fn_return_type(semantic));
        },
        Option::None => {
            semantic_expect_type_match(&RType::Unit, semantic_current_fn_return_type(semantic));
        },
    }
    RType::Never
}

fn semantic_check_assignment(semantic: &mut Semantic, left: &RAstExpr, right: &RAstExpr) -> RType {
    let right_type: RType = semantic_check_expression(semantic, right);
    let left_type: RType = semantic_check_assignment_lvalue_type(semantic, left);
    semantic_expect_type_match(&left_type, &right_type);
    RType::Unit
}

fn semantic_check_assignment_lvalue_type(semantic: &mut Semantic, expression: &RAstExpr) -> RType {
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
            let pointer_type: RType =
                semantic_check_expression(semantic, box_deref::<RAstExpr>(value));
            match pointer_type {
                RType::Reference(inner, mutable) => {
                    if not(mutable) {
                        semantic_check_error("invalid assignment using immutable reference");
                    }
                    rType_clone(box_deref::<RType>(&inner))
                },
                RType::RawPointerMut(inner) => {
                    if not(semantic_is_unsafe_context(semantic)) {
                        semantic_check_error("raw pointer dereference requires unsafe");
                    }
                    rType_clone(box_deref::<RType>(&inner))
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
) -> RType {
    let left_type: RType = semantic_check_expression(semantic, left);
    let right_type: RType = semantic_check_expression(semantic, right);
    semantic_expect_type_match(&left_type, &right_type);

    match operator {
        RAstBinaryOp::Arithmetic(_) => {
            semantic_expect_numeric_type(&left_type);
            left_type
        },
        RAstBinaryOp::Comparison(_) => RType::Bool,
    }
}

fn semantic_check_cast(semantic: &mut Semantic, value: &RAstExpr, to_type: &RType) -> RType {
    let from_type: RType = semantic_check_expression(semantic, value);
    match castOperation_get_cast_operation(&from_type, to_type) {
        CastOperation::Invalid => semantic_check_error("invalid cast"),
        _ => rType_clone(to_type),
    }
}

fn semantic_check_unary_op(
    semantic: &mut Semantic,
    operator: &RAstUnaryOp,
    value: &RAstExpr,
) -> RType {
    match operator {
        RAstUnaryOp::Reference(mutable_ref) => match value {
            RAstExpr::VariableUse(name) => match semantic_lookup_variable(semantic, name) {
                Option::Some(Variable::Variable(ty, mutable_var)) => {
                    if and(*mutable_ref, not(mutable_var)) {
                        semantic_check_error("cannot take mutable reference to immutable variable");
                    }
                    RType::Reference(box_new::<RType>(ty), *mutable_ref)
                },
                _ => semantic_check_error("undefined variable"),
            },
            _ => {
                let ty: RType = semantic_check_expression(semantic, value);
                RType::Reference(box_new::<RType>(ty), *mutable_ref)
            },
        },
        RAstUnaryOp::Dereference => {
            let ty: RType = semantic_check_expression(semantic, value);
            match ty {
                RType::Reference(pointed, _) => rType_clone(box_deref::<RType>(&pointed)),
                RType::RawPointerMut(pointed) => {
                    if not(semantic_is_unsafe_context(semantic)) {
                        semantic_check_error("raw pointer dereference requires unsafe context");
                    }
                    rType_clone(box_deref::<RType>(&pointed))
                },
                _ => semantic_check_error("cannot dereference this expression"),
            }
        },
    }
}

fn semantic_check_variable_use(semantic: &mut Semantic, name: &String) -> RType {
    match semantic_lookup_variable(semantic, name) {
        Option::Some(Variable::Variable(ty, _)) => ty,
        _ => semantic_check_error("undefined variable"),
    }
}

fn semantic_check_path(
    semantic: &mut Semantic,
    path: &Vec<String>,
    values: &Vec<RAstExpr>,
) -> RType {
    // TODO: handle enum case
    let function_name: String = rAstPath_to_string(path);

    let FnSignature::Fn(parameter_types, return_type, is_unsafe): FnSignature =
        match semantic_lookup_function_signature(semantic, &function_name) {
            Option::Some(signature) => signature,
            _ => semantic_check_error("call to undefined function"),
        };

    if and(is_unsafe, not(semantic_is_unsafe_context(semantic))) {
        semantic_check_error("calling an unsafe function requires unsafe");
    }

    let mut i: usize = 0;
    while i < vec_len::<RAstExpr>(values) {
        let argument: &RAstExpr = vec_at::<RAstExpr>(values, i);
        let arg_type: RType = semantic_check_expression(semantic, argument);

        match vec_get::<RType>(&parameter_types, i) {
            Option::Some(ty) => {
                semantic_expect_type_match(ty, &arg_type);
            },
            _ => {
                semantic_check_error("function call has more arguments than there are parameters");
            },
        }

        i = i + 1;
    }

    return_type
}

fn semantic_check_if(semantic: &mut Semantic, if_expression: &RAstIf) -> RType {
    let RAstIf::If(condition, then_block, else_branch): &RAstIf = if_expression;
    let condition_type: RType =
        semantic_check_expression(semantic, box_deref::<RAstExpr>(condition));
    semantic_expect_bool_type(&condition_type);

    let then_type: RType = semantic_check_block(semantic, then_block, false);
    match else_branch {
        Option::Some(else_branch) => {
            let else_type: RType = match else_branch {
                RAstElse::If(nested_if) => {
                    semantic_check_if(semantic, box_deref::<RAstIf>(nested_if))
                },
                RAstElse::Block(block) => semantic_check_block(semantic, block, false),
            };
            semantic_expect_coerced_type_match(&then_type, &else_type);

            rType_coerce(then_type, else_type)
        },
        Option::None => RType::Unit,
    }
}

fn semantic_check_while(semantic: &mut Semantic, condition: &RAstExpr, body: &RAstBlock) -> RType {
    let condition_type: RType = semantic_check_expression(semantic, condition);
    semantic_expect_bool_type(&condition_type);
    let body_type: RType = semantic_check_block(semantic, body, false);
    semantic_expect_coerced_type_match(&RType::Unit, &body_type);
    RType::Unit
}

fn semantic_check_match(semantic: &mut Semantic, value: &RAstExpr, arms: &Vec<RAstArm>) -> RType {
    if vec_len::<RAstArm>(arms) == 0 {
        semantic_check_error("match requires at least one arm");
    }

    let expr_type: RType = semantic_check_expression(semantic, value);
    let mut return_type: RType = RType::Never;

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

        let arm_type: RType = semantic_check_expression(semantic, expression);
        semantic_expect_coerced_type_match(&return_type, &arm_type);

        return_type = rType_coerce(return_type, arm_type);
        i = i + 1;
    }
    return_type
}

fn semantic_check_pattern(pattern: &RAstPattern, expression_type: &RType) {
    let pattern_type: RType = match pattern {
        RAstPattern::Literal(literal) => match literal {
            RAstPatternLiteral::Int(_) => {
                if rType_is_numeric(expression_type) {
                    return; // numeric expression matches on numeric pattern
                } else {
                    RType::Usize
                }
            },
            RAstPatternLiteral::Char(_) => RType::Char,
            RAstPatternLiteral::Bool(_) => RType::Bool,
        },
        RAstPattern::Identifier(_, _) | RAstPattern::Wildcard => return, // type agnostic
        RAstPattern::EnumVariant(enum_name, _, _) => RType::Custom(string_clone(enum_name)),
    };

    semantic_expect_type_match(&pattern_type, &expression_type);
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
fn codegen_scope_insert(codegen: &mut Codegen, name: String, ty: RType, pointer_name: String) {
    let Codegen::Codegen(_, _, _, stack, _): &mut Codegen = codegen;
    let _ = stringMapStack_insert::<STPair>(stack, name, STPair::ST(pointer_name, ty));
}

/// Lookup variable slot information.
fn codegen_scope_lookup(Codegen::Codegen(_, _, _, stack, _): &Codegen, name: &String) -> STPair {
    match stringMapStack_lookup::<STPair>(stack, name) {
        Option::Some(variable) => stPair_clone(variable),
        Option::None => STPair::ST(string_new(), RType::Unit), // should not be reachable
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
    ST(String, RType),
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
            RAstItem::Function(function) => codegen_function(codegen, function),
            _ => {}, // enum definitions do not generate code
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

/// Emit LLVM-IR for one function definition.
fn codegen_function(codegen: &mut Codegen, function: &RAstFunction) {
    let RAstFunction::Function(_, function_name, parameters, return_type, body): &RAstFunction =
        function;

    let llvm_return_type: String = if string_eq(function_name, &string("main")) {
        codegen_mark_as_main(codegen, true);

        if rType_eq(&return_type, &RType::Unit) {
            string("i64")
        } else {
            rType_to_llvm_name(&return_type)
        }
    } else {
        rType_to_llvm_name(&return_type)
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
                let param_ptr: String = codegen_emit_alloca(codegen, param_type);
                let mut param_register: String = string("%");
                string_push_string(&mut param_register, name);
                codegen_emit_store(codegen, param_type, &param_register, &param_ptr);

                let name: String = string_clone(name);
                codegen_scope_insert(codegen, name, rType_clone(param_type), param_ptr);
            },
            _ => {},
        }

        parameter_index = parameter_index + 1;
    }

    let STPair::ST(value_name, block_type): STPair = codegen_block(codegen, body);
    match &return_type {
        RType::Unit | RType::Never => {
            if codegen_is_main(codegen) {
                // exit with success
                codegen_emit_ret_value(codegen, &RType::Usize, &integer_to_string(0));
            } else {
                codegen_emit_ret_void(codegen);
            }
        },
        _ => {
            if rType_eq(&block_type, &RType::Never) {
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
    let mut block_type: RType = RType::Unit;
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

                if rType_eq(&ty, &RType::Never) {
                    // the rest of the block becomes unreachable, so the block type becomes Never
                    block_type = RType::Never;
                }
            },
        }
        i = i + 1;
    }

    let STPair::ST(name, mut ty) = match tail {
        Option::Some(expression) => codegen_expression(codegen, box_deref::<RAstExpr>(expression)),
        Option::None => STPair::ST(string_new(), RType::Unit),
    };

    if rType_eq(&block_type, &RType::Never) {
        // set type of block to Never to indicate that it doesn't return normally
        ty = RType::Never;
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
                let lvalue_pointer: String = codegen_emit_alloca(codegen, binding_type);
                codegen_emit_store(codegen, binding_type, &rvalue_name, &lvalue_pointer);

                let name: String = string_clone(lvalue_name);
                codegen_scope_insert(codegen, name, rType_clone(binding_type), lvalue_pointer);
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
        RAstExpr::Path(path, arguments) => codegen_path(codegen, path, arguments),
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
                codegen_emit_ret_value(codegen, &RType::Usize, &string("0"));
            } else {
                codegen_emit_ret_void(codegen);
            }
        },
    }

    STPair::ST(string_new(), RType::Never)
}

/// Emit LLVM-IR for an assignment expression.
fn codegen_assignment(codegen: &mut Codegen, left: &RAstExpr, right: &RAstExpr) -> STPair {
    let STPair::ST(right_name, _): STPair = codegen_expression(codegen, right);
    let STPair::ST(pointer_name, left_type): STPair = codegen_assignment_lvalue(codegen, left);

    codegen_emit_store(codegen, &left_type, &right_name, &pointer_name);
    STPair::ST(right_name, RType::Unit)
}

fn codegen_assignment_lvalue(codegen: &mut Codegen, expression: &RAstExpr) -> STPair {
    match expression {
        RAstExpr::VariableUse(name) => codegen_scope_lookup(codegen, name),

        RAstExpr::Unary(RAstUnaryOp::Dereference, value) => {
            let STPair::ST(pointer_name, pointer_type): STPair =
                codegen_expression(codegen, box_deref::<RAstExpr>(value));

            match pointer_type {
                RType::Reference(inner, _) => {
                    let ty: RType = rType_clone(box_deref::<RType>(&inner));
                    STPair::ST(pointer_name, ty)
                },
                RType::RawPointerMut(inner) => {
                    let ty: RType = rType_clone(box_deref::<RType>(&inner));
                    STPair::ST(pointer_name, ty)
                },
                _ => STPair::ST(string_new(), RType::Unit), // should not be reachable
            }
        },
        _ => STPair::ST(string_new(), RType::Unit), // should not be reachable
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
            STPair::ST(name, RType::Bool)
        },
    }
}

/// Emit LLVM-IR for a cast expression.
fn codegen_cast(codegen: &mut Codegen, value: &RAstExpr, to_type: &RType) -> STPair {
    let STPair::ST(from_name, from_type): STPair = codegen_expression(codegen, value);
    let to_type: RType = rType_clone(to_type);

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
                    RType::Reference(box_new::<RType>(ty), *mutable_ref),
                )
            },
            _ => {
                let STPair::ST(name, ty): STPair = codegen_expression(codegen, value);
                let reference: String = codegen_emit_alloca(codegen, &ty);
                codegen_emit_store(codegen, &ty, &name, &reference);
                STPair::ST(
                    reference,
                    RType::Reference(box_new::<RType>(ty), *mutable_ref),
                )
            },
        },

        RAstUnaryOp::Dereference => {
            let STPair::ST(name, ty): STPair = codegen_expression(codegen, value);
            let inner_type: RType = match ty {
                RType::Reference(pointed, _) => rType_clone(box_deref::<RType>(&pointed)),
                RType::RawPointerMut(pointed) => rType_clone(box_deref::<RType>(&pointed)),
                _ => RType::Unit, // should be unreachable
            };
            let name: String = codegen_emit_load(codegen, &inner_type, &name);
            STPair::ST(name, inner_type)
        },
    }
}

/// Emit LLVM-IR for a literal expression.
fn codegen_literal(literal: &RLiteral) -> STPair {
    match literal {
        RLiteral::Int(value) => STPair::ST(integer_to_string(*value), RType::Usize),
        RLiteral::Char(value) => STPair::ST(integer_to_string(*value as usize), RType::Char),
        RLiteral::Bool(value) => STPair::ST(integer_to_string(*value as usize), RType::Bool),
        RLiteral::String(_) => STPair::ST(
            string_new(),
            RType::Reference(box_new::<RType>(RType::Custom(string("str"))), false),
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
fn codegen_path(codegen: &mut Codegen, path: &Vec<String>, values: &Vec<RAstExpr>) -> STPair {
    let function_name: String = rAstPath_to_string(path);

    let mut value_types: Vec<RType> = vec_new::<RType>();
    let mut value_names: Vec<String> = vec_new::<String>();
    let mut i: usize = 0;
    while i < vec_len::<RAstExpr>(values) {
        let value: &RAstExpr = vec_at::<RAstExpr>(values, i);

        let STPair::ST(value_name, value_type): STPair = codegen_expression(codegen, value);

        vec_push::<RType>(&mut value_types, value_type);
        vec_push::<String>(&mut value_names, value_name);
        i = i + 1;
    }

    match codegen_function_signature(codegen, &function_name) {
        Option::Some(FnSignature::Fn(_, return_type, _)) => {
            let result_name: String = if type_has_value(&return_type) {
                codegen_emit_call_value(
                    codegen,
                    &function_name,
                    &return_type,
                    &value_types,
                    &value_names,
                )
            } else {
                codegen_emit_call_void(codegen, &function_name, &value_types, &value_names);
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
    let result_pointer: String = codegen_emit_alloca(codegen, &RType::Unit);
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

            if_type = rType_coerce(if_type, else_type);
        },
        _ => if_type = RType::Unit, // else is implicitly unit, so type of if must be unit
    }

    // end of else block, so jump to the end
    codegen_emit_br(codegen, &end_label);

    // start of the merge block
    codegen_emit_label(codegen, &end_label);

    // load and return the value if there is one
    let result: String = if type_has_value(&if_type) {
        // now we know the type and thus the size to allocate on the stack
        codegen_fixup_alloca(codegen, alloca_idx, &if_type);

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

    STPair::ST(string_new(), RType::Unit) // while always returns unit
}

/// Emit LLVM-IR for a match expression.
fn codegen_match(codegen: &mut Codegen, value: &RAstExpr, arms: &Vec<RAstArm>) -> STPair {
    let STPair::ST(expr_name, expr_type): STPair = codegen_expression(codegen, value);

    let end_label: String = codegen_next_label(codegen, "match.end");

    // Allocate memory for potential result value, though size is still unknown.
    // In the event that the result type is unit, this instruction will be removed later.
    let result_pointer: String = codegen_emit_alloca(codegen, &RType::Unit);
    let alloca_idx: usize = codegen_code_last_index(codegen);

    let mut return_type: RType = RType::Never; // still unknown, coercing arm types yields correct type

    let mut i: usize = 0;
    while i < vec_len::<RAstArm>(arms) {
        codegen_push_scope(codegen);

        let is_last_arm: bool = i == vec_len::<RAstArm>(arms) - 1;
        let arm: &RAstArm = vec_at::<RAstArm>(arms, i);

        let arm_type: RType = codegen_arm(
            codegen,
            arm,
            is_last_arm,
            &expr_name,
            &expr_type,
            &result_pointer,
            &end_label,
        );

        return_type = rType_coerce(return_type, arm_type);
        codegen_pop_scope(codegen);
        i = i + 1;
    }

    // start of the merge block
    codegen_emit_label(codegen, &end_label);

    let result: String = if type_has_value(&return_type) {
        // now we know the type and thus the size to allocate on the stack
        codegen_fixup_alloca(codegen, alloca_idx, &return_type);

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
    expr_type: &RType,
    result_pointer: &String,
    end_label: &String,
) -> RType {
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
                let pointer_name: String = codegen_emit_alloca(codegen, &expr_type);
                codegen_emit_store(codegen, &expr_type, &expr_name, &pointer_name);

                let variable_name: String = string_clone(identifier);
                let variable_type: RType = rType_clone(&expr_type);
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

/// Emit the given string as a new line of LLVM-IR code.
fn codegen_emit_line(codegen: &mut Codegen, line: String) {
    let Code::Code(lines): &mut Code = codegen_code_mut(codegen);
    vec_push::<String>(lines, line);
}

/// Emit a binary arithmetic instruction.
/// ```llvm
/// %<name> = <op> <ty> <lhs>, <rhs>
/// ```
/// Returns `%<name>`.
fn codegen_emit_binary(
    codegen: &mut Codegen,
    op: &RAstArithmeticOp,
    ty: &RType,
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
    string_push_string(&mut line, &rType_to_llvm_name(ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, lhs);
    string_push(&mut line, ',');
    string_push_string(&mut line, rhs);

    codegen_emit_line(codegen, line);
    name
}

/// Emit an integer-comparison instruction.
/// ```llvm
/// %<name> = icmp <op> <ty> <lhs>, <rhs>
/// ```
/// Returns `%<name>`.
fn codegen_emit_icmp(
    codegen: &mut Codegen,
    op: &RAstComparisonOp,
    ty: &RType,
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
    string_push_string(&mut line, &rType_to_llvm_name(ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, lhs);
    string_push(&mut line, ',');
    string_push_string(&mut line, rhs);

    codegen_emit_line(codegen, line);
    name
}

/// Emit a return instruction with a value.
/// ```llvm
/// ret <ty> <value>
/// ```
fn codegen_emit_ret_value(codegen: &mut Codegen, ty: &RType, value: &String) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_str(&mut line, "ret ");
    string_push_string(&mut line, &rType_to_llvm_name(ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, value);

    codegen_emit_line(codegen, line);
}

/// Emit a return instruction with no value.
/// ```llvm
/// ret void
/// ```
fn codegen_emit_ret_void(codegen: &mut Codegen) {
    codegen_emit_line(codegen, string("  ret void"));
}

/// Emit a basic block label.
/// ```llvm
/// <label>:
/// ```
fn codegen_emit_label(codegen: &mut Codegen, label: &String) {
    let mut line: String = string_new();
    string_push(&mut line, '\n');
    string_push_string(&mut line, label);
    string_push(&mut line, ':');

    codegen_emit_line(codegen, line);
}

/// Emit an unconditional branch.
/// ```llvm
/// br label %<target_label>
/// ```
fn codegen_emit_br(codegen: &mut Codegen, target_label: &String) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  br label %");
    string_push_string(&mut line, target_label);

    codegen_emit_line(codegen, line);
}

/// Emit a conditional branch.
/// ```llvm
/// br i1 <condition>, label %<then_label>, label %<else_label>
/// ```
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

/// Emit a cast instruction.
/// ```llvm
/// %<name> = <op> <from_type> <value> to <to_type>
/// ```
/// Returns `%<name>`.
fn codegen_emit_cast(
    codegen: &mut Codegen,
    op: &str,
    from_type: &RType,
    to_type: &RType,
    value: &String,
) -> String {
    let name: String = codegen_next_register(codegen);

    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = ");
    string_push_str(&mut line, op);
    string_push(&mut line, ' ');
    string_push_string(&mut line, &rType_to_llvm_name(from_type));
    string_push(&mut line, ' ');
    string_push_string(&mut line, value);
    string_push_str(&mut line, " to ");
    string_push_string(&mut line, &rType_to_llvm_name(to_type));
    string_push(&mut line, '\n');

    codegen_emit_line(codegen, line);
    name
}

/// Emit a zero-extend instruction.
/// ```llvm
/// %<name> = zext <from_type> <value> to <to_type>
/// ```
/// Returns `%<name>`.
fn codegen_emit_zext(
    codegen: &mut Codegen,
    from_type: &RType,
    to_type: &RType,
    value: &String,
) -> String {
    codegen_emit_cast(codegen, "zext", from_type, to_type, value)
}

/// Emit a truncate instruction.
/// ```llvm
/// %<name> = trunc <from_type> <value> to <to_type>
/// ```
/// Returns `%<name>`.
fn codegen_emit_trunc(
    codegen: &mut Codegen,
    from_type: &RType,
    to_type: &RType,
    value: &String,
) -> String {
    codegen_emit_cast(codegen, "trunc", from_type, to_type, value)
}

/// Emit an integer-to-pointer instruction.
/// ```llvm
/// %<name> = inttoptr <from_type> <value> to <to_type>
/// ```
/// Returns `%<name>`.
fn codegen_emit_inttoptr(
    codegen: &mut Codegen,
    from_type: &RType,
    to_type: &RType,
    value: &String,
) -> String {
    codegen_emit_cast(codegen, "inttoptr", from_type, to_type, value)
}

/// Emit a pointer-to-integer instruction.
/// ```llvm
/// %<name> = ptrtoint <from_type> <value> to <to_type>
/// ```
/// Returns `%<name>`.
fn codegen_emit_ptrtoint(
    codegen: &mut Codegen,
    from_type: &RType,
    to_type: &RType,
    value: &String,
) -> String {
    codegen_emit_cast(codegen, "ptrtoint", from_type, to_type, value)
}

/// Emit an allocate instruction.
/// ```llvm
/// %<name> = alloca <ty>
/// ```
/// Returns `%<name>`.
fn codegen_emit_alloca(codegen: &mut Codegen, ty: &RType) -> String {
    let name: String = codegen_next_register(codegen);

    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = alloca ");
    string_push_string(&mut line, &rType_to_llvm_name(ty));

    codegen_emit_line(codegen, line);
    name
}

/// Emit a store instruction.
/// ```llvm
/// store <ty> <value>, ptr <pointer>
/// ```
fn codegen_emit_store(codegen: &mut Codegen, ty: &RType, value: &String, pointer: &String) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  store ");
    string_push_string(&mut line, &rType_to_llvm_name(ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, value);
    string_push(&mut line, ',');
    string_push_str(&mut line, " ptr ");
    string_push_string(&mut line, pointer);

    codegen_emit_line(codegen, line);
}

/// Emit a load instruction.
/// ```llvm
/// %<name> = load <ty>, ptr <pointer>
/// ```
/// Returns `%<name>`.
fn codegen_emit_load(codegen: &mut Codegen, ty: &RType, pointer: &String) -> String {
    let name: String = codegen_next_register(codegen);
    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = load ");
    string_push_string(&mut line, &rType_to_llvm_name(ty));
    string_push(&mut line, ',');
    string_push_str(&mut line, " ptr ");
    string_push_string(&mut line, pointer);

    codegen_emit_line(codegen, line);
    name
}

/// Emit pointer arithmetic using integer casts and addition.
/// This is a substitute for `getelementptr`, which is the instruction usually used for this.
/// ```llvm
/// %t0 = ptrtoint ptr <pointer> to i64
/// %t1 = add i64 %t0, <size_of(ty) * index>
/// %<name> = inttoptr i64 %t1 to ptr
/// ```
/// Returns `%<name>`.
fn emit_pointer_add(codegen: &mut Codegen, pointer: &String, ty: &RType, index: usize) -> String {
    let ptr_type: RType = RType::RawPointerMut(box_new(RType::Unit)); // dummy type to use `ptr` type
    let addition: RAstArithmeticOp = RAstArithmeticOp::Add;
    let offset: String = integer_to_string(index * rType_size(ty));

    let t0: String = codegen_emit_ptrtoint(codegen, &ptr_type, &RType::Usize, pointer);
    let t1: String = codegen_emit_binary(codegen, &addition, &RType::Usize, &t0, &offset);
    let name: String = codegen_emit_inttoptr(codegen, &RType::Usize, &ptr_type, &t1);

    name
}

/// Emit a call instruction that returns a value.
/// ```llvm
/// %<name> = call <return_type> @<function_name>(<arg_type> <arg_value>, ...)
/// ```
/// Returns `%<name>`.
fn codegen_emit_call_value(
    codegen: &mut Codegen,
    function_name: &String,
    return_type: &RType,
    argument_types: &Vec<RType>,
    argument_values: &Vec<String>,
) -> String {
    let name: String = codegen_next_register(codegen);

    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = call ");
    string_push_string(&mut line, &rType_to_llvm_name(return_type));
    string_push_str(&mut line, " @");
    string_push_string(&mut line, function_name);
    string_push(&mut line, '(');

    let mut i: usize = 0;
    let len: usize = vec_len::<RType>(argument_types);
    while i < len {
        let argument_type: &RType = vec_at::<RType>(argument_types, i);
        let argument_value: &String = vec_at::<String>(argument_values, i);
        string_push_string(&mut line, &rType_to_llvm_name(argument_type));
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

/// Emit a call instruction that returns `void`.
/// ```llvm
/// call void @<function_name>(<arg_type> <arg_value>, ...)
/// ```
fn codegen_emit_call_void(
    codegen: &mut Codegen,
    function_name: &String,
    argument_types: &Vec<RType>,
    argument_values: &Vec<String>,
) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  call void @");
    string_push_string(&mut line, function_name);
    string_push(&mut line, '(');

    let mut i: usize = 0;
    let len: usize = vec_len::<RType>(argument_types);
    while i < len {
        let argument_type: &RType = vec_at::<RType>(argument_types, i);
        let argument_value: &String = vec_at::<String>(argument_values, i);
        string_push_string(&mut line, &rType_to_llvm_name(argument_type));
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

/// Emit a function header with an entry label.
/// ```llvm
/// define <return_type> @<fn_name>(<param_type> %<param_name>, ...) {
/// entry:
/// ```
/// Does not return a value.
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

        string_push_string(&mut line, &rType_to_llvm_name(parameter_type));
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

/// Emit an LLVM `declare` for an extern function.
/// ```llvm
/// declare <return_type> @<fn_name>(<param_type>, ...)
/// ```
/// Does not return a value.
fn codegen_emit_declare(
    codegen: &mut Codegen,
    fn_name: &String,
    parameters: &Vec<RAstVariable>,
    return_type: &RType,
) {
    let mut line: String = string_new();
    string_push_str(&mut line, "declare ");
    string_push_string(&mut line, &rType_to_llvm_name(return_type));
    string_push_str(&mut line, " @");
    string_push_string(&mut line, fn_name);
    string_push_str(&mut line, "(");

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstVariable>(parameters);
    while i < len {
        let RAstVariable::Variable(_, parameter_type): &RAstVariable =
            vec_at::<RAstVariable>(parameters, i);

        string_push_string(&mut line, &rType_to_llvm_name(parameter_type));

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
fn codegen_fixup_alloca(codegen: &mut Codegen, index: usize, new_type: &RType) {
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

    string_push_string(&mut new_alloca, &rType_to_llvm_name(new_type));

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
enum LToken {
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

/// The lexer state of the scanned LLVM-IR code.
enum LLexer {
    /// LLVM-IR human-readable source file, current token
    Lexer(SourceFile, LToken),
}

/// Create a new LLVM lexer and scan the first token.
fn lLexer_new(source: String) -> LLexer {
    let source_file: SourceFile = SourceFile::SourceFile(source, 0, 1, 0);
    let mut lexer: LLexer = LLexer::Lexer(source_file, LToken::Eof);
    lLexer_next_token(&mut lexer);
    lexer
}

/// Get the lexer source file.
fn lLexer_sourcefile(LLexer::Lexer(source, _): &LLexer) -> &SourceFile {
    source
}

/// Get the lexer source file.
fn lLexer_sourcefile_mut(LLexer::Lexer(source, _): &mut LLexer) -> &mut SourceFile {
    source
}

/// Get the current lexer token.
fn lLexer_current_token(LLexer::Lexer(_, token): &LLexer) -> &LToken {
    token
}

/// Set the current lexer token.
fn lLexer_set_current_token(LLexer::Lexer(_, old_token): &mut LLexer, token: LToken) {
    *old_token = token;
}

/// Peek the current source character.
fn lLexer_peek_char(lexer: &LLexer) -> Option<char> {
    let SourceFile::SourceFile(string, index, _, _): &SourceFile = lLexer_sourcefile(lexer);
    string_get(string, *index)
}

/// Peek the next source character after the current one and return true if it is the expected
/// character
fn lLexer_next_char_eq(lexer: &LLexer, expected: char) -> bool {
    let SourceFile::SourceFile(content, index, _, _): &SourceFile = lLexer_sourcefile(lexer);
    match string_get(content, *index + 1) {
        Option::Some(character) => character == expected,
        _ => false,
    }
}

fn lLexer_expect_char(lexer: &mut LLexer, expected: char) {
    match lLexer_consume_char(lexer) {
        Option::Some(c) => {
            if c != expected {
                panic("unexpected character");
            }
        },
        _ => panic("unexpected EOF"),
    }
}

/// Consume and return the current source character.
fn lLexer_consume_char(lexer: &mut LLexer) -> Option<char> {
    let SourceFile::SourceFile(source, index, line, last_newline_idx): &mut SourceFile =
        lLexer_sourcefile_mut(lexer);

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
fn lLexer_next_token(lexer: &mut LLexer) -> LToken {
    lLexer_skip_whitespace_and_comments(lexer);

    let token: LToken = match lLexer_peek_char(lexer) {
        Option::Some(ch) => {
            if and(ch == 'c', lLexer_next_char_eq(lexer, '"')) {
                let value: String = lLexer_scan_cstring(lexer);
                LToken::CString(value)
            } else if or(is_alpha(ch), ch == '.') {
                let ident: String = lLexer_scan_identifier_or_keyword(lexer);
                llvm_identifier_to_token(ident)
            } else if is_digit(ch) {
                let value: usize = lLexer_scan_integer(lexer);
                LToken::Integer(value)
            } else {
                lLexer_scan_symbol(lexer)
            }
        },
        Option::None => LToken::Eof,
    };

    lLexer_set_current_token(lexer, llvmToken_clone(&token));
    token
}

/// Scan and return a c"..." string literal.
fn lLexer_scan_cstring(lexer: &mut LLexer) -> String {
    let mut literal: String = string_new();
    lLexer_expect_char(lexer, 'c');
    lLexer_expect_char(lexer, '"');

    while true {
        match lLexer_consume_char(lexer) {
            Option::Some('"') => return literal,
            Option::Some('\\') => {
                let character: char = lLexer_scan_escape(lexer);
                string_push(&mut literal, character);
            },
            Option::Some(ch) => string_push(&mut literal, ch),
            Option::None => panic("unterminated LLVM c-string"),
        }
    }
    literal // satisfy compiler
}

fn lLexer_scan_escape(lexer: &mut LLexer) -> char {
    match lLexer_consume_char(lexer) {
        Option::Some(hex_digit) => {
            if is_hexadecimal_digit(hex_digit) {
                match lLexer_consume_char(lexer) {
                    Option::Some(second_hex_digit) => {
                        let mut char_byte: String = string_new();
                        string_push(&mut char_byte, hex_digit);
                        string_push(&mut char_byte, second_hex_digit);

                        unwrap::<usize>(string_to_integer(&char_byte, 16)) as u8 as char
                    },
                    _ => panic("expected second digit for escaped character byte"),
                }
            } else {
                hex_digit
            }
        },
        Option::None => panic("unterminated LLVM c-string"),
    }
}

fn lLexer_scan_identifier_or_keyword(lexer: &mut LLexer) -> String {
    let mut identifier: String = string_new();
    while true {
        match lLexer_peek_char(lexer) {
            Option::Some(ch) => {
                if is_alphanumeric_or_dot(ch) {
                    lLexer_consume_char(lexer);
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

fn llvm_identifier_to_token(identifier: String) -> LToken {
    if string_eq(&identifier, &string("define")) {
        LToken::Define
    } else if string_eq(&identifier, &string("declare")) {
        LToken::Declare
    } else if string_eq(&identifier, &string("ret")) {
        LToken::Ret
    } else if string_eq(&identifier, &string("inttoptr")) {
        LToken::IntToPtr
    } else if string_eq(&identifier, &string("ptrtoint")) {
        LToken::PtrToInt
    } else if string_eq(&identifier, &string("br")) {
        LToken::Br
    } else if string_eq(&identifier, &string("label")) {
        LToken::Label
    } else if string_eq(&identifier, &string("add")) {
        LToken::Add
    } else if string_eq(&identifier, &string("sub")) {
        LToken::Sub
    } else if string_eq(&identifier, &string("mul")) {
        LToken::Mul
    } else if string_eq(&identifier, &string("udiv")) {
        LToken::Udiv
    } else if string_eq(&identifier, &string("urem")) {
        LToken::Urem
    } else if string_eq(&identifier, &string("icmp")) {
        LToken::Icmp
    } else if string_eq(&identifier, &string("zext")) {
        LToken::Zext
    } else if string_eq(&identifier, &string("trunc")) {
        LToken::Trunc
    } else if string_eq(&identifier, &string("alloca")) {
        LToken::Alloca
    } else if string_eq(&identifier, &string("store")) {
        LToken::Store
    } else if string_eq(&identifier, &string("load")) {
        LToken::Load
    } else if string_eq(&identifier, &string("to")) {
        LToken::To
    } else if string_eq(&identifier, &string("call")) {
        LToken::Call
    } else if string_eq(&identifier, &string("constant")) {
        LToken::Constant
    } else if string_eq(&identifier, &string("eq")) {
        LToken::Eq
    } else if string_eq(&identifier, &string("ne")) {
        LToken::Ne
    } else if string_eq(&identifier, &string("ugt")) {
        LToken::Ugt
    } else if string_eq(&identifier, &string("uge")) {
        LToken::Uge
    } else if string_eq(&identifier, &string("ult")) {
        LToken::Ult
    } else if string_eq(&identifier, &string("ule")) {
        LToken::Ule
    } else if string_eq(&identifier, &string("ptr")) {
        LToken::Ptr
    } else if string_eq(&identifier, &string("i64")) {
        LToken::I64
    } else if string_eq(&identifier, &string("i8")) {
        LToken::I8
    } else if string_eq(&identifier, &string("i1")) {
        LToken::I1
    } else if string_eq(&identifier, &string("void")) {
        LToken::Void
    } else {
        LToken::Identifier(identifier)
    }
}

fn lLexer_scan_integer(lexer: &mut LLexer) -> usize {
    let mut value: usize = 0;
    while true {
        match lLexer_peek_char(lexer) {
            Option::Some(ch) => {
                if is_digit(ch) {
                    let digit: usize = (ch as usize) - ('0' as usize);
                    value = value * 10 + digit;
                    lLexer_consume_char(lexer);
                } else {
                    return value;
                }
            },
            Option::None => return value,
        }
    }
    value
}

fn lLexer_scan_symbol(lexer: &mut LLexer) -> LToken {
    match unwrap::<char>(lLexer_consume_char(lexer)) {
        '@' => LToken::At,
        '%' => LToken::Percent,
        '(' => LToken::LParen,
        ')' => LToken::RParen,
        '{' => LToken::LBrace,
        '}' => LToken::RBrace,
        '[' => LToken::LBracket,
        ']' => LToken::RBracket,
        ',' => LToken::Comma,
        '=' => LToken::Assign,
        ':' => LToken::Colon,
        _ => panic("unsupported token in LLVM input"),
    }
}

fn lLexer_skip_whitespace_and_comments(lexer: &mut LLexer) {
    while true {
        match lLexer_peek_char(lexer) {
            Option::Some(ch) => {
                if is_whitespace(ch) {
                    lLexer_consume_char(lexer);
                } else if ch == ';' {
                    lLexer_consume_char(lexer);
                    lLexer_skip_line(lexer);
                } else {
                    return;
                }
            },
            Option::None => return,
        }
    }
}

fn lLexer_skip_line(lexer: &mut LLexer) {
    while true {
        match lLexer_consume_char(lexer) {
            Option::Some('\n') => return,
            Option::Some(_) => (),
            Option::None => return,
        }
    }
}

// -----------------------------------------------------------------
// ------------------------- Parser --------------------------------
// -----------------------------------------------------------------

/// The parser state for a LLVM-IR module.
enum Parser {
    Parser(LLexer, LAst, LLocalSymTable),
}

/// Create an LLVM parser and prime the first token.
fn parser_new(source: String) -> Parser {
    Parser::Parser(lLexer_new(source), lAst_new(), lLocalSymbolTable_new())
}

/// Get immutable parser lexer access.
fn parser_lexer(Parser::Parser(lexer, _, _): &Parser) -> &LLexer {
    lexer
}

/// Get mutable parser lexer access.
fn parser_lexer_mut(Parser::Parser(lexer, _, _): &mut Parser) -> &mut LLexer {
    lexer
}

/// Get mutable parser AST access.
fn parser_ast_mut(Parser::Parser(_, ast, _): &mut Parser) -> &mut LAst {
    ast
}

fn parser_local(Parser::Parser(_, _, local): &Parser) -> &LLocalSymTable {
    local
}

fn parser_local_mut(Parser::Parser(_, _, local): &mut Parser) -> &mut LLocalSymTable {
    local
}

/// Parse LLVM source into LLVM AST.
fn parser_parse_to_ast(source: String) -> LAst {
    let mut parser: Parser = parser_new(source);
    parser_parse_language(&mut parser);
    let Parser::Parser(_, ast, _): Parser = parser;
    ast
}

/// Get current LLVM parser token.
fn parser_current_token(parser: &Parser) -> &LToken {
    lLexer_current_token(parser_lexer(parser))
}

/// Consume and return the current LLVM parser token.
fn parser_consume_current_token(parser: &mut Parser) -> LToken {
    let lexer: &LLexer = parser_lexer(parser);
    let token: LToken = llvmToken_clone(lLexer_current_token(lexer));
    parser_next_token(parser);
    token
}

/// Advance and return next LLVM parser token.
fn parser_next_token(parser: &mut Parser) -> LToken {
    lLexer_next_token(parser_lexer_mut(parser))
}

/// Check whether parser current token equals expected token.
fn parser_current_token_eq(parser: &Parser, token: &LToken) -> bool {
    llvmToken_eq(parser_current_token(parser), token)
}

/// Try consuming one token and report success.
fn parser_try_consume(parser: &mut Parser, token: &LToken) -> bool {
    if parser_current_token_eq(parser, token) {
        parser_next_token(parser);
        true
    } else {
        false
    }
}

/// Require and consume one token.
fn parser_expect_token(parser: &mut Parser, token: &LToken) {
    if not(parser_try_consume(parser, token)) {
        let message: String = parser_expected_message(parser, &llvmToken_to_string(token));
        parser_error(parser, &message);
    }
}

/// Read and consume one identifier token.
fn parser_expect_identifier(parser: &mut Parser) -> String {
    match parser_current_token(parser) {
        LToken::Identifier(identifier) => {
            let value: String = string_clone(identifier);
            parser_next_token(parser);
            value
        },
        _ => {
            let message: String = parser_expected_message(parser, &string("LLVM identifier"));
            parser_error(parser, &message)
        },
    }
}

fn parser_expect_value_type(parser: &Parser, value: &LValue, expected: &LType) {
    if not(parser_value_has_type(parser, value, expected)) {
        parser_warning(parser, &string("LLVM value does not match expected type"));
    }
}

fn parser_value_has_type(parser: &Parser, value: &LValue, expected: &LType) -> bool {
    match value {
        LValue::Register(name) => {
            match lLocalSymbolTable_lookup_register_type(parser_local(parser), name) {
                Option::Some(actual) => llvmType_eq(actual, expected),
                Option::None => false,
            }
        },
        LValue::Literal(_) => match expected {
            LType::I1 | LType::I8 | LType::I64 => true, // allow overflows
            _ => false,
        },
        LValue::Global(_) => match expected {
            LType::Ptr => true,
            _ => false,
        },
    }
}

/// Return true if the current token indicates the start of a new instruction.
fn parser_is_instruction_start(parser: &mut Parser) -> bool {
    match parser_current_token(parser) {
        LToken::RBrace | LToken::Identifier(_) => false,
        _ => true,
    }
}

/// Abstract syntax tree of a LLVM-IR module.
enum LAst {
    AST(Vec<LGlobal>, StringMap<LFunction>),
}

/// Top-level LLVM global data.
enum LGlobal {
    /// name, bytes
    String(String, String),
}

/// Create an empty LLVM AST.
fn lAst_new() -> LAst {
    LAst::AST(vec_new::<LGlobal>(), stringMap_new::<LFunction>())
}

/// Get immutable access to the top-level globals list.
fn lAst_globals(LAst::AST(globals, _): &LAst) -> &Vec<LGlobal> {
    globals
}

/// Get mutable access to the top-level globals list.
fn lAst_globals_mut(LAst::AST(globals, _): &mut LAst) -> &mut Vec<LGlobal> {
    globals
}

/// Insert a global entry into the AST. Returns false on duplicate name.
fn lAst_insert_global(ast: &mut LAst, name: String, global: LGlobal) -> bool {
    let globals: &Vec<LGlobal> = lAst_globals(ast);

    let mut i: usize = 0;
    while i < vec_len::<LGlobal>(globals) {
        let LGlobal::String(existing_name, _): &LGlobal = vec_at::<LGlobal>(globals, i);
        if string_eq(existing_name, &name) {
            return false;
        }
        i = i + 1;
    }

    vec_push::<LGlobal>(lAst_globals_mut(ast), global);
    true
}

/// Get immutable access to the top-level function map.
fn lAst_functions(LAst::AST(_, functions): &LAst) -> &StringMap<LFunction> {
    functions
}

/// Get mutable access to the top-level function map.
fn lAst_functions_mut(LAst::AST(_, functions): &mut LAst) -> &mut StringMap<LFunction> {
    functions
}

/// Insert a function entry into the AST. Returns false on duplicate name.
fn lAst_insert_function(ast: &mut LAst, name: String, function: LFunction) -> bool {
    if stringMap_contains::<LFunction>(lAst_functions(ast), &name) {
        false
    } else {
        stringMap_insert::<LFunction>(lAst_functions_mut(ast), name, function);
        true
    }
}

/// Lookup a function in the AST by name.
fn lAst_lookup_function(ast: &LAst, name: String) -> &LFunction {
    match stringMap_get::<LFunction>(lAst_functions(ast), &name) {
        Option::Some(function) => function,
        Option::None => panic("unknown LLVM function"),
    }
}

/// Local symbol table for LLVM to track virtual register
enum LLocalSymTable {
    Registers(StringMap<LType>),
}

/// Create an empty LLVM local symbol table.
fn lLocalSymbolTable_new() -> LLocalSymTable {
    LLocalSymTable::Registers(stringMap_new::<LType>())
}

/// Clear local register table buckets.
fn lLocalSymbolTable_clear(symtable: &mut LLocalSymTable) {
    match symtable {
        LLocalSymTable::Registers(registers) => *registers = stringMap_new::<LType>(),
    }
}

/// Insert register name. Returns false on duplicate.
fn lLocalSymbolTable_insert_register(
    LLocalSymTable::Registers(registers): &mut LLocalSymTable,
    name: String,
    ty: LType,
) -> bool {
    let is_defined: bool = stringMap_contains::<LType>(registers, &name);
    stringMap_insert::<LType>(registers, name, ty);
    !is_defined
}

/// Lookup a register type in the local symbol table.
fn lLocalSymbolTable_lookup_register_type<'a>(
    LLocalSymTable::Registers(registers): &'a LLocalSymTable,
    name: &String,
) -> Option<&'a LType> {
    stringMap_get::<LType>(registers, name)
}

/// An executable LLVM-IR function.
enum LFunction {
    /// return type, parameters, basic blocks
    // TODO: use StringMap for InstructionBlocks
    Function(LType, Vec<LParameter>, Vec<InstructionBlock>),
    /// return type, parameters, builtin
    BuiltIn(BuiltIn, LType, Vec<LParameter>),
}

/// Supported LLVM-IR declared functions.
enum BuiltIn {
    Exit,
    Malloc,
}

/// Represents a parameter of an LLVM function.
enum LParameter {
    /// identifier, type
    Parameter(String, LType),
}

/// Supported LLVM types in the subset.
#[derive(Debug)]
enum LType {
    I1,
    I8,
    I64,
    Ptr,
    Array(usize, Box<LType>),
    Void,
}

fn llvmType_bitwidth(ty: &LType) -> usize {
    match ty {
        LType::I1 => 1,
        LType::I8 => 8,
        LType::I64 => 64,
        LType::Ptr => size_of::<usize>() * 8,
        LType::Array(len, inner) => *len * llvmType_bitwidth(box_deref::<LType>(inner)),
        LType::Void => 0,
    }
}

/// Return the size of an LLVM type in bytes.
fn llvmType_size(ty: &LType) -> usize {
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
    panic("unknown LLVM block label");
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
    Store(LType, LValue, LValue),
    Call(Call),
    /// return type, optional value
    Ret(LType, Option<LValue>),
    Br(Branch),
}

/// A `call` instruction.
enum Call {
    /// return type, callee, arguments
    Call(LType, String, Vec<LTypedValue>),
}

/// Represents "br", either a conditional or unconditional jump.
enum Branch {
    /// label
    Unconditional(String),
    /// condition, then label, else label
    Conditional(LValue, String, String),
}

/// Represents an assignment instruction.
enum AssignInstruction {
    Assign(String, AssignOp),
}

/// Represents the right-hand-side of an assignment
enum AssignOp {
    /// operation, type, left operand, right operand
    Binary(BinaryOp, LType, LValue, LValue),
    /// operation, operand type, left operand, right operand
    Icmp(IcmpOp, LType, LValue, LValue),
    /// operation, target type, value
    Cast(CastOp, LType, LValue),
    /// allocated type
    Alloca(LType),
    /// loaded type, address
    Load(LType, LValue),
    Call(Call),
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

fn assignOp_get_type(operation: &AssignOp) -> LType {
    match operation {
        AssignOp::Binary(_, ty, _, _) => llvmType_clone(ty),
        AssignOp::Icmp(_, _, _, _) => LType::I1,
        AssignOp::Call(Call::Call(ty, _, _)) => llvmType_clone(ty),
        AssignOp::Cast(_, ty, _) => llvmType_clone(ty),
        AssignOp::Alloca(_) => LType::Ptr,
        AssignOp::Load(ty, _) => llvmType_clone(ty),
    }
}

/// Represents an LLVM value operand.
#[derive(Debug)]
enum LValue {
    /// identifier
    Register(String),
    /// integer value
    Literal(usize),
    /// identifier
    Global(String),
}

/// Represents a value with a specified type.
// TODO: drop this: the AST does not need to know about types. Parser ensures type safety.
enum LTypedValue {
    Pair(LType, LValue),
}

fn parser_parse_language(parser: &mut Parser) {
    while not(parser_current_token_eq(parser, &LToken::Eof)) {
        match parser_current_token(parser) {
            LToken::At => parser_parse_string(parser),
            LToken::Define => parser_parse_function(parser),
            LToken::Declare => parser_parse_declare(parser),
            _ => {
                let message: String =
                    parser_expected_message(parser, &string("LLVM top-level item"));
                parser_error(parser, &message)
            },
        }
    }
}

fn parser_parse_string(parser: &mut Parser) {
    let name: String = parser_parse_global_name(parser);
    parser_expect_token(parser, &LToken::Assign);
    parser_expect_token(parser, &LToken::Constant);
    parser_parse_type(parser);

    match parser_current_token(parser) {
        LToken::CString(value) => {
            let string_value: String = string_clone(value);
            parser_next_token(parser);
            if not(lAst_insert_global(
                parser_ast_mut(parser),
                string_clone(&name),
                LGlobal::String(name, string_value),
            )) {
                parser_error(parser, &string("duplicate LLVM global string"));
            }
        },
        _ => {
            let message: String = parser_expected_message(parser, &string("LLVM c-string literal"));
            parser_error(parser, &message)
        },
    }
}

fn parser_parse_function(parser: &mut Parser) {
    parser_expect_token(parser, &LToken::Define);
    let return_type: LType = parser_parse_type(parser);
    let function_name: String = parser_parse_global_name(parser);

    lLocalSymbolTable_clear(parser_local_mut(parser));

    let parameters: Vec<LParameter> = parser_parse_parameters(parser, true);

    parser_expect_token(parser, &LToken::LBrace);
    let blocks: Vec<InstructionBlock> = parser_parse_blocks(parser);
    parser_expect_token(parser, &LToken::RBrace);

    let function: LFunction = LFunction::Function(return_type, parameters, blocks);
    if not(lAst_insert_function(
        parser_ast_mut(parser),
        function_name,
        function,
    )) {
        parser_error(parser, &string("duplicate LLVM function definition"));
    }
}

fn parser_parse_declare(parser: &mut Parser) {
    parser_expect_token(parser, &LToken::Declare);
    let return_type: LType = parser_parse_type(parser);
    let function_name: String = parser_parse_global_name(parser);

    lLocalSymbolTable_clear(parser_local_mut(parser));
    let parameters: Vec<LParameter> = parser_parse_parameters(parser, false);

    let builtin: BuiltIn = if string_eq(&function_name, &string("malloc")) {
        BuiltIn::Malloc
    } else if string_eq(&function_name, &string("exit")) {
        BuiltIn::Exit
    } else {
        parser_error(parser, &string("unknown declared function"));
    };

    let function: LFunction = LFunction::BuiltIn(builtin, return_type, parameters);
    if not(lAst_insert_function(
        parser_ast_mut(parser),
        function_name,
        function,
    )) {
        parser_error(parser, &string("duplicate LLVM function declaration"));
    }
}

/// Parse parameters of a function.
///
/// * `parser`: The parser state
/// * `require_names`: True, if the parameters are named (function definition). False, if they are
/// not (function declaration).
fn parser_parse_parameters(parser: &mut Parser, named: bool) -> Vec<LParameter> {
    let mut parameters: Vec<LParameter> = vec_new::<LParameter>();

    parser_expect_token(parser, &LToken::LParen);

    if not(parser_current_token_eq(parser, &LToken::RParen)) {
        let parameter_type: LType = parser_parse_type(parser);
        let param_name: String = parser_parse_parameter_name(parser, 0);
        lLocalSymbolTable_insert_register(
            parser_local_mut(parser),
            string_clone(&param_name),
            llvmType_clone(&parameter_type),
        );

        let parameter: LParameter = LParameter::Parameter(param_name, parameter_type);
        vec_push::<LParameter>(&mut parameters, parameter);

        while parser_current_token_eq(parser, &LToken::Comma) {
            parser_next_token(parser);

            let parameter_type: LType = parser_parse_type(parser);
            let param_name: String =
                parser_parse_parameter_name(parser, vec_len::<LParameter>(&parameters));

            if named {
                if not(lLocalSymbolTable_insert_register(
                    parser_local_mut(parser),
                    string_clone(&param_name),
                    llvmType_clone(&parameter_type),
                )) {
                    parser_error(parser, &string("duplicate parameters in LLVM function"));
                }
            }

            let parameter: LParameter = LParameter::Parameter(param_name, parameter_type);
            vec_push::<LParameter>(&mut parameters, parameter);
        }
    }
    parser_expect_token(parser, &LToken::RParen);
    parameters
}

fn parser_parse_parameter_name(parser: &mut Parser, index: usize) -> String {
    if parser_current_token_eq(parser, &LToken::Percent) {
        parser_parse_register(parser)
    } else {
        let mut name: String = string("arg");
        string_push_string(&mut name, &integer_to_string(index));
        name
    }
}

fn parser_parse_global_name(parser: &mut Parser) -> String {
    parser_expect_token(parser, &LToken::At);
    parser_expect_identifier(parser)
}

fn parser_parse_blocks(parser: &mut Parser) -> Vec<InstructionBlock> {
    let mut blocks: Vec<InstructionBlock> = vec_new::<InstructionBlock>();
    while not(parser_current_token_eq(parser, &LToken::RBrace)) {
        let block: InstructionBlock = parser_parse_block(parser);
        vec_push::<InstructionBlock>(&mut blocks, block);
    }
    blocks
}

fn parser_parse_block(parser: &mut Parser) -> InstructionBlock {
    let label: String = parser_expect_identifier(parser);
    parser_expect_token(parser, &LToken::Colon);
    // TODO: insert into symbol table

    let mut instructions: Vec<Instruction> = vec_new::<Instruction>();
    while parser_is_instruction_start(parser) {
        let instruction: Instruction = parser_parse_instruction(parser);
        vec_push::<Instruction>(&mut instructions, instruction);
    }

    InstructionBlock::Block(label, instructions)
}

fn parser_parse_register(parser: &mut Parser) -> String {
    parser_expect_token(parser, &LToken::Percent);
    parser_expect_identifier(parser)
}

fn parser_parse_instruction(parser: &mut Parser) -> Instruction {
    match parser_current_token(parser) {
        LToken::Ret => parser_parse_return(parser),
        LToken::Br => parser_parse_branch(parser),
        LToken::Percent => Instruction::Assignment(parser_parse_assignment(parser)),
        LToken::Store => parser_parse_store(parser),
        LToken::Call => {
            parser_next_token(parser);
            Instruction::Call(parser_parse_call(parser))
        },
        _ => {
            let message: String = parser_expected_message(parser, &string("LLVM instruction"));
            parser_error(parser, &message)
        },
    }
}

fn parser_parse_return(parser: &mut Parser) -> Instruction {
    parser_expect_token(parser, &LToken::Ret);
    let returned_type: LType = parser_parse_type(parser);
    let return_value: Option<LValue> = if llvmType_eq(&returned_type, &LType::Void) {
        Option::None
    } else {
        Option::Some(parser_parse_value(parser))
    };
    Instruction::Ret(returned_type, return_value)
}

fn parser_parse_branch(parser: &mut Parser) -> Instruction {
    parser_expect_token(parser, &LToken::Br);
    let branch: Branch = if parser_try_consume(parser, &LToken::Label) {
        let target_label: String = parser_parse_register(parser);
        Branch::Unconditional(target_label)
    } else {
        parser_expect_token(parser, &LToken::I1);
        let condition: LValue = parser_parse_value(parser);
        parser_expect_token(parser, &LToken::Comma);

        parser_expect_token(parser, &LToken::Label);
        let then_label: String = parser_parse_register(parser);
        parser_expect_token(parser, &LToken::Comma);

        parser_expect_token(parser, &LToken::Label);
        let else_label: String = parser_parse_register(parser);

        Branch::Conditional(condition, then_label, else_label)
    };
    Instruction::Br(branch)
}

fn parser_parse_assignment(parser: &mut Parser) -> AssignInstruction {
    let target_register: String = parser_parse_register(parser);

    parser_expect_token(parser, &LToken::Assign);
    let operation: AssignOp = match parser_consume_current_token(parser) {
        LToken::Add => parser_parse_binary_assign(parser, BinaryOp::Add),
        LToken::Sub => parser_parse_binary_assign(parser, BinaryOp::Sub),
        LToken::Mul => parser_parse_binary_assign(parser, BinaryOp::Mul),
        LToken::Udiv => parser_parse_binary_assign(parser, BinaryOp::Udiv),
        LToken::Urem => parser_parse_binary_assign(parser, BinaryOp::Urem),
        LToken::Icmp => parser_parse_icmp_assign(parser),
        LToken::Zext => parser_parse_cast_assign(parser, CastOp::Zext),
        LToken::Trunc => parser_parse_cast_assign(parser, CastOp::Trunc),
        LToken::IntToPtr => parser_parse_cast_assign(parser, CastOp::IntToPtr),
        LToken::PtrToInt => parser_parse_cast_assign(parser, CastOp::PtrToInt),
        LToken::Alloca => parser_parse_alloca_assign(parser),
        LToken::Load => parser_parse_load_assign(parser),
        LToken::Call => parser_parse_call_assign(parser),
        _ => {
            let message: String =
                parser_expected_message(parser, &string("LLVM assignment operation"));
            parser_error(parser, &message)
        },
    };

    if not(lLocalSymbolTable_insert_register(
        parser_local_mut(parser),
        string_clone(&target_register),
        assignOp_get_type(&operation),
    )) {
        parser_warning(parser, &string("SSA: duplicate register assignment"));
    }

    AssignInstruction::Assign(target_register, operation)
}

fn parser_parse_binary_assign(parser: &mut Parser, operator: BinaryOp) -> AssignOp {
    let ty: LType = parser_parse_type(parser);
    let left: LValue = parser_parse_value(parser);
    parser_expect_value_type(parser, &left, &ty);

    parser_expect_token(parser, &LToken::Comma);
    let right: LValue = parser_parse_value(parser);
    parser_expect_value_type(parser, &right, &ty);

    AssignOp::Binary(operator, ty, left, right)
}

fn parser_parse_icmp_assign(parser: &mut Parser) -> AssignOp {
    let predicate: IcmpOp = match parser_consume_current_token(parser) {
        LToken::Eq => IcmpOp::Eq,
        LToken::Ne => IcmpOp::Ne,
        LToken::Ugt => IcmpOp::Ugt,
        LToken::Uge => IcmpOp::Uge,
        LToken::Ult => IcmpOp::Ult,
        LToken::Ule => IcmpOp::Ule,
        _ => {
            let message: String = parser_expected_message(parser, &string("LLVM icmp operator"));
            parser_error(parser, &message)
        },
    };

    let ty: LType = parser_parse_type(parser);
    let left: LValue = parser_parse_value(parser);
    parser_expect_value_type(parser, &left, &ty);

    parser_expect_token(parser, &LToken::Comma);
    let right: LValue = parser_parse_value(parser);
    parser_expect_value_type(parser, &right, &ty);

    AssignOp::Icmp(predicate, ty, left, right)
}

fn parser_parse_call_assign(parser: &mut Parser) -> AssignOp {
    let call: Call = parser_parse_call(parser);

    let Call::Call(return_type, _, _): &Call = &call;
    if llvmType_eq(return_type, &LType::Void) {
        parser_error(parser, &string("cannot assign void to a register"));
    }

    AssignOp::Call(call)
}

fn parser_parse_cast_assign(parser: &mut Parser, operator: CastOp) -> AssignOp {
    let from_type: LType = parser_parse_type(parser);

    let value: LValue = parser_parse_value(parser);
    parser_expect_value_type(parser, &value, &from_type);

    parser_expect_token(parser, &LToken::To);
    let to_type: LType = parser_parse_type(parser);

    match &operator {
        CastOp::Zext => {
            let from_bits: usize = llvmType_bitwidth(&from_type);
            let to_bits: usize = llvmType_bitwidth(&to_type);
            if not(from_bits < to_bits) {
                parser_warning(parser, &string("zext: source is not smaller than target"));
            }
        },
        CastOp::Trunc => {
            let from_bits: usize = llvmType_bitwidth(&from_type);
            let to_bits: usize = llvmType_bitwidth(&to_type);
            if not(from_bits > to_bits) {
                parser_warning(parser, &string("zext: source is not larger than target"));
            }
        },
        CastOp::IntToPtr => {
            if not(llvmType_eq(&from_type, &LType::I64)) {
                parser_warning(parser, &string("inttoptr: source type must be i64"));
            }
            if not(llvmType_eq(&to_type, &LType::Ptr)) {
                parser_warning(parser, &string("inttoptr: target type must be ptr"));
            }
        },
        CastOp::PtrToInt => {
            if not(llvmType_eq(&from_type, &LType::Ptr)) {
                parser_warning(parser, &string("ptrtoint: source type must be ptr"));
            }
            if not(llvmType_eq(&to_type, &LType::I64)) {
                parser_warning(parser, &string("ptrtoint: target type must be i64"));
            }
        },
    }

    AssignOp::Cast(operator, to_type, value)
}

fn parser_parse_alloca_assign(parser: &mut Parser) -> AssignOp {
    let allocated_type: LType = parser_parse_type(parser);
    AssignOp::Alloca(allocated_type)
}

fn parser_parse_load_assign(parser: &mut Parser) -> AssignOp {
    let loaded_type: LType = parser_parse_type(parser);
    parser_expect_token(parser, &LToken::Comma);

    parser_expect_token(parser, &LToken::Ptr);
    let address: LValue = parser_parse_value(parser);
    parser_expect_value_type(parser, &address, &LType::Ptr);

    AssignOp::Load(loaded_type, address)
}

fn parser_parse_store(parser: &mut Parser) -> Instruction {
    parser_expect_token(parser, &LToken::Store);

    let store_type: LType = parser_parse_type(parser);
    let value: LValue = parser_parse_value(parser);
    parser_expect_value_type(parser, &value, &store_type);

    parser_expect_token(parser, &LToken::Comma);
    parser_expect_token(parser, &LToken::Ptr);

    let address: LValue = parser_parse_value(parser);
    parser_expect_value_type(parser, &address, &LType::Ptr);

    Instruction::Store(store_type, value, address)
}

fn parser_parse_call(parser: &mut Parser) -> Call {
    let return_type: LType = parser_parse_type(parser);
    let callee: String = parser_parse_global_name(parser);

    parser_expect_token(parser, &LToken::LParen);
    let mut arguments: Vec<LTypedValue> = vec_new::<LTypedValue>();
    if not(parser_current_token_eq(parser, &LToken::RParen)) {
        let arg_type: LType = parser_parse_type(parser);
        let arg_value: LValue = parser_parse_value(parser);
        parser_expect_value_type(parser, &arg_value, &arg_type);
        vec_push::<LTypedValue>(&mut arguments, LTypedValue::Pair(arg_type, arg_value));

        while parser_current_token_eq(parser, &LToken::Comma) {
            parser_next_token(parser);

            let arg_type: LType = parser_parse_type(parser);
            let arg_value: LValue = parser_parse_value(parser);
            parser_expect_value_type(parser, &arg_value, &arg_type);
            vec_push::<LTypedValue>(&mut arguments, LTypedValue::Pair(arg_type, arg_value));
        }
    }
    parser_expect_token(parser, &LToken::RParen);

    Call::Call(return_type, callee, arguments)
}

fn parser_parse_type(parser: &mut Parser) -> LType {
    match parser_consume_current_token(parser) {
        LToken::I1 => LType::I1,
        LToken::I8 => LType::I8,
        LToken::I64 => LType::I64,
        LToken::Void => LType::Void,
        LToken::Ptr => LType::Ptr,
        LToken::LBracket => {
            let len: usize = parser_parse_integer(parser);
            match parser_current_token(parser) {
                LToken::Identifier(separator) => {
                    if not(string_eq(separator, &string("x"))) {
                        let message: String =
                            parser_expected_message(parser, &string("x in LLVM array type"));
                        parser_error(parser, &message);
                    }
                    parser_next_token(parser);
                },
                _ => {
                    let message: String =
                        parser_expected_message(parser, &string("x in LLVM array type"));
                    parser_error(parser, &message)
                },
            }
            let inner: LType = parser_parse_type(parser);
            parser_expect_token(parser, &LToken::RBracket);
            LType::Array(len, box_new::<LType>(inner))
        },
        _ => {
            let message: String = parser_expected_message(parser, &string("LLVM type"));
            parser_error(parser, &message)
        },
    }
}

fn parser_parse_value(parser: &mut Parser) -> LValue {
    match parser_current_token(parser) {
        LToken::Percent => LValue::Register(parser_parse_register(parser)),
        LToken::At => LValue::Global(parser_parse_global_name(parser)),
        LToken::Integer(_) => LValue::Literal(parser_parse_integer(parser)),
        _ => {
            let message: String = parser_expected_message(parser, &string("LLVM value"));
            parser_error(parser, &message)
        },
    }
}

fn parser_parse_integer(parser: &mut Parser) -> usize {
    match parser_consume_current_token(parser) {
        LToken::Integer(value) => value,
        _ => {
            let message: String = parser_expected_message(parser, &string("LLVM integer literal"));
            parser_error(parser, &message)
        },
    }
}

// ------------------------- Interpreter -----------------------------

/// Execution control flow after one LLVM-IR instruction.
enum ExecFlow {
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
fn emu_load_globals(emulator: &mut Emu, ast: &LAst) {
    let mut data_pointer: usize = emu_get_gp(emulator);

    let mut i: usize = 0;
    while i < vec_len::<LGlobal>(lAst_globals(ast)) {
        let LGlobal::String(name, value): &LGlobal = vec_at::<LGlobal>(lAst_globals(ast), i);

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
    let ast: LAst = parser_parse_to_ast(source);

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
    ast: &LAst,
    function_name: &String,
    arguments: &Vec<usize>,
) -> usize {
    let function: &LFunction = lAst_lookup_function(ast, string_clone(function_name));
    emu_execute_function(emulator, ast, function, arguments)
}

/// Execute the given function's body.
fn emu_execute_function(
    emulator: &mut Emu,
    ast: &LAst,
    function: &LFunction,
    arguments: &Vec<usize>,
) -> usize {
    let previous_frame_size: usize = emu_get_frame_size(emulator);
    emu_set_frame_size(emulator, 0);

    match function {
        LFunction::BuiltIn(builtin, _, _) => {
            let value: usize = emu_execute_builtin(emulator, builtin, arguments);
            emu_set_frame_size(emulator, previous_frame_size);
            return value;
        },
        LFunction::Function(_, parameters, blocks) => {
            let mut virtual_registers: StringMap<usize> = stringMap_new::<usize>();

            let mut i: usize = 0;
            while i < vec_len::<LParameter>(parameters) {
                let parameter: &LParameter = vec_at::<LParameter>(parameters, i);
                let LParameter::Parameter(name, _): &LParameter = parameter;

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

                let flow: ExecFlow =
                    emu_execute_instructions(emulator, ast, &mut virtual_registers, instructions);

                match flow {
                    ExecFlow::Continue => panic("LLVM block did not terminate"),
                    ExecFlow::Jump(next_label) => current_label = next_label,
                    ExecFlow::Return(value) => {
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
fn emu_execute_builtin(emulator: &mut Emu, builtin: &BuiltIn, arguments: &Vec<usize>) -> usize {
    match builtin {
        BuiltIn::Malloc => {
            let value: usize = *vec_at::<usize>(arguments, 0);
            match emu_allocate_heap(emulator, value) {
                Option::Some(address) => address,
                Option::None => panic("heap overflow of emu"),
            }
        },
        BuiltIn::Exit => {
            let value: usize = *vec_at::<usize>(arguments, 0);
            emu_set_exit_code(emulator, value);
            value
        },
    }
}

/// Execute a given list of instructions.
fn emu_execute_instructions(
    emulator: &mut Emu,
    ast: &LAst,
    registers: &mut StringMap<usize>,
    instructions: &Vec<Instruction>,
) -> ExecFlow {
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
                return ExecFlow::Return(match return_value {
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
                        ExecFlow::Jump(string_clone(target_label))
                    },
                    Branch::Conditional(condition, then_label, else_label) => {
                        let condition_value: usize =
                            llvm_eval_value(emulator, registers, condition);

                        if condition_value == 1 {
                            ExecFlow::Jump(string_clone(then_label))
                        } else {
                            ExecFlow::Jump(string_clone(else_label))
                        }
                    },
                };
            },
        }

        match emu_exit_code(emulator) {
            Option::Some(code) => return ExecFlow::Return(code),
            Option::None => {},
        }

        i = i + 1;
    }
    ExecFlow::Continue
}

/// Execute the given assignment instruction.
fn emu_execute_assignment(
    emulator: &mut Emu,
    ast: &LAst,
    registers: &mut StringMap<usize>,
    AssignInstruction::Assign(target, operation): &AssignInstruction,
) {
    let value: usize = emu_evaluate_assign_op(emulator, ast, registers, operation);
    stringMap_insert::<usize>(registers, string_clone(target), value);
}

/// Evaluate the value of the assignment operation.
fn emu_evaluate_assign_op(
    emulator: &mut Emu,
    ast: &LAst,
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
                CastOp::Trunc => llvm_overflow_value(evaluated_value, to_type),
                CastOp::Zext | CastOp::IntToPtr | CastOp::PtrToInt => evaluated_value,
            }
        },
        AssignOp::Alloca(allocated_type) => {
            let space: usize = llvmType_size(allocated_type);
            match emu_allocate_stack(emulator, space) {
                Option::Some(address) => address,
                Option::None => panic("Stack overflow of emu"),
            }
        },
        AssignOp::Load(loaded_type, address_value) => {
            let address: usize = llvm_eval_value(emulator, registers, address_value);
            match emu_load_bytes(emulator, address, llvmType_size(loaded_type)) {
                Option::Some(value) => llvm_overflow_value(value, loaded_type),
                Option::None => panic("invalid LLVM load address"),
            }
        },
        AssignOp::Call(Call::Call(call_type, callee, arguments)) => {
            emu_execute_call(emulator, ast, registers, call_type, callee, arguments)
        },
    }
}

/// Execute an LLVM call and return the raw result value.
fn emu_execute_call(
    emulator: &mut Emu,
    ast: &LAst,
    registers: &StringMap<usize>,
    call_type: &LType,
    callee: &String,
    arguments: &Vec<LTypedValue>,
) -> usize {
    let mut arg_values: Vec<usize> = vec_new::<usize>();
    let mut i: usize = 0;
    while i < vec_len::<LTypedValue>(arguments) {
        let argument: &LTypedValue = vec_at::<LTypedValue>(arguments, i);
        let LTypedValue::Pair(ty, argument_value): &LTypedValue = argument;

        let value: usize = llvm_eval_value(emulator, registers, argument_value);
        let wrapped_value: usize = llvm_overflow_value(value, ty);
        vec_push::<usize>(&mut arg_values, wrapped_value);

        i = i + 1;
    }

    let value: usize = emu_execute_function_named(emulator, ast, callee, &arg_values);
    llvm_overflow_value(value, call_type)
}

/// Normalize a value so it wraps around according to the given type.
fn llvm_overflow_value(value: usize, ty: &LType) -> usize {
    match ty {
        LType::I1 => value % 2,
        LType::I8 => value % 256,
        _ => value,
    }
}

/// Execute the given store instruction.
fn emu_execute_store(
    emulator: &mut Emu,
    registers: &StringMap<usize>,
    store_type: &LType,
    value: &LValue,
    address: &LValue,
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
        panic("invalid LLVM store address");
    }
}

/// Evaluate the value of a virtual register, global name or literal.
fn llvm_eval_value(emulator: &Emu, registers: &StringMap<usize>, value: &LValue) -> usize {
    match value {
        LValue::Literal(number) => *number,
        LValue::Register(name) => match stringMap_get::<usize>(registers, name) {
            Option::Some(register_value) => *register_value,
            Option::None => panic("unknown LLVM register"),
        },
        LValue::Global(name) => match stringMap_get::<usize>(emu_globals(emulator), name) {
            Option::Some(value) => *value,
            Option::None => panic("unknown LLVM global value"),
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

/// Panic by printing a message and exiting the program.
fn panic(message: &str) -> ! {
    eprint_str(message);
    eprint_str("\n");
    exit_process(1);
}

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
    eprint_str("\n");

    let mut i: usize = 1;
    while i < col {
        eprint_str(" ");
        i = i + 1;
    }
    eprint_str("^ ");
    eprint_string(message);
    eprint_str("\n");

    exit_process(1);
}

/// Report a warning message with source location and continue.
fn report_warning(file: &SourceFile, message: &String) {
    let line: usize = sourceFile_current_line(file);
    let col: usize = sourceFile_current_column(file);

    let mut header: String = string("WARNING at ");
    let line_text: String = integer_to_string(line);
    let col_text: String = integer_to_string(col);
    string_push_string(&mut header, &line_text);
    string_push(&mut header, ':');
    string_push_string(&mut header, &col_text);
    string_push_str(&mut header, ":\n");
    eprint_string(&header);

    let mut start: usize = sourceFile_current_line_start(file);
    let mut reached_end: bool = false;
    let mut line_content: String = string_new();
    while not(reached_end) {
        match sourceFile_get_char(file, start) {
            Option::Some('\n') => reached_end = true,
            Option::Some(c) => string_push(&mut line_content, c),
            Option::None => reached_end = true,
        }
        start = start + 1;
    }
    eprint_string(&line_content);
    eprint_str("\n");

    let mut i: usize = 1;
    while i < col {
        eprint_str(" ");
        i = i + 1;
    }
    eprint_str("^ ");
    eprint_string(message);
    eprint_str("\n");
}

fn lexer_error(lexer: &RLexer, message: &String) -> ! {
    report_error(rLexer_sourcefile(lexer), message)
}

/// Emit an error at the parser current location and abort.
fn parse_error(lexer: &RLexer, message: &String) -> ! {
    lexer_error(lexer, message)
}

fn codegen_error(message: &str) -> ! {
    panic!("Codegeneration error: {}", message)
}

fn semantic_check_error(message: &str) -> ! {
    panic!("Semantic error: {}", message);
}

/// Emit an LLVM parser error and panic.
fn parser_error(parser: &Parser, message: &String) -> ! {
    let file: &SourceFile = lLexer_sourcefile(parser_lexer(parser));
    report_error(file, message)
}

/// Emit an LLVM parser warning and continue.
fn parser_warning(parser: &Parser, message: &String) {
    let file: &SourceFile = lLexer_sourcefile(parser_lexer(parser));
    report_warning(file, message)
}

fn parser_expected_message(parser: &Parser, expected: &String) -> String {
    let mut message: String = string("expected ");
    string_push_string(&mut message, expected);
    let token: &LToken = parser_current_token(parser);
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
        Option::None => panic("tried to unwrap None variant of Option<T>"),
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

        let new_ptr: *mut T = alloc::<T>(*capacity_ref);
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
    if index >= vec_len::<T>(vec) {
        panic("Out-of-bounds vector access!");
    }
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

fn llvmType_eq(left: &LType, right: &LType) -> bool {
    match left {
        LType::I1 => match right {
            LType::I1 => true,
            _ => false,
        },
        LType::I8 => match right {
            LType::I8 => true,
            _ => false,
        },
        LType::I64 => match right {
            LType::I64 => true,
            _ => false,
        },
        LType::Ptr => match right {
            LType::Ptr => true,
            _ => false,
        },
        LType::Array(left_len, left_inner) => match right {
            LType::Array(right_len, right_inner) => {
                *left_len == *right_len
                    && llvmType_eq(
                        box_deref::<LType>(left_inner),
                        box_deref::<LType>(right_inner),
                    )
            },
            _ => false,
        },
        LType::Void => match right {
            LType::Void => true,
            _ => false,
        },
    }
}

/// Check if two tokens are equal.
fn token_eq(a: &RToken, b: &RToken) -> bool {
    match a {
        RToken::Unsafe => match b {
            RToken::Unsafe => true,
            _ => false,
        },
        RToken::Fn => match b {
            RToken::Fn => true,
            _ => false,
        },
        RToken::Enum => match b {
            RToken::Enum => true,
            _ => false,
        },
        RToken::Extern => match b {
            RToken::Extern => true,
            _ => false,
        },
        RToken::Let => match b {
            RToken::Let => true,
            _ => false,
        },
        RToken::If => match b {
            RToken::If => true,
            _ => false,
        },
        RToken::Else => match b {
            RToken::Else => true,
            _ => false,
        },
        RToken::While => match b {
            RToken::While => true,
            _ => false,
        },
        RToken::Return => match b {
            RToken::Return => true,
            _ => false,
        },
        RToken::Match => match b {
            RToken::Match => true,
            _ => false,
        },
        RToken::As => match b {
            RToken::As => true,
            _ => false,
        },
        RToken::Mut => match b {
            RToken::Mut => true,
            _ => false,
        },
        RToken::Pipe => match b {
            RToken::Pipe => true,
            _ => false,
        },
        RToken::Ampersand => match b {
            RToken::Ampersand => true,
            _ => false,
        },
        RToken::LBrace => match b {
            RToken::LBrace => true,
            _ => false,
        },
        RToken::RBrace => match b {
            RToken::RBrace => true,
            _ => false,
        },
        RToken::LParen => match b {
            RToken::LParen => true,
            _ => false,
        },
        RToken::RParen => match b {
            RToken::RParen => true,
            _ => false,
        },
        RToken::Colon => match b {
            RToken::Colon => true,
            _ => false,
        },
        RToken::DoubleColon => match b {
            RToken::DoubleColon => true,
            _ => false,
        },
        RToken::SemiColon => match b {
            RToken::SemiColon => true,
            _ => false,
        },
        RToken::Comma => match b {
            RToken::Comma => true,
            _ => false,
        },
        RToken::Assign => match b {
            RToken::Assign => true,
            _ => false,
        },
        RToken::Bang => match b {
            RToken::Bang => true,
            _ => false,
        },
        RToken::Cmp(left_comparison) => match b {
            RToken::Cmp(right_comparison) => comparison_eq(left_comparison, right_comparison),
            _ => false,
        },
        RToken::FatArrow => match b {
            RToken::FatArrow => true,
            _ => false,
        },
        RToken::Plus => match b {
            RToken::Plus => true,
            _ => false,
        },
        RToken::Minus => match b {
            RToken::Minus => true,
            _ => false,
        },
        RToken::Star => match b {
            RToken::Star => true,
            _ => false,
        },
        RToken::Slash => match b {
            RToken::Slash => true,
            _ => false,
        },
        RToken::Remainder => match b {
            RToken::Remainder => true,
            _ => false,
        },
        RToken::Usize => match b {
            RToken::Usize => true,
            _ => false,
        },
        RToken::U8 => match b {
            RToken::U8 => true,
            _ => false,
        },
        RToken::Bool => match b {
            RToken::Bool => true,
            _ => false,
        },
        RToken::Char => match b {
            RToken::Char => true,
            _ => false,
        },
        RToken::Str => match b {
            RToken::Str => true,
            _ => false,
        },
        RToken::Arrow => match b {
            RToken::Arrow => true,
            _ => false,
        },
        RToken::Literal(left_literal) => match b {
            RToken::Literal(right_literal) => rLiteral_eq(left_literal, right_literal),
            _ => false,
        },
        RToken::Identifier(left) => match b {
            RToken::Identifier(right) => string_eq(left, right),
            _ => false,
        },
        RToken::Eof => match b {
            RToken::Eof => true,
            _ => false,
        },
    }
}

/// Check if two comparison tokens are equal.
fn comparison_eq(left: &RComparisonOp, right: &RComparisonOp) -> bool {
    match left {
        RComparisonOp::Eq => match right {
            RComparisonOp::Eq => true,
            _ => false,
        },
        RComparisonOp::Ne => match right {
            RComparisonOp::Ne => true,
            _ => false,
        },
        RComparisonOp::Gt => match right {
            RComparisonOp::Gt => true,
            _ => false,
        },
        RComparisonOp::Lt => match right {
            RComparisonOp::Lt => true,
            _ => false,
        },
        RComparisonOp::Geq => match right {
            RComparisonOp::Geq => true,
            _ => false,
        },
        RComparisonOp::Leq => match right {
            RComparisonOp::Leq => true,
            _ => false,
        },
    }
}

/// Check if two literal tokens are equal.
fn rLiteral_eq(left: &RLiteral, right: &RLiteral) -> bool {
    match left {
        RLiteral::Int(left_value) => match right {
            RLiteral::Int(right_value) => left_value == right_value,
            _ => false,
        },
        RLiteral::String(left_value) => match right {
            RLiteral::String(right_value) => string_eq(left_value, right_value),
            _ => false,
        },
        RLiteral::Char(left_value) => match right {
            RLiteral::Char(right_value) => left_value == right_value,
            _ => false,
        },
        RLiteral::Bool(left_value) => match right {
            RLiteral::Bool(right_value) => left_value == right_value,
            _ => false,
        },
    }
}

/// Check two Rust AST types for equality.
fn rType_eq(a: &RType, b: &RType) -> bool {
    match a {
        RType::U8 => match b {
            RType::U8 => true,
            _ => false,
        },
        RType::Usize => match b {
            RType::Usize => true,
            _ => false,
        },
        RType::Bool => match b {
            RType::Bool => true,
            _ => false,
        },
        RType::Char => match b {
            RType::Char => true,
            _ => false,
        },
        RType::Unit => match b {
            RType::Unit => true,
            _ => false,
        },
        RType::Never => match b {
            RType::Never => true,
            _ => false,
        },
        RType::Custom(left) => match b {
            RType::Custom(right) => string_eq(left, right),
            _ => false,
        },
        RType::Reference(left, left_mut) => match b {
            RType::Reference(right, right_mut) => and(
                *left_mut == *right_mut,
                rType_eq(box_deref::<RType>(left), box_deref::<RType>(right)),
            ),
            _ => false,
        },
        RType::RawPointerMut(left) => match b {
            RType::RawPointerMut(right) => {
                rType_eq(box_deref::<RType>(left), box_deref::<RType>(right))
            },
            _ => false,
        },
    }
}

/// Check two LLVM tokens for equality.
fn llvmToken_eq(left: &LToken, right: &LToken) -> bool {
    match left {
        LToken::Define => match right {
            LToken::Define => true,
            _ => false,
        },
        LToken::Declare => match right {
            LToken::Declare => true,
            _ => false,
        },
        LToken::Ret => match right {
            LToken::Ret => true,
            _ => false,
        },
        LToken::IntToPtr => match right {
            LToken::IntToPtr => true,
            _ => false,
        },
        LToken::PtrToInt => match right {
            LToken::PtrToInt => true,
            _ => false,
        },
        LToken::Br => match right {
            LToken::Br => true,
            _ => false,
        },
        LToken::Label => match right {
            LToken::Label => true,
            _ => false,
        },
        LToken::Add => match right {
            LToken::Add => true,
            _ => false,
        },
        LToken::Sub => match right {
            LToken::Sub => true,
            _ => false,
        },
        LToken::Mul => match right {
            LToken::Mul => true,
            _ => false,
        },
        LToken::Udiv => match right {
            LToken::Udiv => true,
            _ => false,
        },
        LToken::Urem => match right {
            LToken::Urem => true,
            _ => false,
        },
        LToken::Icmp => match right {
            LToken::Icmp => true,
            _ => false,
        },
        LToken::Zext => match right {
            LToken::Zext => true,
            _ => false,
        },
        LToken::Trunc => match right {
            LToken::Trunc => true,
            _ => false,
        },
        LToken::Alloca => match right {
            LToken::Alloca => true,
            _ => false,
        },
        LToken::Store => match right {
            LToken::Store => true,
            _ => false,
        },
        LToken::Load => match right {
            LToken::Load => true,
            _ => false,
        },
        LToken::To => match right {
            LToken::To => true,
            _ => false,
        },
        LToken::Call => match right {
            LToken::Call => true,
            _ => false,
        },
        LToken::Constant => match right {
            LToken::Constant => true,
            _ => false,
        },
        LToken::Eq => match right {
            LToken::Eq => true,
            _ => false,
        },
        LToken::Ne => match right {
            LToken::Ne => true,
            _ => false,
        },
        LToken::Ugt => match right {
            LToken::Ugt => true,
            _ => false,
        },
        LToken::Uge => match right {
            LToken::Uge => true,
            _ => false,
        },
        LToken::Ult => match right {
            LToken::Ult => true,
            _ => false,
        },
        LToken::Ule => match right {
            LToken::Ule => true,
            _ => false,
        },
        LToken::Ptr => match right {
            LToken::Ptr => true,
            _ => false,
        },
        LToken::I64 => match right {
            LToken::I64 => true,
            _ => false,
        },
        LToken::I8 => match right {
            LToken::I8 => true,
            _ => false,
        },
        LToken::I1 => match right {
            LToken::I1 => true,
            _ => false,
        },
        LToken::Void => match right {
            LToken::Void => true,
            _ => false,
        },
        LToken::At => match right {
            LToken::At => true,
            _ => false,
        },
        LToken::Percent => match right {
            LToken::Percent => true,
            _ => false,
        },
        LToken::LParen => match right {
            LToken::LParen => true,
            _ => false,
        },
        LToken::RParen => match right {
            LToken::RParen => true,
            _ => false,
        },
        LToken::LBrace => match right {
            LToken::LBrace => true,
            _ => false,
        },
        LToken::RBrace => match right {
            LToken::RBrace => true,
            _ => false,
        },
        LToken::LBracket => match right {
            LToken::LBracket => true,
            _ => false,
        },
        LToken::RBracket => match right {
            LToken::RBracket => true,
            _ => false,
        },
        LToken::Comma => match right {
            LToken::Comma => true,
            _ => false,
        },
        LToken::Assign => match right {
            LToken::Assign => true,
            _ => false,
        },
        LToken::Colon => match right {
            LToken::Colon => true,
            _ => false,
        },
        LToken::CString(left_value) => match right {
            LToken::CString(right_value) => string_eq(left_value, right_value),
            _ => false,
        },
        LToken::Identifier(left_name) => match right {
            LToken::Identifier(right_name) => string_eq(left_name, right_name),
            _ => false,
        },
        LToken::Integer(left_value) => match right {
            LToken::Integer(right_value) => *left_value == *right_value,
            _ => false,
        },
        LToken::Eof => match right {
            LToken::Eof => true,
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
fn token_clone(token: &RToken) -> RToken {
    match token {
        RToken::Unsafe => RToken::Unsafe,
        RToken::Fn => RToken::Fn,
        RToken::Enum => RToken::Enum,
        RToken::Extern => RToken::Extern,
        RToken::Let => RToken::Let,
        RToken::If => RToken::If,
        RToken::Else => RToken::Else,
        RToken::While => RToken::While,
        RToken::Return => RToken::Return,
        RToken::Match => RToken::Match,
        RToken::As => RToken::As,
        RToken::Mut => RToken::Mut,
        RToken::Ampersand => RToken::Ampersand,
        RToken::LBrace => RToken::LBrace,
        RToken::RBrace => RToken::RBrace,
        RToken::LParen => RToken::LParen,
        RToken::RParen => RToken::RParen,
        RToken::Colon => RToken::Colon,
        RToken::DoubleColon => RToken::DoubleColon,
        RToken::SemiColon => RToken::SemiColon,
        RToken::Comma => RToken::Comma,
        RToken::Pipe => RToken::Pipe,
        RToken::Assign => RToken::Assign,
        RToken::Bang => RToken::Bang,
        RToken::Cmp(comparison) => RToken::Cmp(comparison_clone(comparison)),
        RToken::FatArrow => RToken::FatArrow,
        RToken::Plus => RToken::Plus,
        RToken::Minus => RToken::Minus,
        RToken::Star => RToken::Star,
        RToken::Slash => RToken::Slash,
        RToken::Remainder => RToken::Remainder,
        RToken::Usize => RToken::Usize,
        RToken::U8 => RToken::U8,
        RToken::Bool => RToken::Bool,
        RToken::Char => RToken::Char,
        RToken::Str => RToken::Str,
        RToken::Arrow => RToken::Arrow,
        RToken::Literal(literal) => RToken::Literal(rLiteral_clone(literal)),
        RToken::Identifier(value) => RToken::Identifier(string_clone(value)),
        RToken::Eof => RToken::Eof,
    }
}

/// Clone a comparison operator.
fn comparison_clone(comparison: &RComparisonOp) -> RComparisonOp {
    match comparison {
        RComparisonOp::Eq => RComparisonOp::Eq,
        RComparisonOp::Ne => RComparisonOp::Ne,
        RComparisonOp::Gt => RComparisonOp::Gt,
        RComparisonOp::Lt => RComparisonOp::Lt,
        RComparisonOp::Geq => RComparisonOp::Geq,
        RComparisonOp::Leq => RComparisonOp::Leq,
    }
}

/// Clone a literal token payload.
fn rLiteral_clone(literal: &RLiteral) -> RLiteral {
    match literal {
        RLiteral::Int(value) => RLiteral::Int(*value),
        RLiteral::String(value) => RLiteral::String(string_clone(value)),
        RLiteral::Char(value) => RLiteral::Char(*value),
        RLiteral::Bool(value) => RLiteral::Bool(*value),
    }
}

/// Clone a function signature.
fn fnSignature_clone(signature: &FnSignature) -> FnSignature {
    match signature {
        FnSignature::Fn(parameter_types, return_type, is_unsafe) => {
            let mut cloned_params: Vec<RType> = vec_new::<RType>();
            let mut i: usize = 0;
            while i < vec_len::<RType>(parameter_types) {
                let param: &RType = vec_at::<RType>(parameter_types, i);
                vec_push::<RType>(&mut cloned_params, rType_clone(param));
                i = i + 1;
            }
            FnSignature::Fn(cloned_params, rType_clone(return_type), *is_unsafe)
        },
    }
}

/// Clone a Rust AST type value.
fn rType_clone(t: &RType) -> RType {
    match t {
        RType::U8 => RType::U8,
        RType::Usize => RType::Usize,
        RType::Bool => RType::Bool,
        RType::Char => RType::Char,
        RType::Unit => RType::Unit,
        RType::Never => RType::Never,
        RType::Custom(name) => RType::Custom(string_clone(name)),
        RType::Reference(inner, mutable) => RType::Reference(
            box_new::<RType>(rType_clone(box_deref::<RType>(inner))),
            *mutable,
        ),
        RType::RawPointerMut(inner) => {
            RType::RawPointerMut(box_new::<RType>(rType_clone(box_deref::<RType>(inner))))
        },
    }
}

/// Clone a STPair
fn stPair_clone(STPair::ST(string, ty): &STPair) -> STPair {
    STPair::ST(string_clone(string), rType_clone(ty))
}

/// Clone an LLVM token.
fn llvmToken_clone(token: &LToken) -> LToken {
    match token {
        LToken::Define => LToken::Define,
        LToken::Declare => LToken::Declare,
        LToken::Ret => LToken::Ret,
        LToken::IntToPtr => LToken::IntToPtr,
        LToken::PtrToInt => LToken::PtrToInt,
        LToken::Br => LToken::Br,
        LToken::Label => LToken::Label,
        LToken::Add => LToken::Add,
        LToken::Sub => LToken::Sub,
        LToken::Mul => LToken::Mul,
        LToken::Udiv => LToken::Udiv,
        LToken::Urem => LToken::Urem,
        LToken::Icmp => LToken::Icmp,
        LToken::Zext => LToken::Zext,
        LToken::Trunc => LToken::Trunc,
        LToken::Alloca => LToken::Alloca,
        LToken::Store => LToken::Store,
        LToken::Load => LToken::Load,
        LToken::To => LToken::To,
        LToken::Call => LToken::Call,
        LToken::Constant => LToken::Constant,
        LToken::Eq => LToken::Eq,
        LToken::Ne => LToken::Ne,
        LToken::Ugt => LToken::Ugt,
        LToken::Uge => LToken::Uge,
        LToken::Ult => LToken::Ult,
        LToken::Ule => LToken::Ule,
        LToken::Ptr => LToken::Ptr,
        LToken::I64 => LToken::I64,
        LToken::I8 => LToken::I8,
        LToken::I1 => LToken::I1,
        LToken::Void => LToken::Void,
        LToken::At => LToken::At,
        LToken::Percent => LToken::Percent,
        LToken::LParen => LToken::LParen,
        LToken::RParen => LToken::RParen,
        LToken::LBrace => LToken::LBrace,
        LToken::RBrace => LToken::RBrace,
        LToken::LBracket => LToken::LBracket,
        LToken::RBracket => LToken::RBracket,
        LToken::Comma => LToken::Comma,
        LToken::Assign => LToken::Assign,
        LToken::Colon => LToken::Colon,
        LToken::CString(value) => LToken::CString(string_clone(value)),
        LToken::Identifier(name) => LToken::Identifier(string_clone(name)),
        LToken::Integer(value) => LToken::Integer(*value),
        LToken::Eof => LToken::Eof,
    }
}

/// Clone an LLVM type.
fn llvmType_clone(ty: &LType) -> LType {
    match ty {
        LType::I1 => LType::I1,
        LType::I8 => LType::I8,
        LType::I64 => LType::I64,
        LType::Ptr => LType::Ptr,
        LType::Array(len, inner) => LType::Array(
            *len,
            box_new::<LType>(llvmType_clone(box_deref::<LType>(inner))),
        ),
        LType::Void => LType::Void,
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
fn token_to_string(token: &RToken) -> String {
    match token {
        RToken::Fn => string("fn"),
        RToken::Enum => string("enum"),
        RToken::Extern => string("extern"),
        RToken::Let => string("let"),
        RToken::If => string("if"),
        RToken::Else => string("else"),
        RToken::While => string("while"),
        RToken::Return => string("return"),
        RToken::Match => string("match"),
        RToken::As => string("as"),
        RToken::Unsafe => string("unsafe"),
        RToken::Mut => string("mut"),
        RToken::Ampersand => string("&"),
        RToken::LBrace => string("{"),
        RToken::RBrace => string("}"),
        RToken::LParen => string("("),
        RToken::RParen => string(")"),
        RToken::Colon => string(":"),
        RToken::DoubleColon => string("::"),
        RToken::SemiColon => string(";"),
        RToken::Comma => string(","),
        RToken::Pipe => string("|"),
        RToken::Assign => string("="),
        RToken::Bang => string("!"),
        RToken::Cmp(comparison) => comparison_to_string(comparison),
        RToken::FatArrow => string("=>"),
        RToken::Plus => string("+"),
        RToken::Minus => string("-"),
        RToken::Star => string("*"),
        RToken::Slash => string("/"),
        RToken::Remainder => string("%"),
        RToken::Usize => string("usize"),
        RToken::U8 => string("u8"),
        RToken::Bool => string("bool"),
        RToken::Char => string("char"),
        RToken::Str => string("str"),
        RToken::Arrow => string("->"),
        RToken::Literal(literal) => rLiteral_to_string(literal),
        RToken::Identifier(name) => string_clone(name),
        RToken::Eof => string("<eof>"),
    }
}

/// Convert an LLVM token into a string.
fn llvmToken_to_string(token: &LToken) -> String {
    match token {
        LToken::Define => string("define"),
        LToken::Declare => string("declare"),
        LToken::Ret => string("ret"),
        LToken::IntToPtr => string("inttoptr"),
        LToken::PtrToInt => string("ptrtoint"),
        LToken::Br => string("br"),
        LToken::Label => string("label"),
        LToken::Add => string("add"),
        LToken::Sub => string("sub"),
        LToken::Mul => string("mul"),
        LToken::Udiv => string("udiv"),
        LToken::Urem => string("urem"),
        LToken::Icmp => string("icmp"),
        LToken::Zext => string("zext"),
        LToken::Trunc => string("trunc"),
        LToken::Alloca => string("alloca"),
        LToken::Store => string("store"),
        LToken::Load => string("load"),
        LToken::To => string("to"),
        LToken::Call => string("call"),
        LToken::Constant => string("constant"),
        LToken::Eq => string("eq"),
        LToken::Ne => string("ne"),
        LToken::Ugt => string("ugt"),
        LToken::Uge => string("uge"),
        LToken::Ult => string("ult"),
        LToken::Ule => string("ule"),
        LToken::Ptr => string("ptr"),
        LToken::I64 => string("i64"),
        LToken::I8 => string("i8"),
        LToken::I1 => string("i1"),
        LToken::Void => string("void"),
        LToken::At => string("@"),
        LToken::Percent => string("%"),
        LToken::LParen => string("("),
        LToken::RParen => string(")"),
        LToken::LBrace => string("{"),
        LToken::RBrace => string("}"),
        LToken::LBracket => string("["),
        LToken::RBracket => string("]"),
        LToken::Comma => string(","),
        LToken::Assign => string("="),
        LToken::Colon => string(":"),
        LToken::CString(value) => {
            let mut string: String = string_new();
            string_push_str(&mut string, "c\"");
            string_push_string(&mut string, value);
            string_push(&mut string, '"');
            string
        },
        LToken::Identifier(name) => string_clone(name),
        LToken::Integer(value) => integer_to_string(*value),
        LToken::Eof => string("<eof>"),
    }
}

/// Convert a comparison token into a string.
fn comparison_to_string(comparison: &RComparisonOp) -> String {
    match comparison {
        RComparisonOp::Eq => string("=="),
        RComparisonOp::Ne => string("!="),
        RComparisonOp::Gt => string(">"),
        RComparisonOp::Lt => string("<"),
        RComparisonOp::Geq => string(">="),
        RComparisonOp::Leq => string("<="),
    }
}

/// Convert a literal token into a string.
fn rLiteral_to_string(literal: &RLiteral) -> String {
    match literal {
        RLiteral::Int(value) => integer_to_string(*value),
        RLiteral::Bool(value) => {
            if *value {
                string("true")
            } else {
                string("false")
            }
        },
        RLiteral::Char(value) => {
            let mut string: String = string_new();
            string_push(&mut string, '\'');
            string_push(&mut string, *value);
            string_push(&mut string, '\'');
            string
        },
        RLiteral::String(value) => {
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
            eprint_str("Heap Memory Allocation Error!\n");
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
