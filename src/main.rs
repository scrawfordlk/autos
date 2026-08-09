#![allow(
    clippy::assign_op_pattern,
    clippy::match_like_matches_macro,
    clippy::single_match,
    clippy::collapsible_match,
    clippy::too_many_arguments,
    clippy::unnecessary_cast,
    clippy::manual_bits,
    clippy::upper_case_acronyms,
    clippy::manual_is_multiple_of,
    clippy::char_lit_as_u8,
    clippy::zero_ptr,
    while_true,
    non_snake_case,
    unused_assignments
)] // attributes such as these are ignored by autos
#![no_main]
#[unsafe(no_mangle)]
fn main(argc: usize, argv: *mut *mut u8) {
    let args: Args = args_new(argc, argv);
    if arg_eq(&args, 1, "-c") {
        if args_len(&args) <= 2 {
            print_help_exit()
        }
        let input_name: &String = args_at(&args, 2);
        let source: String = read_file(string_clone(input_name));

        let mut do_emulate: bool = false;
        let mut emulator_memory_size: usize = 0;
        let mut do_semantic_analysis: bool = true;
        let mut output_file: Option<String> = Option::<String>::None;

        let mut i: usize = 3;
        if arg_eq(&args, i, "-o") {
            if i + 1 >= args_len(&args) {
                print_help_exit();
            }
            output_file = Option::<String>::Some(string_clone(args_at(&args, i + 1)));
            i = i + 2;
        }
        if arg_eq(&args, i, "--unsafe") {
            do_semantic_analysis = false;
            i = i + 1;
        }
        if arg_eq(&args, i, "-e") {
            if i + 1 >= args_len(&args) {
                print_help_exit();
            }
            do_emulate = true;
            emulator_memory_size = parse_memory_size_mb(args_at(&args, i + 1));
            i = i + 2;
        }

        let output_code: String = compile(source, do_semantic_analysis);
        let output_name: String = match output_file {
            Option::Some(name) => name,
            _ => path_to_file_with_ending(input_name, "ll"),
        };
        write_file(output_name, &output_code);
        if do_emulate {
            let args_rest: Args = args_subargs(&args, i);
            let exit_code: usize = emulate(output_code, emulator_memory_size, &args_rest);
            exit_process(exit_code);
        }
        exit_process(0);
    }
    if arg_eq(&args, 1, "-e") {
        if args_len(&args) <= 3 {
            print_help_exit()
        }
        let memory_size: usize = parse_memory_size_mb(args_at(&args, 2));
        let input_name: &String = args_at(&args, 3);
        let llvm: String = read_file(string_clone(input_name));
        let args_rest: Args = args_subargs(&args, 4);
        let exit_code: usize = emulate(llvm, memory_size, &args_rest);
        exit_process(exit_code);
    }
    print_help_exit()
}

fn print_help_exit() -> ! {
    print_str(
        "Usage: autos ( -c <input> [ -o <output> ] [ --unsafe ] [ -e <mb> ... ] | -e <mb> <input> ... )",
    );
    println();
    exit_process(1);
}

fn parse_memory_size_mb(value: &String) -> usize {
    match string_to_integer(value, 10) {
        Option::Some(memory_mb) => {
            if memory_mb == 0 {
                print_help_exit()
            } else {
                memory_mb * 1000000
            }
        },
        Option::None => print_help_exit(),
    }
}

// -----------------------------------------------------------------
// -----------------------------------------------------------------
// --------------------- RawRust Compiler --------------------------
// -----------------------------------------------------------------
// -----------------------------------------------------------------

/// Compile source code into LLVM-IR.
fn compile(source: String, do_semantic_analysis: bool) -> String {
    print_str("[Starting Compilation]");
    println();
    let mut lexer: RLexer = rLexer_new(source);
    let ast: RAst = parse_language(&mut lexer);
    print_str("=> Completed Parsing\n");

    let items: StringMap<Item> = collect_items(&ast);
    if do_semantic_analysis {
        semantic_check_run(&ast, &items);
        print_str("=> Completed Semantic Analysis\n");
    }

    let mut codegen: Codegen = codegen_new();
    let icg: ICodegen = iCodegenStatic_new(ast, items);
    codegen_language(&mut codegen, &icg);
    print_str("=> Completed Code Generation\n");

    codegen_into_llvm(codegen)
}

// -----------------------------------------------------------------
// ---------------------- Lexical Analysis -------------------------
// -----------------------------------------------------------------

enum RToken {
    Fn,          // "fn"
    Enum,        // "enum"
    Extern,      // "extern"
    Let,         // "let"
    If,          // "if"
    Else,        // "else"
    While,       // "while"
    Return,      // "return"
    Match,       // "match"
    As,          // "as"
    Unsafe,      // "unsafe"
    Mut,         // "mut"
    Ampersand,   // "&"
    LBrace,      // "{"
    RBrace,      // "}"
    LParen,      // "("
    RParen,      // ")"
    Colon,       // ":"
    DoubleColon, // "::"
    SemiColon,   // ";"
    Comma,       // ","
    Pipe,        // "|"
    Assign,      // "="
    Bang,        // "!"
    Eq,          // "=="
    Neq,         // "!="
    LAngle,      // "<"
    RAngle,      // ">"
    Leq,         // "<="
    Geq,         // ">="
    FatArrow,    // "=>"
    Plus,        // "+"
    Minus,       // "-"
    Star,        // "*"
    Slash,       // "/"
    Remainder,   // "%"
    Usize,       // "usize"
    U8,          // "u8"
    Bool,        // "bool"
    Char,        // "char"
    Arrow,       // "->"
    Lifetime,    // "'.."
    Literal(RLiteral),
    Identifier(String),
    Eof,
}

/// Literal tokens.
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
    rToken_eq(rLexer_current_token(lexer), token)
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
        _ => {},
    };
    current
}

/// Peek at the next character and consume/return true if it matches the given character.
fn rLexer_try_consume_char(lexer: &mut RLexer, expected: char) -> bool {
    match rLexer_peek_char(lexer) {
        Option::Some(c) => {
            if c == expected {
                rLexer_consume_char(lexer);
                true
            } else {
                false
            }
        },
        _ => false,
    }
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
                rLexer_error(lexer, &message);
            }
        },
        _ => rLexer_error(lexer, &string("unexpected end of input")),
    }
}

// ---------------------- Lexer ----------------------

/// Consume and return the next token.
fn rLexer_next_token(lexer: &mut RLexer) -> RToken {
    rLexer_skip_whitespace_and_attributes(lexer);

    let token: RToken = match rLexer_peek_char(lexer) {
        Option::Some(c) => {
            if is_alpha(c) {
                let ident: String = rLexer_scan_identifier(lexer);
                rust_identifier_to_token(ident)
            } else if is_digit(c) {
                let value: usize = rLexer_scan_integer(lexer);
                RToken::Literal(RLiteral::Int(value))
            } else if c == '\'' {
                rLexer_scan_char_literal_or_lifetime(lexer)
            } else if c == '"' {
                let s: String = rLexer_scan_string_literal(lexer);
                RToken::Literal(RLiteral::String(s))
            } else {
                rLexer_scan_symbol(lexer)
            }
        },
        _ => RToken::Eof,
    };

    rLexer_set_current_token(lexer, rToken_clone(&token));
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
            _ => return ident,
        }
    }
    unreachable()
}

/// Convert an identifier to a keyword token if applicable.
fn rust_identifier_to_token(ident: String) -> RToken {
    if str_eq(&ident, "fn") {
        RToken::Fn
    } else if str_eq(&ident, "enum") {
        RToken::Enum
    } else if str_eq(&ident, "extern") {
        RToken::Extern
    } else if str_eq(&ident, "let") {
        RToken::Let
    } else if str_eq(&ident, "if") {
        RToken::If
    } else if str_eq(&ident, "else") {
        RToken::Else
    } else if str_eq(&ident, "while") {
        RToken::While
    } else if str_eq(&ident, "return") {
        RToken::Return
    } else if str_eq(&ident, "match") {
        RToken::Match
    } else if str_eq(&ident, "as") {
        RToken::As
    } else if str_eq(&ident, "unsafe") {
        RToken::Unsafe
    } else if str_eq(&ident, "mut") {
        RToken::Mut
    } else if str_eq(&ident, "usize") {
        RToken::Usize
    } else if str_eq(&ident, "u8") {
        RToken::U8
    } else if str_eq(&ident, "bool") {
        RToken::Bool
    } else if str_eq(&ident, "char") {
        RToken::Char
    } else if str_eq(&ident, "true") {
        RToken::Literal(RLiteral::Bool(true))
    } else if str_eq(&ident, "false") {
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
            _ => done = true,
        }
    }
    match string_to_integer(&value, 10) {
        Option::Some(int) => int,
        _ => {
            let mut message: String = string("invalid integer literal: ");
            string_push_string(&mut message, &value);
            rLexer_error(lexer, &message);
        },
    }
}

fn rLexer_scan_char_literal_or_lifetime(lexer: &mut RLexer) -> RToken {
    rLexer_expect_char(lexer, '\'');
    let c: char = match rLexer_consume_char(lexer) {
        Option::Some(ch) => {
            if ch == '\\' {
                rLexer_scan_escape_char(lexer)
            } else {
                ch
            }
        },
        _ => rLexer_error(lexer, &string("unexpected end of file")),
    };
    if rLexer_try_consume_char(lexer, '\'') {
        RToken::Literal(RLiteral::Char(c))
    } else {
        rLexer_scan_identifier(lexer); // lifetimes are ignored, so ignore identifier
        RToken::Lifetime
    }
}

fn rLexer_scan_string_literal(lexer: &mut RLexer) -> String {
    rLexer_expect_char(lexer, '"');
    let mut s: String = string_new();
    while true {
        match rLexer_consume_char(lexer) {
            Option::Some(c) => {
                if c == '"' {
                    return s;
                } else if c == '\\' {
                    string_push(&mut s, rLexer_scan_escape_char(lexer));
                } else {
                    string_push(&mut s, c);
                }
            },
            _ => rLexer_error(lexer, &string("unexpected end of string literal")),
        }
    }
    unreachable()
}

/// Scan an escape sequence after backslash.
fn rLexer_scan_escape_char(lexer: &mut RLexer) -> char {
    match rLexer_consume_char(lexer) {
        Option::Some(c) => match c {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            '0' => '\0',
            c => c,
        },
        _ => rLexer_error(lexer, &string("unexpected end of escape sequence")),
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
            rLexer_error(lexer, &message);
        },
    }
}

fn rLexer_scan_slash(lexer: &mut RLexer) -> RToken {
    if rLexer_try_consume_char(lexer, '/') {
        rLexer_consume_char(lexer);
        rLexer_skip_line_comment(lexer);
        rLexer_next_token(lexer)
    } else {
        RToken::Slash
    }
}

fn rLexer_scan_colon(lexer: &mut RLexer) -> RToken {
    if rLexer_try_consume_char(lexer, ':') {
        RToken::DoubleColon
    } else {
        RToken::Colon
    }
}

fn rLexer_scan_equals(lexer: &mut RLexer) -> RToken {
    if rLexer_try_consume_char(lexer, '=') {
        RToken::Eq
    } else if rLexer_try_consume_char(lexer, '>') {
        RToken::FatArrow
    } else {
        RToken::Assign
    }
}

fn rLexer_scan_minus(lexer: &mut RLexer) -> RToken {
    if rLexer_try_consume_char(lexer, '>') {
        RToken::Arrow
    } else {
        RToken::Minus
    }
}

fn rLexer_scan_bang(lexer: &mut RLexer) -> RToken {
    if rLexer_try_consume_char(lexer, '=') {
        RToken::Neq
    } else {
        RToken::Bang
    }
}

fn rLexer_scan_less(lexer: &mut RLexer) -> RToken {
    if rLexer_try_consume_char(lexer, '=') {
        RToken::Leq
    } else {
        RToken::LAngle
    }
}

fn rLexer_scan_greater(lexer: &mut RLexer) -> RToken {
    if rLexer_try_consume_char(lexer, '=') {
        RToken::Geq
    } else {
        RToken::RAngle
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
            _ => return,
        }
    }
}

fn rLexer_skip_line_comment(lexer: &mut RLexer) {
    while true {
        match rLexer_consume_char(lexer) {
            Option::Some(c) => {
                if c == '\n' {
                    return;
                }
            },
            _ => return,
        }
    }
}

/// Skips attributes which are useful in Rust, but unsupported.
fn rLexer_skip_whitespace_and_attributes(lexer: &mut RLexer) {
    rLexer_skip_whitespace(lexer);
    while true {
        if rLexer_try_consume_char(lexer, '#') {
            rLexer_skip_whitespace(lexer);
            rLexer_try_consume_char(lexer, '!');
            rLexer_skip_whitespace(lexer);

            if rLexer_try_consume_char(lexer, '[') {
                let mut skipping: bool = true;
                while skipping {
                    match rLexer_consume_char(lexer) {
                        Option::Some(c) => {
                            if c == ']' {
                                skipping = false
                            }
                        },
                        _ => rLexer_error(lexer, &string("attribute is missing closing ']'")),
                    }
                }
            } else {
                rLexer_error(lexer, &string("expected '[' after '#'"));
            }
        } else {
            return;
        }
        rLexer_skip_whitespace(lexer);
    }
}

// -----------------------------------------------------------------
// -------------------------- Parser -------------------------------
// -----------------------------------------------------------------

/// Abstract Syntax Tree of a parsed Rust source.
enum RAst {
    Language(Vec<RAstItem>),
}

/// Top-level items.
enum RAstItem {
    Function(RAstFunction),
    Enum(RAstEnum),
    ExternBlock(Vec<RAstExternFn>),
}

/// Function definition.
enum RAstFunction {
    /// generic, unsafe, name, parameters, return type, body
    Fn(bool, bool, String, Vec<RAstVariable>, RType, RAstBlock),
}

/// Enum definition.
enum RAstEnum {
    /// name, variants, is_generic
    Enum(String, Vec<RAstVariant>, bool),
}

/// Extern function declaration.
enum RAstExternFn {
    /// name, parameters, return type
    Fn(String, Vec<RAstVariable>, RType),
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

/// Types from the Rust subset.
enum RType {
    U8,
    Usize,
    Bool,
    Char,
    Unit,
    Never,
    /// name, optional generic type
    Enum(String, Option<Box<RType>>),
    /// pointee, mutable
    Reference(Box<RType>, bool),
    RawPointerMut(Box<RType>),
    Generic,
}

/// A Rust expression.
enum RAstExpr {
    Return(Option<Box<RAstExpr>>),
    Assign(Box<RAstExpr>, Box<RAstExpr>),
    Binary(RAstBinaryOp, Box<RAstExpr>, Box<RAstExpr>),
    Cast(Box<RAstExpr>, RType),
    Unary(RAstUnaryOp, Box<RAstExpr>),
    Literal(RLiteral),
    Variable(String),
    /// path segments, arguments, optional generic type instance
    Path(Vec<String>, Vec<RAstExpr>, Option<RType>), // either function call or enum instantiaton
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
    /// `&*` / `&mut *`
    DereferenceReference(bool),
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

/// Return true if the given function is generic.
fn rAstFunction_name(RAstFunction::Fn(_, _, name, _, _, _): &RAstFunction) -> &String {
    name
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
        RLiteral::String(_) => RType::Enum(string("&str"), Option::<Box<RType>>::None),
    }
}

fn rAstPattern_is_wildcard(pattern: &RAstPattern) -> bool {
    match pattern {
        RAstPattern::Wildcard => true,
        _ => false,
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
fn rType_size(codegen: &mut Codegen, icg: &ICodegen, ty: &RType) -> usize {
    match ty {
        RType::U8 | RType::Char | RType::Bool => 1,
        RType::Usize | RType::Reference(_, _) | RType::RawPointerMut(_) => size_of::<usize>(),
        RType::Unit | RType::Never => 0,
        RType::Enum(name, generic) => {
            let generic: Option<RType> = match generic {
                Option::Some(instance) => Option::<RType>::Some(rType_clone(box_deref::<RType>(instance))),
                _ => Option::<RType>::None,
            };
            match iCodegen_search_global(icg, name) {
                Option::Some(item) => match item {
                    Item::Enum(rast_enum) => {
                        match codegen_check_enum_size(codegen, rast_enum) {
                            Option::Some(size) => size, // return memoised size
                            Option::None => {
                                let size: usize = rAstEnum_size(codegen, icg, rast_enum, &generic);
                                let mut mangled_name: String = string_clone(name);
                                if is_some::<RType>(&generic) {
                                    string_push(&mut mangled_name, '.');
                                    let type_name: String = rType_to_string(&unwrap::<RType>(generic));
                                    string_push_string(&mut mangled_name, &type_name)
                                }
                                codegen_cache_enum_size(codegen, mangled_name, size);
                                size
                            },
                        }
                    },
                    _ => 16, // assume that it is the built-in &str (8 bytes for pointer, 8 bytes for length)
                },
                _ => 16, // assume that it is the built-in &str (8 bytes for pointer, 8 bytes for length)
            }
        },
        RType::Generic => match codegen_generic_instance(codegen, codegen_current_function(codegen)) {
            Option::Some(instance) => rType_size(codegen, icg, &instance),
            _ => panic("cannot identify size of uninstantiated generic type"),
        },
    }
}

/// Return the size of an enum in bytes.
fn rAstEnum_size(codegen: &mut Codegen, icg: &ICodegen, e: &RAstEnum, generic: &Option<RType>) -> usize {
    let RAstEnum::Enum(_, variants, _): &RAstEnum = e;
    let mut max_size: usize = 0;
    let mut i: usize = 0;
    while i < vec_len::<RAstVariant>(variants) {
        let RAstVariant::Variant(_, field_types): &RAstVariant = vec_at::<RAstVariant>(variants, i);

        let mut j: usize = 0;
        let mut size: usize = 0;
        while j < vec_len::<RType>(field_types) {
            let ty: &RType = vec_at::<RType>(field_types, j);
            let field: RType = rType_instantiate_generic(ty, generic, iCodegen_globals(icg));
            size = size + rType_size(codegen, icg, &field);
            j = j + 1;
        }

        max_size = max(max_size, size);
        i = i + 1;
    }
    // The size is aligned to 8, because the current implementation uses i64 (8 byte wide) elements.
    // If the size were not aligned to 8, there would be a size mismatch between size_of::<T>() (which is
    // the size returned here) and the actual size of the allocated memory (which is always a multiple of 8).
    round_to_next_multiple(8 + max_size, 8) // + 8 bytes for the discriminant
}

/// Get the types of the given enum variant's fields.
fn rAstEnum_variant_fields<'a>(variants: &'a Vec<RAstVariant>, variant: &String) -> Option<&'a Vec<RType>> {
    let mut i: usize = 0;
    while i < vec_len::<RAstVariant>(variants) {
        let RAstVariant::Variant(name, types): &RAstVariant = vec_at::<RAstVariant>(variants, i);
        if string_eq(name, variant) {
            return Option::<&Vec<RType>>::Some(types);
        }
        i = i + 1;
    }
    Option::<&Vec<RType>>::None
}

/// Get an identifying discriminator for a given variant of variants.
/// If the variant is not present, 0 is returned.
fn variants_get_discriminator(variants: &Vec<RAstVariant>, variant: &String) -> usize {
    let mut tag: usize = 0;
    while tag < vec_len::<RAstVariant>(variants) {
        let RAstVariant::Variant(name, _): &RAstVariant = vec_at::<RAstVariant>(variants, tag);
        if string_eq(name, variant) {
            return tag;
        }
        tag = tag + 1;
    }
    0
}

/// Convert a Rust type into a LLVM-IR type name.
fn rType_to_llvm_name(codegen: &mut Codegen, ty: &RType) -> String {
    match ty {
        RType::U8 | RType::Char => string("i8"),
        RType::Usize => string("i64"), // assume 64-bit for now
        RType::Bool => string("i1"),
        RType::Unit | RType::Never => string("void"),
        RType::Reference(_, _) | RType::RawPointerMut(_) => string("ptr"),
        RType::Generic => match codegen_generic_instance(codegen, codegen_current_function(codegen)) {
            Option::Some(instance) => rType_to_llvm_name(codegen, &instance),
            _ => panic("can't determine a LLVM type for an uninstantiated generic type"),
        },
        RType::Enum(_, _) => string("ptr"),
    }
}

fn rType_is_numeric(ty: &RType) -> bool {
    match ty {
        RType::U8 | RType::Usize => true,
        _ => false,
    }
}

fn rType_is_enum(codegen: &Codegen, ty: &RType) -> bool {
    match ty {
        RType::Enum(_, _) => true,
        RType::Generic => match codegen_generic_instance(codegen, codegen_current_function(codegen)) {
            Option::Some(instance) => rType_is_enum(codegen, &instance),
            _ => panic("unexpected missing instantiation for generic type in generic function"),
        },
        _ => false,
    }
}

fn rType_is_reference(ty: &RType) -> bool {
    match ty {
        RType::Reference(_, _) => true,
        _ => false,
    }
}

fn rType_is_pointer(ty: &RType) -> bool {
    match ty {
        RType::Reference(_, _) | RType::RawPointerMut(_) => true,
        _ => false,
    }
}

/// Rawrust allows comparison of integers, characters and booleans.
fn rType_is_comparable(ty: &RType) -> bool {
    match ty {
        RType::Usize | RType::U8 | RType::Char | RType::Bool => true,
        _ => false,
    }
}

/// Coalesces two types into one type. This is a simplified version of Rust's Least Upper Bound
/// Coercion.  If `left` cannot be coerced to `right`, `left` is returned, else returns `right`.
fn rType_coalesce(left: RType, right: RType) -> RType {
    if rType_coerced_match(&left, &right) {
        right
    } else {
        left
    }
}

/// Return true if the given types, coerced from `left` to `right`, match.
/// Coercions:
///   - T      ~> T
///   - &mut T ~> &T
///   - !      ~> T (any type)
///
/// That is, two types `a`, `b` match, if
/// `a == b || a == ! || a == &mut T && b == &T
fn rType_coerced_match(left: &RType, right: &RType) -> bool {
    or(
        or(rType_eq(left, right), rType_eq(left, &RType::Never)),
        match left {
            RType::Reference(inner_left, mutable_a) => match right {
                RType::Reference(inner_right, mutable_b) => and(
                    rType_eq(box_deref::<RType>(inner_left), box_deref::<RType>(inner_right)),
                    // mut_l ~> mut_r iff mut_r => mut_l (i.e. not coercable if right is mutable, but left is not)
                    or(not(*mutable_b), *mutable_a),
                ),
                _ => false,
            },
            _ => false,
        },
    )
}

/// Return true if the type has a value.
/// This is true for all types, other than Unit and Never.
fn rType_has_value(ty: &RType) -> bool {
    not(rType_coerced_match(ty, &RType::Unit))
}

/// If the given type is generic and the generic mapping is not None, return the mapped type.
fn rType_instantiate_generic(ty: &RType, mapping: &Option<RType>, items: &StringMap<Item>) -> RType {
    match ty {
        RType::Generic => match mapping {
            Option::Some(instance) => rType_clone(instance),
            _ => rType_clone(ty),
        },
        RType::Reference(inner, mutable) => RType::Reference(
            box_new::<RType>(rType_instantiate_generic(
                box_deref::<RType>(inner),
                mapping,
                items,
            )),
            *mutable,
        ),
        RType::RawPointerMut(inner) => RType::RawPointerMut(box_new::<RType>(rType_instantiate_generic(
            box_deref::<RType>(inner),
            mapping,
            items,
        ))),
        RType::Enum(name, generic) => match generic {
            Option::Some(instance) => RType::Enum(
                string_clone(name),
                Option::<Box<RType>>::Some(box_new::<RType>(rType_instantiate_generic(
                    box_deref::<RType>(instance),
                    mapping,
                    items,
                ))),
            ),
            // set instantiation only if the enum is generic
            _ => match mapping {
                Option::Some(instance) => match stringMap_get::<Item>(items, name) {
                    Option::Some(item) => match item {
                        Item::Enum(RAstEnum::Enum(_, _, is_generic)) => {
                            if *is_generic {
                                RType::Enum(
                                    string_clone(name),
                                    Option::<Box<RType>>::Some(box_new::<RType>(rType_clone(instance))),
                                )
                            } else {
                                rType_clone(ty)
                            }
                        },
                        _ => rType_clone(ty),
                    },
                    _ => rType_clone(ty),
                },
                _ => rType_clone(ty),
            },
        },
        _ => rType_clone(ty),
    }
}

/// If the given type is a generic enum, extract the instance type if there is one.
fn rType_extract_enum_generic(ty: &RType) -> Option<RType> {
    match ty {
        RType::Enum(_, generic) => match generic {
            Option::Some(instance) => Option::<RType>::Some(rType_clone(box_deref::<RType>(instance))),
            _ => Option::<RType>::None,
        },
        RType::Reference(inner, _) => rType_extract_enum_generic(box_deref::<RType>(inner)),
        RType::RawPointerMut(inner) => rType_extract_enum_generic(box_deref::<RType>(inner)),
        _ => Option::<RType>::None,
    }
}

/// Possible types of a scrutinee in a match expression. The contained type is the type the Scrutinee is matched
/// against, while the variant encodes whether bindings are (mutable) references.
enum Scrutinee {
    Value(RType),
    /// inner type, mutable
    Reference(RType, bool),
}

/// Construct Scrutinee from a given Rust type.
fn scrutinee_from_type(ty: &RType) -> Scrutinee {
    match ty {
        RType::Reference(inner, mutable) => {
            Scrutinee::Reference(rType_clone(box_deref::<RType>(inner)), *mutable)
        },
        _ => Scrutinee::Value(rType_clone(ty)),
    }
}

/// Return the type that the scrutinee is matched on.
fn scrutinee_match_type(scrutinee: &Scrutinee) -> &RType {
    match scrutinee {
        Scrutinee::Value(ty) => ty,
        Scrutinee::Reference(ty, _) => ty,
    }
}

/// If the scrutinee is a reference, the returned type is a reference to `ty`, otherwise, return `ty`.
fn scrutinee_inherit_borrow(scrutinee: &Scrutinee, ty: &RType) -> RType {
    match scrutinee {
        Scrutinee::Value(_) => rType_clone(ty),
        Scrutinee::Reference(_, mutable) => RType::Reference(box_new::<RType>(rType_clone(ty)), *mutable),
    }
}

/// Return the type of the binding, which binds the given scrutinee.
fn scrutinee_binding_type(scrutinee: &Scrutinee) -> RType {
    match scrutinee {
        Scrutinee::Value(ty) => rType_clone(ty),
        Scrutinee::Reference(ty, mutable) => RType::Reference(box_new::<RType>(rType_clone(ty)), *mutable),
    }
}

/// Return true if the type of the scrutinee is a reference.
fn scrutinee_is_reference(scrutinee: &Scrutinee) -> bool {
    match scrutinee {
        Scrutinee::Reference(_, _) => true,
        _ => false,
    }
}

/// Require and consume the given token.
fn expect_token(lexer: &mut RLexer, token: &RToken) {
    if not(rLexer_try_consume(lexer, token)) {
        let bad_token: &RToken = rLexer_current_token(lexer);
        let mut message: String = string("expected ");
        string_push_string(&mut message, &rToken_to_string(token));
        string_push_str(&mut message, ", but got: ");
        string_push_string(&mut message, &rToken_to_string(bad_token));
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
            string_push_string(&mut message, &rToken_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

/// Try to parse a generic type parameter. If there is none, return false.
fn parse_generic(lexer: &mut RLexer) -> bool {
    if not(rLexer_try_consume(lexer, &RToken::LAngle)) {
        return false;
    }
    // lifetime parameter is ignored
    if rLexer_try_consume(lexer, &RToken::Lifetime) {
        if rLexer_try_consume(lexer, &RToken::RAngle) {
            return false;
        }
        expect_token(lexer, &RToken::Comma);
    }
    let type_param: String = expect_identifier(lexer);
    if not(str_eq(&type_param, "T")) {
        parse_error(lexer, &string("generic type parameter must be \"T\""));
    }
    expect_token(lexer, &RToken::RAngle);
    true
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
                    string_push_string(&mut message, &rToken_to_string(&token));
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
                let mut message: String = string("expected function, enum, or extern block, but got: ");
                string_push_string(&mut message, &rToken_to_string(token));
                parse_error(lexer, &message);
            },
        }
    }

    RAst::Language(items)
}

fn parse_extern_block(lexer: &mut RLexer) -> Vec<RAstExternFn> {
    expect_token(lexer, &RToken::Extern);

    match rLexer_current_token(lexer) {
        RToken::Literal(literal) => match literal {
            RLiteral::String(value) => {
                if not(str_eq(value, "C")) {
                    let mut message: String = string("expected \"C\", but got: ");
                    string_push_string(&mut message, &rToken_to_string(rLexer_current_token(lexer)));
                    parse_error(lexer, &message);
                }
                rLexer_next_token(lexer);
            },
            _ => {
                let mut message: String = string("expected \"C\", but got: ");
                string_push_string(&mut message, &rToken_to_string(rLexer_current_token(lexer)));
                parse_error(lexer, &message);
            },
        },
        _ => {
            let mut message: String = string("expected \"C\", but got: ");
            string_push_string(&mut message, &rToken_to_string(rLexer_current_token(lexer)));
            parse_error(lexer, &message);
        },
    };

    expect_token(lexer, &RToken::LBrace);

    let mut functions: Vec<RAstExternFn> = vec_new::<RAstExternFn>();
    while not(rLexer_current_token_eq(lexer, &RToken::RBrace)) {
        let function: RAstExternFn = parse_function_declaration(lexer);
        vec_push::<RAstExternFn>(&mut functions, function);
    }
    expect_token(lexer, &RToken::RBrace);

    functions
}

fn parse_function_declaration(lexer: &mut RLexer) -> RAstExternFn {
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
    RAstExternFn::Fn(name, parameters, return_type)
}

fn parse_function(lexer: &mut RLexer, is_unsafe: bool) -> RAstFunction {
    expect_token(lexer, &RToken::Fn);
    let name: String = expect_identifier(lexer);
    let is_generic: bool = parse_generic(lexer);
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

    RAstFunction::Fn(is_generic, is_unsafe, name, parameters, return_type, body)
}

fn parse_enum(lexer: &mut RLexer) -> RAstEnum {
    expect_token(lexer, &RToken::Enum);
    let name: String = expect_identifier(lexer);
    let is_generic: bool = parse_generic(lexer);
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

    RAstEnum::Enum(name, variants, is_generic)
}

fn parse_variant(lexer: &mut RLexer) -> RAstVariant {
    let name: String = expect_identifier(lexer);

    let mut field_types: Vec<RType> = vec_new::<RType>();
    if rLexer_try_consume(lexer, &RToken::LParen) {
        vec_push::<RType>(&mut field_types, parse_type(lexer));

        while and(
            rLexer_try_consume(lexer, &RToken::Comma),
            not(rLexer_current_token_eq(lexer, &RToken::RParen)),
        ) {
            vec_push::<RType>(&mut field_types, parse_type(lexer));
        }
        expect_token(lexer, &RToken::RParen);
    }

    RAstVariant::Variant(name, field_types)
}

fn parse_block(lexer: &mut RLexer) -> RAstBlock {
    expect_token(lexer, &RToken::LBrace);
    let mut statements: Vec<RAstStatement> = vec_new::<RAstStatement>();
    let mut tail: Option<Box<RAstExpr>> = Option::<Box<RAstExpr>>::None;

    while not(rLexer_current_token_eq(lexer, &RToken::RBrace)) {
        if rLexer_current_token_eq(lexer, &RToken::Let) {
            let let_binding: RAstStatement = parse_binding(lexer);
            vec_push::<RAstStatement>(&mut statements, let_binding);
            expect_token(lexer, &RToken::SemiColon);
        } else {
            let expression: RAstExpr = parse_expression(lexer);

            if rLexer_current_token_eq(lexer, &RToken::RBrace) {
                // end of block with expression as return value
                tail = Option::<Box<RAstExpr>>::Some(box_new::<RAstExpr>(expression));
            } else {
                rLexer_try_consume(lexer, &RToken::SemiColon); // optional for if/while/match
                let statement: RAstStatement = RAstStatement::Expression(box_new::<RAstExpr>(expression));
                vec_push::<RAstStatement>(&mut statements, statement);
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
            rLexer_try_consume(lexer, &RToken::Lifetime); // ignore lifetime annotation
            match rLexer_current_token(lexer) {
                RToken::Identifier(name) => {
                    if str_eq(name, "str") {
                        rLexer_next_token(lexer);
                        return RType::Enum(string("&str"), Option::<Box<RType>>::None);
                    }
                },
                _ => {},
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
            let name: String = expect_identifier(lexer);
            if str_eq(&name, "T") {
                RType::Generic // T is always a generic parameter
            } else if rLexer_try_consume(lexer, &RToken::LAngle) {
                if rLexer_try_consume(lexer, &RToken::Lifetime) {
                    expect_token(lexer, &RToken::RAngle);
                    return RType::Enum(name, Option::<Box<RType>>::None);
                }
                let instance: RType = parse_type(lexer);
                expect_token(lexer, &RToken::RAngle);
                RType::Enum(name, Option::<Box<RType>>::Some(box_new::<RType>(instance)))
            } else {
                RType::Enum(name, Option::<Box<RType>>::None)
            }
        },
        token => {
            let mut message: String = string("expected a type, but got: ");
            string_push_string(&mut message, &rToken_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

fn parse_expression(lexer: &mut RLexer) -> RAstExpr {
    match rLexer_current_token(lexer) {
        RToken::Return => {
            rLexer_next_token(lexer);
            match rLexer_current_token(lexer) {
                RToken::SemiColon | RToken::RBrace | RToken::Comma => {
                    RAstExpr::Return(Option::<Box<RAstExpr>>::None)
                },
                _ => {
                    let expression: RAstExpr = parse_expression(lexer);
                    RAstExpr::Return(Option::<Box<RAstExpr>>::Some(box_new::<RAstExpr>(expression)))
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
        RToken::Eq | RToken::Neq | RToken::LAngle | RToken::RAngle | RToken::Leq | RToken::Geq => {
            let comparison: RToken = rToken_clone(rLexer_current_token(lexer));
            rLexer_next_token(lexer);

            let right: RAstExpr = parse_arithmetic(lexer);

            let operator: RAstComparisonOp = match comparison {
                RToken::Eq => RAstComparisonOp::Eq,
                RToken::Neq => RAstComparisonOp::Ne,
                RToken::RAngle => RAstComparisonOp::Gt,
                RToken::LAngle => RAstComparisonOp::Lt,
                RToken::Geq => RAstComparisonOp::Ge,
                RToken::Leq => RAstComparisonOp::Le,
                _ => unreachable(),
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
            _ => unreachable(),
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
            _ => unreachable(),
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
            let op: RAstUnaryOp = if rLexer_current_token_eq(lexer, &RToken::Star) {
                rLexer_next_token(lexer);
                RAstUnaryOp::DereferenceReference(mutable)
            } else {
                RAstUnaryOp::Reference(mutable)
            };
            let inner: RAstExpr = parse_unary(lexer);
            RAstExpr::Unary(op, box_new::<RAstExpr>(inner))
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
            string_push_string(&mut message, &rToken_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

/// Parses either a variable, a function call or an instantiation of an enum.
fn parse_identifier_expression(lexer: &mut RLexer) -> RAstExpr {
    let first_identifier: String = expect_identifier(lexer);
    let mut generic: Option<RType> = Option::<RType>::None;

    if rLexer_try_consume(lexer, &RToken::DoubleColon) {
        if rLexer_try_consume(lexer, &RToken::LAngle) {
            let ty: RType = parse_type(lexer);
            expect_token(lexer, &RToken::RAngle);
            generic = Option::<RType>::Some(ty);
        }
        let mut segments: Vec<String> = vec_new::<String>();
        vec_push::<String>(&mut segments, first_identifier);
        match rLexer_current_token(lexer) {
            RToken::Identifier(_) => vec_push::<String>(&mut segments, expect_identifier(lexer)),
            RToken::DoubleColon => {
                rLexer_next_token(lexer);
                vec_push::<String>(&mut segments, expect_identifier(lexer));
            },
            _ => {},
        };
        parse_path_values(lexer, segments, generic)
    } else if rLexer_current_token_eq(lexer, &RToken::LParen) {
        let mut segments: Vec<String> = vec_new::<String>();
        vec_push::<String>(&mut segments, first_identifier);
        parse_path_values(lexer, segments, generic)
    } else {
        RAstExpr::Variable(first_identifier)
    }
}

fn parse_if(lexer: &mut RLexer) -> RAstIf {
    expect_token(lexer, &RToken::If);
    let condition: RAstExpr = parse_expression(lexer);
    let then_block: RAstBlock = parse_block(lexer);

    let else_branch: Option<RAstElse> = if rLexer_try_consume(lexer, &RToken::Else) {
        if rLexer_current_token_eq(lexer, &RToken::If) {
            let else_if: RAstIf = parse_if(lexer);
            Option::<RAstElse>::Some(RAstElse::If(box_new::<RAstIf>(else_if)))
        } else {
            let else_block: RAstBlock = parse_block(lexer);
            Option::<RAstElse>::Some(RAstElse::Block(else_block))
        }
    } else {
        Option::<RAstElse>::None
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

            if str_eq(&identifier, "_") {
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
            string_push_string(&mut message, &rToken_to_string(token));
            parse_error(lexer, &message);
        },
    }
}

fn parse_path_values(lexer: &mut RLexer, path: Vec<String>, generic: Option<RType>) -> RAstExpr {
    if not(rLexer_try_consume(lexer, &RToken::LParen)) {
        return RAstExpr::Path(path, vec_new::<RAstExpr>(), generic); // enum without fields
    }

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

    RAstExpr::Path(path, values, generic)
}

/// Collect all global items (functions and enums) into a map and return it.
fn collect_items(ast: &RAst) -> StringMap<Item> {
    let RAst::Language(ast_items): &RAst = ast;
    let mut items: StringMap<Item> = stringMap_new::<Item>();

    let mut i: usize = 0;
    while i < vec_len::<RAstItem>(ast_items) {
        let item: &RAstItem = vec_at::<RAstItem>(ast_items, i);
        insert_item_into_global_table(&mut items, item);
        i = i + 1;
    }
    insert_builtin_functions(&mut items);
    items
}

/// Clones function signatures and enum definitions from the AST into global symbol table entries
/// and inserts them into the table.
fn insert_item_into_global_table(table: &mut StringMap<Item>, item: &RAstItem) {
    match item {
        RAstItem::Function(RAstFunction::Fn(is_generic, is_unsafe, name, params, return_type, _)) => {
            let mut param_types: Vec<RType> = vec_with_capacity::<RType>(vec_len::<RAstVariable>(params));
            let mut i: usize = 0;
            while i < vec_len::<RAstVariable>(params) {
                let RAstVariable::Variable(_, t): &RAstVariable = vec_at::<RAstVariable>(params, i);
                vec_push::<RType>(&mut param_types, rType_clone(t));
                i = i + 1;
            }

            let item: Item = Item::Function(rType_clone(return_type), param_types, *is_unsafe, *is_generic);
            stringMap_insert_or_update::<Item>(table, name, item);
        },
        RAstItem::Enum(RAstEnum::Enum(name, variants, is_generic)) => {
            let mut cloned_variants: Vec<RAstVariant> =
                vec_with_capacity::<RAstVariant>(vec_len::<RAstVariant>(variants));
            let mut i: usize = 0;
            while i < vec_len::<RAstVariant>(variants) {
                let RAstVariant::Variant(name, field_types): &RAstVariant =
                    vec_at::<RAstVariant>(variants, i);

                let types: Vec<RType> = types_clone(field_types);
                let variant: RAstVariant = RAstVariant::Variant(string_clone(name), types);
                vec_push::<RAstVariant>(&mut cloned_variants, variant);
                i = i + 1;
            }
            let item: Item = Item::Enum(RAstEnum::Enum(string_clone(name), cloned_variants, *is_generic));
            stringMap_insert_or_update::<Item>(table, name, item);
        },
        RAstItem::ExternBlock(functions) => {
            let mut i: usize = 0;
            while i < vec_len::<RAstExternFn>(functions) {
                let RAstExternFn::Fn(name, params, return_type): &RAstExternFn =
                    vec_at::<RAstExternFn>(functions, i);

                let mut types: Vec<RType> = vec_with_capacity::<RType>(vec_len::<RAstVariable>(params));
                let mut j: usize = 0;
                while j < vec_len::<RAstVariable>(params) {
                    let RAstVariable::Variable(_, param_type): &RAstVariable =
                        vec_at::<RAstVariable>(params, j);
                    vec_push::<RType>(&mut types, rType_clone(param_type));
                    j = j + 1;
                }

                let item: Item = Item::Function(rType_clone(return_type), types, true, false);
                stringMap_insert_or_update::<Item>(table, name, item);
                i = i + 1;
            }
        },
    }
}

/// Insert all built-in functions into the global table
fn insert_builtin_functions(table: &mut StringMap<Item>) {
    let as_ptr: String = string("str::as_ptr");
    let mut parameters: Vec<RType> = vec_new::<RType>();
    let str_type: RType = RType::Enum(string("&str"), Option::<Box<RType>>::None);
    vec_push::<RType>(&mut parameters, str_type);
    let return_type: RType = RType::RawPointerMut(box_new::<RType>(RType::U8));
    let item: Item = Item::Function(return_type, parameters, false, false);
    stringMap_insert::<Item>(table, as_ptr, item);

    let len: String = string("str::len");
    let mut parameters: Vec<RType> = vec_new::<RType>();
    let str_type: RType = RType::Enum(string("&str"), Option::<Box<RType>>::None);
    vec_push::<RType>(&mut parameters, str_type);
    let item: Item = Item::Function(RType::Usize, parameters, false, false);
    stringMap_insert::<Item>(table, len, item);

    let sizeof: String = string("size_of");
    let item: Item = Item::Function(RType::Usize, vec_new::<RType>(), false, true);
    stringMap_insert::<Item>(table, sizeof, item);
}

// -----------------------------------------------------------------
// --------------------- Semantic Analysis -------------------------
// -----------------------------------------------------------------

/// Semantic analysis state.
enum Semantic {
    /// local symbol table, current return type, unsafe context depth, current function is generic
    Semantic(StringMapStack<Variable>, RType, usize, bool),
}

fn semantic_new() -> Semantic {
    Semantic::Semantic(stringMapStack_new::<Variable>(), RType::Unit, 0, false)
}

fn semantic_locals(semantic: &Semantic) -> &StringMapStack<Variable> {
    let Semantic::Semantic(locals, _, _, _): &Semantic = semantic;
    locals
}

fn semantic_locals_mut(semantic: &mut Semantic) -> &mut StringMapStack<Variable> {
    let Semantic::Semantic(locals, _, _, _): &mut Semantic = semantic;
    locals
}

fn semantic_current_fn_return_type(semantic: &Semantic) -> &RType {
    let Semantic::Semantic(_, return_type, _, _): &Semantic = semantic;
    return_type
}

fn semantic_set_current_fn_return_type(semantic: &mut Semantic, ty: RType) {
    let Semantic::Semantic(_, return_type, _, _): &mut Semantic = semantic;
    *return_type = ty;
}

/// Enter a new unsafe context.
fn semantic_push_unsafe_context(Semantic::Semantic(_, _, current_depth, _): &mut Semantic) {
    *current_depth = *current_depth + 1;
}

/// Exit an unsafe context.
fn semantic_pop_unsafe_context(Semantic::Semantic(_, _, current_depth, _): &mut Semantic) {
    if *current_depth == 0 {
        panic("unexpected leaving of unsafe block - this is a compiler bug")
    };
    *current_depth = *current_depth - 1;
}

/// Return true if unsafe operations are allowed.
fn semantic_is_unsafe_context(Semantic::Semantic(_, _, current_depth, _): &Semantic) -> bool {
    *current_depth > 0
}

fn semantic_set_is_generic(Semantic::Semantic(_, _, _, state): &mut Semantic, is_generic: bool) {
    *state = is_generic;
}

fn semantic_current_function_is_generic(Semantic::Semantic(_, _, _, is_generic): &Semantic) -> bool {
    *is_generic
}

/// Run semantic analysis and return collected items.
fn semantic_check_run(ast: &RAst, items: &StringMap<Item>) {
    let mut semantic: Semantic = semantic_new();
    semantic_check_language(&mut semantic, ast, items);
}

fn semantic_check_generic_usage(semantic: &Semantic, ty: &RType) {
    match ty {
        RType::Generic => {
            if not(semantic_current_function_is_generic(semantic)) {
                semantic_error(&string("cannot use type parameter \"T\" in non-generic function"));
            }
        },
        RType::Reference(inner, _) => {
            semantic_check_generic_usage(semantic, box_deref::<RType>(inner));
        },
        RType::RawPointerMut(inner) => {
            semantic_check_generic_usage(semantic, box_deref::<RType>(inner));
        },
        _ => {},
    }
}

/// Check if the given types are equal, otherwise throw an error.
fn semantic_expect_type_match(semantic: &Semantic, left: &RType, right: &RType) {
    semantic_check_generic_usage(semantic, left);
    semantic_check_generic_usage(semantic, right);
    if not(rType_eq(left, right)) {
        let mut message: String = string("type mismatch: expected ");
        string_push_string(&mut message, &rType_to_string(left));
        string_push_str(&mut message, ", but got ");
        string_push_string(&mut message, &rType_to_string(right));
        semantic_error(&message);
    }
}

/// Check if the given types, coerced from `actual` to `expected`, match.
fn semantic_expect_coerced_type_match(semantic: &Semantic, actual: &RType, expected: &RType) {
    semantic_check_generic_usage(semantic, actual);
    semantic_check_generic_usage(semantic, expected);
    if not(rType_coerced_match(actual, expected)) {
        let mut message: String = string("coerced type mismatch: expected ");
        string_push_string(&mut message, &rType_to_string(expected));
        string_push_str(&mut message, ", but got ");
        string_push_string(&mut message, &rType_to_string(actual));
        semantic_error(&message);
    }
}

/// Check if the given types, using Least Upper Bound Coercion, match.
fn semantic_expect_coalescing_type_match(semantic: &Semantic, left: &RType, right: &RType) {
    if not(rType_coerced_match(left, right)) {
        semantic_expect_coerced_type_match(semantic, right, left);
    }
}

fn semantic_expect_numeric_type(ty: &RType) {
    if not(rType_is_numeric(ty)) {
        let mut message: String = string("type mismatch: expected numeric type, but got ");
        string_push_string(&mut message, &rType_to_string(ty));
        semantic_error(&message);
    }
}

fn semantic_expect_bool_type(ty: &RType) {
    if not(rType_eq(ty, &RType::Bool)) {
        let mut message: String = string("type mismatch: expected bool type, but got ");
        string_push_string(&mut message, &rType_to_string(ty));
        semantic_error(&message);
    }
}

fn semantic_expect_comparable_type(ty: &RType) {
    if not(rType_is_comparable(ty)) {
        let mut message: String = string("type mismatch: expected a comparable type, but got ");
        string_push_string(&mut message, &rType_to_string(ty));
        semantic_error(&message);
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
    name: &String,
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
    /// return type, parameter types, is unsafe, is generic
    Function(RType, Vec<RType>, bool, bool),
    Enum(RAstEnum),
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
fn castOperation_select_operation(left_type: &RType, right_type: &RType) -> CastOperation {
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
                    rType_eq(box_deref::<RType>(left_inner), box_deref::<RType>(right_inner)),
                    *mutable,
                ) {
                    CastOperation::None
                } else {
                    CastOperation::Invalid
                }
            },
            RType::Reference(right_inner, other_mutable) => {
                if and(
                    rType_eq(box_deref::<RType>(left_inner), box_deref::<RType>(right_inner)),
                    or(not(*other_mutable), *mutable), // other_mutable => mutable
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

/// Run semantic analysis on the full AST.
// TODO: check duplicate functions/enums
fn semantic_check_language(semantic: &mut Semantic, ast: &RAst, globals: &StringMap<Item>) {
    let RAst::Language(items): &RAst = ast;
    let mut i: usize = 0;
    let len: usize = vec_len::<RAstItem>(items);
    while i < len {
        let item: &RAstItem = vec_at::<RAstItem>(items, i);
        match item {
            RAstItem::Function(function) => semantic_check_function(semantic, function, globals),
            RAstItem::Enum(e) => semantic_check_enum_def(semantic, e, globals),
            RAstItem::ExternBlock(_) => {}, // Rawrust does not enforce what the extern functions are
        };
        i = i + 1;
    }
}

fn semantic_check_enum_def(semantic: &mut Semantic, e: &RAstEnum, globals: &StringMap<Item>) {
    let RAstEnum::Enum(name, variants, is_generic): &RAstEnum = e;
    let mut i: usize = 0;
    while i < vec_len::<RAstVariant>(variants) {
        let RAstVariant::Variant(variant_name, fields): &RAstVariant = vec_at::<RAstVariant>(variants, i);

        let mut j: usize = 0;
        while j < vec_len::<RAstVariant>(variants) {
            let RAstVariant::Variant(other, _): &RAstVariant = vec_at::<RAstVariant>(variants, j);
            if and(i != j, string_eq(variant_name, other)) {
                semantic_error(&string("duplicate variants in enum found"));
            }
            j = j + 1;
        }
        semantic_check_variant(
            semantic,
            &RType::Enum(string_clone(name), Option::<Box<RType>>::None),
            fields,
            *is_generic,
            globals,
        );
        i = i + 1;
    }
}

fn semantic_check_variant(
    semantic: &Semantic,
    enum_type: &RType,
    fields: &Vec<RType>,
    is_generic: bool,
    globals: &StringMap<Item>,
) {
    let mut i: usize = 0;
    while i < vec_len::<RType>(fields) {
        let field: &RType = vec_at::<RType>(fields, i);
        semantic_check_variant_field(semantic, enum_type, field, is_generic, globals);
        i = i + 1;
    }
}

fn semantic_check_variant_field(
    semantic: &Semantic,
    enum_ty: &RType,
    field: &RType,
    is_generic: bool,
    globals: &StringMap<Item>,
) {
    match field {
        RType::Generic => {
            if not(is_generic) {
                semantic_error(&string(
                    "cannot use a generic type parameter in a non-generic enum",
                ))
            }
        },
        RType::Reference(inner, _) => {
            semantic_check_variant_field(semantic, enum_ty, box_deref::<RType>(inner), is_generic, globals)
        },
        RType::RawPointerMut(inner) => {
            semantic_check_variant_field(semantic, enum_ty, box_deref::<RType>(inner), is_generic, globals)
        },
        RType::Enum(name, generic) => {
            // TODO: check recursion
            match generic {
                Option::Some(instance) => {
                    let ty: &RType = box_deref::<RType>(instance);
                    semantic_check_variant_field(semantic, enum_ty, ty, is_generic, globals);
                },
                _ => {},
            };
            match stringMap_get::<Item>(globals, name) {
                Option::Some(item) => match item {
                    Item::Enum(_) => {},
                    _ => semantic_error(&string("cannot use an undefined enum in enum definition")),
                },
                _ => semantic_error(&string("cannot use an undefined enum in enum definition")),
            };
        },
        RType::Unit | RType::Never => semantic_error(&string("cannot use value-less type as enum field")),
        _ => {},
    }
}

/// Analyze one function and validate body against its signature.
fn semantic_check_function(semantic: &mut Semantic, function: &RAstFunction, globals: &StringMap<Item>) {
    let RAstFunction::Fn(is_generic, is_unsafe, _, parameters, return_type, body): &RAstFunction = function;

    semantic_set_is_generic(semantic, *is_generic);
    semantic_set_current_fn_return_type(semantic, rType_clone(return_type));
    semantic_enter_scope(semantic);

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstVariable>(parameters);
    while i < len {
        let RAstVariable::Variable(pattern, parameter_type): &RAstVariable =
            vec_at::<RAstVariable>(parameters, i);
        // TODO: error on duplicate names
        semantic_check_pattern(semantic, pattern, parameter_type, false, globals);
        i = i + 1;
    }
    let block_type: RType = semantic_check_block(semantic, body, *is_unsafe, globals);
    semantic_expect_coerced_type_match(semantic, &block_type, return_type);

    semantic_leave_scope(semantic);
    semantic_set_current_fn_return_type(semantic, RType::Unit);
    semantic_set_is_generic(semantic, false);
}

/// Analyze one block and return its resulting type.
fn semantic_check_block(
    semantic: &mut Semantic,
    block: &RAstBlock,
    is_unsafe: bool,
    globals: &StringMap<Item>,
) -> RType {
    let RAstBlock::Block(statements, tail): &RAstBlock = block;
    if is_unsafe {
        semantic_push_unsafe_context(semantic);
    }
    semantic_enter_scope(semantic);

    let mut diverges: bool = false;
    let mut i: usize = 0;
    let len: usize = vec_len::<RAstStatement>(statements);
    while i < len {
        let statement: &RAstStatement = vec_at::<RAstStatement>(statements, i);
        match statement {
            RAstStatement::Let(variable, value) => {
                let never: bool =
                    semantic_check_binding(semantic, variable, box_deref::<RAstExpr>(value), globals);
                diverges = or(diverges, never);
            },
            RAstStatement::Expression(expression) => {
                let expr_type: RType =
                    semantic_check_expression(semantic, box_deref::<RAstExpr>(expression), globals);
                diverges = or(diverges, rType_eq(&expr_type, &RType::Never));
            },
        };
        i = i + 1;
    }

    let block_type: RType = if diverges {
        RType::Never
    } else {
        match tail {
            Option::Some(expression) => {
                semantic_check_expression(semantic, box_deref::<RAstExpr>(expression), globals)
            },
            Option::None => RType::Unit,
        }
    };
    if is_unsafe {
        semantic_pop_unsafe_context(semantic);
    }
    semantic_leave_scope(semantic);
    block_type
}

/// Analyze one let-binding statement.
/// Return true if the right-hand side is a diverging expression
fn semantic_check_binding(
    semantic: &mut Semantic,
    variable: &RAstVariable,
    value: &RAstExpr,
    globals: &StringMap<Item>,
) -> bool {
    let RAstVariable::Variable(pattern, binding_type): &RAstVariable = variable;
    let actual_type: RType = semantic_check_expression(semantic, value, globals);
    semantic_expect_coerced_type_match(semantic, &actual_type, binding_type);
    semantic_check_pattern(semantic, pattern, binding_type, false, globals);
    rType_eq(&actual_type, &RType::Never)
}

fn semantic_check_expression(
    semantic: &mut Semantic,
    expression: &RAstExpr,
    globals: &StringMap<Item>,
) -> RType {
    match expression {
        RAstExpr::Return(returned) => semantic_check_return(semantic, returned, globals),
        RAstExpr::Assign(left, right) => semantic_check_assign(
            semantic,
            box_deref::<RAstExpr>(left),
            box_deref::<RAstExpr>(right),
            globals,
        ),
        RAstExpr::Binary(operator, left, right) => semantic_check_binary_op(
            semantic,
            operator,
            box_deref::<RAstExpr>(left),
            box_deref::<RAstExpr>(right),
            globals,
        ),
        RAstExpr::Cast(value, to_type) => {
            semantic_check_cast(semantic, box_deref::<RAstExpr>(value), to_type, globals)
        },
        RAstExpr::Unary(operator, value) => {
            semantic_check_unary_op(semantic, operator, box_deref::<RAstExpr>(value), globals)
        },
        RAstExpr::Literal(literal) => rAstLiteral_type(literal),
        RAstExpr::Variable(name) => semantic_check_variable_use(semantic, false, name),
        RAstExpr::Path(path, values, generic) => {
            semantic_check_path(semantic, path, values, globals, generic)
        },
        RAstExpr::Block(is_unsafe, block) => semantic_check_block(semantic, block, *is_unsafe, globals),
        RAstExpr::If(if_expression) => semantic_check_if(semantic, if_expression, globals),
        RAstExpr::While(condition, body) => {
            semantic_check_while(semantic, box_deref::<RAstExpr>(condition), body, globals)
        },
        RAstExpr::Match(value, arms) => {
            semantic_check_match(semantic, box_deref::<RAstExpr>(value), arms, globals)
        },
    }
}

fn semantic_check_return(
    semantic: &mut Semantic,
    returned: &Option<Box<RAstExpr>>,
    globals: &StringMap<Item>,
) -> RType {
    match returned {
        Option::Some(expression) => {
            let ty: RType = semantic_check_expression(semantic, box_deref::<RAstExpr>(expression), globals);
            semantic_expect_coerced_type_match(semantic, &ty, semantic_current_fn_return_type(semantic));
        },
        Option::None => {
            semantic_expect_type_match(semantic, semantic_current_fn_return_type(semantic), &RType::Unit);
        },
    };
    RType::Never
}

fn semantic_check_assign(
    semantic: &mut Semantic,
    left: &RAstExpr,
    right: &RAstExpr,
    globals: &StringMap<Item>,
) -> RType {
    let right_type: RType = semantic_check_expression(semantic, right, globals);
    let left_type: RType = match left {
        RAstExpr::Variable(name) => semantic_check_variable_use(semantic, true, name),
        RAstExpr::Unary(op, value) => match op {
            RAstUnaryOp::Dereference => {
                let expr: &RAstExpr = box_deref::<RAstExpr>(value);
                let pointer_type: RType = semantic_check_expression(semantic, expr, globals);
                let left_type: &RType = match &pointer_type {
                    RType::Reference(inner, mutable) => {
                        if not(*mutable) {
                            semantic_error(&string("invalid assignment using immutable reference"));
                        }
                        box_deref::<RType>(inner)
                    },
                    RType::RawPointerMut(inner) => {
                        if not(semantic_is_unsafe_context(semantic)) {
                            semantic_error(&string("raw pointer dereference requires unsafe"));
                        }
                        box_deref::<RType>(inner)
                    },
                    _ => semantic_error(&string("invalid assignment to an expression")),
                };
                semantic_expect_coerced_type_match(semantic, &right_type, left_type);
                return RType::Unit;
            },
            _ => semantic_error(&string("invalid assignment target")),
        },
        _ => semantic_error(&string("invalid assignment target")),
    };
    semantic_expect_coerced_type_match(semantic, &right_type, &left_type);
    RType::Unit
}

fn semantic_check_binary_op(
    semantic: &mut Semantic,
    operator: &RAstBinaryOp,
    left: &RAstExpr,
    right: &RAstExpr,
    globals: &StringMap<Item>,
) -> RType {
    let left_type: RType = semantic_check_expression(semantic, left, globals);
    let right_type: RType = semantic_check_expression(semantic, right, globals);
    semantic_expect_type_match(semantic, &left_type, &right_type);

    match operator {
        RAstBinaryOp::Arithmetic(_) => {
            semantic_expect_numeric_type(&left_type);
            left_type
        },
        RAstBinaryOp::Comparison(_) => {
            semantic_expect_comparable_type(&left_type);
            RType::Bool
        },
    }
}

fn semantic_check_cast(
    semantic: &mut Semantic,
    value: &RAstExpr,
    to_type: &RType,
    globals: &StringMap<Item>,
) -> RType {
    let from_type: RType = semantic_check_expression(semantic, value, globals);
    match castOperation_select_operation(&from_type, to_type) {
        CastOperation::Invalid => semantic_error(&string("invalid cast")),
        _ => rType_clone(to_type),
    }
}

fn semantic_check_unary_op(
    semantic: &mut Semantic,
    operator: &RAstUnaryOp,
    value: &RAstExpr,
    globals: &StringMap<Item>,
) -> RType {
    match operator {
        RAstUnaryOp::Reference(mutable_ref) => match value {
            RAstExpr::Variable(name) => RType::Reference(
                box_new::<RType>(semantic_check_variable_use(semantic, *mutable_ref, name)),
                *mutable_ref,
            ),
            _ => {
                let ty: RType = semantic_check_expression(semantic, value, globals);
                RType::Reference(box_new::<RType>(ty), *mutable_ref)
            },
        },
        RAstUnaryOp::Dereference | RAstUnaryOp::DereferenceReference(_) => {
            let expr_type: RType = semantic_check_expression(semantic, value, globals);
            let result_type: RType = match expr_type {
                RType::Reference(pointee, _) => rType_clone(box_deref::<RType>(&pointee)),
                RType::RawPointerMut(pointee) => {
                    if not(semantic_is_unsafe_context(semantic)) {
                        semantic_error(&string("raw pointer dereference requires unsafe context"));
                    }
                    rType_clone(box_deref::<RType>(&pointee))
                },
                _ => {
                    let mut message: String = string("cannot dereference an expression of type ");
                    string_push_string(&mut message, &rType_to_string(&expr_type));
                    semantic_error(&message);
                },
            };
            match operator {
                RAstUnaryOp::Dereference => result_type,
                RAstUnaryOp::DereferenceReference(mutable) => {
                    RType::Reference(box_new::<RType>(result_type), *mutable)
                },
                _ => unreachable(),
            }
        },
    }
}

fn semantic_check_variable_use(semantic: &mut Semantic, mutable: bool, name: &String) -> RType {
    match stringMapStack_get::<Variable>(semantic_locals(semantic), name) {
        Option::Some(Variable::Variable(ty, is_mutable)) => {
            if and(mutable, not(*is_mutable)) {
                let mut message: String = string("immutable variable cannot be used in mutable context: ");
                string_push_string(&mut message, name);
                semantic_error(&message)
            }
            rType_clone(ty)
        },
        _ => {
            let mut message: String = string("undefined variable: ");
            string_push_string(&mut message, name);
            semantic_error(&message)
        },
    }
}

fn semantic_check_path(
    semantic: &mut Semantic,
    path: &Vec<String>,
    values: &Vec<RAstExpr>,
    globals: &StringMap<Item>,
    generic: &Option<RType>,
) -> RType {
    let first_ident: &String = vec_at::<String>(path, 0);
    match stringMap_get::<Item>(globals, first_ident) {
        Option::Some(item) => match item {
            Item::Enum(e) => {
                let variant: &String = vec_at::<String>(path, 1);
                return semantic_check_enum(semantic, e, variant, values, globals, generic);
            },
            Item::Function(return_type, parameter_types, is_unsafe, is_generic) => {
                return semantic_check_call(
                    semantic,
                    first_ident,
                    return_type,
                    parameter_types,
                    *is_unsafe,
                    values,
                    globals,
                    *is_generic,
                    generic,
                );
            },
        },
        _ => {
            let function_name: String = rAstPath_to_string(path);
            match stringMap_get::<Item>(globals, &function_name) {
                Option::Some(item) => match item {
                    Item::Function(return_type, parameter_types, is_unsafe, is_generic) => {
                        return semantic_check_call(
                            semantic,
                            &function_name,
                            return_type,
                            parameter_types,
                            *is_unsafe,
                            values,
                            globals,
                            *is_generic,
                            generic,
                        );
                    },
                    _ => {},
                },
                _ => {},
            }
        },
    };
    let mut message: String = string("undefined function or enum: ");
    string_push_string(&mut message, &rAstPath_to_string(path));
    semantic_error(&message);
}

fn semantic_check_call(
    semantic: &mut Semantic,
    name: &String,
    return_type: &RType,
    parameter_types: &Vec<RType>,
    is_unsafe: bool,
    values: &Vec<RAstExpr>,
    globals: &StringMap<Item>,
    is_generic: bool,
    generic: &Option<RType>,
) -> RType {
    if and(is_unsafe, not(semantic_is_unsafe_context(semantic))) {
        semantic_error(&string("calling an unsafe function requires unsafe"));
    }
    match generic {
        Option::Some(_) => {
            if not(is_generic) {
                semantic_error(&string("non-generic function calls should not specify a type"));
            }
        },
        _ => {
            if is_generic {
                let mut msg: String = string("calling generic function requires turbofish syntax: ");
                string_push_string(&mut msg, name);
                string_push_str(&mut msg, " is a generic function");
                semantic_error(&msg)
            }
        },
    };
    if vec_len::<RType>(parameter_types) != vec_len::<RAstExpr>(values) {
        semantic_error(&string("function call does not have correct amount of arguments"));
    }
    let mut i: usize = 0;
    while i < vec_len::<RAstExpr>(values) {
        let param_type: &RType = vec_at::<RType>(parameter_types, i);
        let param_type: RType = rType_instantiate_generic(param_type, generic, globals);

        let arg: &RAstExpr = vec_at::<RAstExpr>(values, i);
        let arg_type: RType = semantic_check_expression(semantic, arg, globals);

        semantic_expect_coerced_type_match(semantic, &arg_type, &param_type);
        i = i + 1;
    }
    rType_instantiate_generic(return_type, generic, globals)
}

fn semantic_check_enum(
    semantic: &mut Semantic,
    RAstEnum::Enum(name, variants, is_generic): &RAstEnum,
    variant: &String,
    values: &Vec<RAstExpr>,
    globals: &StringMap<Item>,
    generic: &Option<RType>,
) -> RType {
    let instance: Option<Box<RType>> = match generic {
        Option::Some(ty) => {
            if not(*is_generic) {
                semantic_error(&string("non-generic enum constructors cannot specify a type"));
            }
            Option::<Box<RType>>::Some(box_new::<RType>(rType_clone(ty)))
        },
        _ => {
            if *is_generic {
                semantic_error(&string("generic enum constructors require turbofish syntax"))
            }
            Option::<Box<RType>>::None
        },
    };
    let fields: &Vec<RType> = match rAstEnum_variant_fields(variants, variant) {
        Option::Some(fields) => fields,
        _ => semantic_error(&string("use of undefined enum variant constructor")),
    };
    if vec_len::<RType>(fields) != vec_len::<RAstExpr>(values) {
        semantic_error(&string("enum constructor does not correct amount of fields"));
    }
    let mut i: usize = 0;
    while i < vec_len::<RType>(fields) {
        let field_type: &RType = vec_at::<RType>(fields, i);
        let field_type: RType = rType_instantiate_generic(field_type, generic, globals);
        let expr: &RAstExpr = vec_at::<RAstExpr>(values, i);
        let expr_type: RType = semantic_check_expression(semantic, expr, globals);
        semantic_expect_coerced_type_match(semantic, &expr_type, &field_type);
        i = i + 1;
    }
    RType::Enum(string_clone(name), instance)
}

fn semantic_check_if(semantic: &mut Semantic, if_expression: &RAstIf, globals: &StringMap<Item>) -> RType {
    let RAstIf::If(condition, then_block, else_branch): &RAstIf = if_expression;
    let cond_type: RType = semantic_check_expression(semantic, box_deref::<RAstExpr>(condition), globals);
    semantic_expect_bool_type(&cond_type);

    let then_type: RType = semantic_check_block(semantic, then_block, false, globals);
    match else_branch {
        Option::Some(else_branch) => {
            let else_type: RType = match else_branch {
                RAstElse::If(nested_if) => {
                    semantic_check_if(semantic, box_deref::<RAstIf>(nested_if), globals)
                },
                RAstElse::Block(block) => semantic_check_block(semantic, block, false, globals),
            };
            semantic_expect_coalescing_type_match(semantic, &then_type, &else_type);
            rType_coalesce(then_type, else_type)
        },
        Option::None => {
            let return_type: RType = RType::Unit;
            semantic_expect_coerced_type_match(semantic, &then_type, &return_type);
            return_type
        },
    }
}

fn semantic_check_while(
    semantic: &mut Semantic,
    condition: &RAstExpr,
    body: &RAstBlock,
    globals: &StringMap<Item>,
) -> RType {
    let condition_type: RType = semantic_check_expression(semantic, condition, globals);
    semantic_expect_bool_type(&condition_type);
    let body_type: RType = semantic_check_block(semantic, body, false, globals);
    semantic_expect_coerced_type_match(semantic, &body_type, &RType::Unit);
    RType::Unit
}

// TODO: exhaustiveness checking
fn semantic_check_match(
    semantic: &mut Semantic,
    value: &RAstExpr,
    arms: &Vec<RAstArm>,
    globals: &StringMap<Item>,
) -> RType {
    if vec_len::<RAstArm>(arms) == 0 {
        semantic_error(&string("match requires at least one arm"));
    }

    let expr_type: RType = semantic_check_expression(semantic, value, globals);
    let mut return_type: RType = RType::Never;

    let mut i: usize = 0;
    while i < vec_len::<RAstArm>(arms) {
        semantic_enter_scope(semantic);
        let arm: &RAstArm = vec_at::<RAstArm>(arms, i);
        let RAstArm::Arm(patterns, expression): &RAstArm = arm;
        let is_multi_pattern: bool = vec_len::<RAstPattern>(patterns) > 1;

        let mut j: usize = 0;
        while j < vec_len::<RAstPattern>(patterns) {
            let pattern: &RAstPattern = vec_at::<RAstPattern>(patterns, j);

            if is_multi_pattern {
                match pattern {
                    RAstPattern::EnumVariant(_, _, inner_patterns) => {
                        let mut i: usize = 0;
                        while i < vec_len::<RAstPattern>(inner_patterns) {
                            let pattern: &RAstPattern = vec_at::<RAstPattern>(inner_patterns, i);
                            if not(rAstPattern_is_wildcard(pattern)) {
                                semantic_error(&string(
                                    "enums in multi-pattern match arms cannot bind values, use wildcards",
                                ));
                            }
                            i = i + 1;
                        }
                    },
                    RAstPattern::Literal(_) => {},
                    _ => {
                        semantic_error(&string("multi-pattern match arms only support literal patterns"));
                    },
                };
            }

            semantic_check_pattern(semantic, pattern, &expr_type, true, globals);
            j = j + 1;
        }

        let arm_type: RType = semantic_check_expression(semantic, expression, globals);
        semantic_expect_coalescing_type_match(semantic, &return_type, &arm_type);
        return_type = rType_coalesce(return_type, arm_type);
        semantic_leave_scope(semantic);
        i = i + 1;
    }
    return_type
}

/// Check semantics of a pattern.
///
/// * `pattern`: the pattern to check.
/// * `expression_type`: the type of the value being matched on.
/// * `refutable`: if true, allow refutable patterns, otherwise do not.
fn semantic_check_pattern(
    semantic: &mut Semantic,
    pattern: &RAstPattern,
    expression_type: &RType,
    refutable_ok: bool,
    globals: &StringMap<Item>,
) {
    let scrutinee: Scrutinee = scrutinee_from_type(expression_type);
    let pattern_type: RType = match pattern {
        RAstPattern::Literal(literal) => {
            if not(refutable_ok) {
                semantic_error(&string("pattern must be irrefutable"))
            }
            match literal {
                RAstPatternLiteral::Int(_) => {
                    if rType_is_numeric(scrutinee_match_type(&scrutinee)) {
                        return; // numeric expression matches on numeric pattern
                    } else {
                        RType::Usize
                    }
                },
                RAstPatternLiteral::Char(_) => RType::Char,
                RAstPatternLiteral::Bool(_) => RType::Bool,
            }
        },
        RAstPattern::Identifier(mutable, name) => {
            let variable_type: RType = scrutinee_binding_type(&scrutinee);
            semantic_insert_variable(semantic, name, variable_type, *mutable);
            semantic_check_generic_usage(semantic, expression_type);
            return; // type agnostic
        },
        RAstPattern::Wildcard => return, // type agnostic
        RAstPattern::EnumVariant(enum_name, variant, inner_patterns) => {
            let mut enum_type: RType = RType::Enum(string_clone(enum_name), Option::<Box<RType>>::None);
            let generic: Option<RType> = rType_extract_enum_generic(scrutinee_match_type(&scrutinee));

            let fields: &Vec<RType> = match stringMap_get::<Item>(globals, enum_name) {
                Option::Some(item) => match item {
                    Item::Enum(RAstEnum::Enum(_, variants, is_generic)) => {
                        if *is_generic {
                            enum_type = rType_instantiate_generic(&enum_type, &generic, globals);
                        }
                        if and(not(refutable_ok), vec_len::<RAstVariant>(variants) > 1) {
                            let mut msg: String = string("nested enum patterns in must all be irrefutable: ");
                            string_push_string(&mut msg, enum_name);
                            string_push_str(&mut msg, " has more than one variant");
                            semantic_error(&msg);
                        }
                        match rAstEnum_variant_fields(variants, variant) {
                            Option::Some(fields) => fields,
                            _ => semantic_error(&string("unknown enum variant used in pattern")),
                        }
                    },
                    _ => semantic_error(&string("unknown enum used in pattern")),
                },
                _ => semantic_error(&string("unknown enum used in pattern")),
            };
            if vec_len::<RType>(fields) != vec_len::<RAstPattern>(inner_patterns) {
                semantic_error(&string("enum field count mismatch in pattern"));
            }
            let mut i: usize = 0;
            while i < vec_len::<RAstPattern>(inner_patterns) {
                let pattern: &RAstPattern = vec_at::<RAstPattern>(inner_patterns, i);
                let field: &RType = vec_at::<RType>(fields, i);
                let mut field_type: RType = rType_instantiate_generic(field, &generic, globals);
                field_type = scrutinee_inherit_borrow(&scrutinee, &field_type);
                match pattern {
                    RAstPattern::Identifier(mutable, name) => {
                        semantic_insert_variable(semantic, name, field_type, *mutable);
                    },
                    RAstPattern::EnumVariant(_, _, _) => {
                        semantic_check_pattern(semantic, pattern, &field_type, false, globals);
                    },
                    RAstPattern::Wildcard => {},
                    _ => semantic_error(&string("An enum's inner patterns must all be irrefutable!")),
                }
                i = i + 1;
            }
            enum_type
        },
    };
    semantic_expect_type_match(semantic, &pattern_type, scrutinee_match_type(&scrutinee));
}

// -----------------------------------------------------------------
// ---------------------- Code Generation --------------------------
// -----------------------------------------------------------------

/// Type that encapsulates the mutable state during LLVM-IR code generation from an AST.
enum Codegen {
    /// llvm code, current function, SSA counter, local symbol tables, generic, cached enum sizes
    Gen(
        Code,
        String,
        Counter,
        StringMapStack<STPair>,
        Generic,
        StringMap<usize>,
    ),
}

/// Type that encapsulates the immutable information needed by the code generator.
enum ICodegen {
    /// abstract syntax tree, global symbol table
    Static(RAst, StringMap<Item>),
}

/// Type that keeps track of counters to avoid name collisions.
enum Counter {
    /// Local variable counter, global string counter
    Counter(usize, usize),
}

fn iCodegenStatic_new(ast: RAst, items: StringMap<Item>) -> ICodegen {
    ICodegen::Static(ast, items)
}

fn iCodegen_globals(ICodegen::Static(_, items): &ICodegen) -> &StringMap<Item> {
    items
}

fn iCodegen_ast_items(ICodegen::Static(RAst::Language(items), _): &ICodegen) -> &Vec<RAstItem> {
    items
}

/// Lookup a global item (function or enum).
fn iCodegen_search_global<'a>(codegen: &'a ICodegen, name: &String) -> Option<&'a Item> {
    stringMap_get::<Item>(iCodegen_globals(codegen), name)
}

fn iCodegen_get_enum_discriminator(icg: &ICodegen, name: &String, variant: &String) -> usize {
    match iCodegen_search_global(icg, name) {
        Option::Some(item) => match item {
            Item::Enum(RAstEnum::Enum(_, variants, _)) => variants_get_discriminator(variants, variant),
            _ => 0, // assume this case does not occur
        },
        _ => 0, // assume this case does not occur
    }
}

fn iCodegen_get_enum_variant_fields<'a>(
    icg: &'a ICodegen,
    name: &String,
    variant: &String,
) -> Option<&'a Vec<RType>> {
    match iCodegen_search_global(icg, name) {
        Option::Some(item) => match item {
            Item::Enum(RAstEnum::Enum(_, variants, _)) => rAstEnum_variant_fields(variants, variant),
            _ => Option::<&Vec<RType>>::None,
        },
        _ => Option::<&Vec<RType>>::None,
    }
}

fn codegen_new() -> Codegen {
    let locals: StringMapStack<STPair> = stringMapStack_new::<STPair>();
    let counter: Counter = Counter::Counter(0, 0);
    let sizes: StringMap<usize> = stringMap_new::<usize>();
    Codegen::Gen(code_new(), string_new(), counter, locals, generic_new(), sizes)
}

/// Get a shared reference to the code.
fn codegen_code(Codegen::Gen(code, _, _, _, _, _): &Codegen) -> &Code {
    code
}

/// Get a mutable reference to the code.
fn codegen_code_mut(Codegen::Gen(code, _, _, _, _, _): &mut Codegen) -> &mut Code {
    code
}

/// Sets the current function's name to the given name.
fn codegen_set_current_function(Codegen::Gen(_, current_function, _, _, _, _): &mut Codegen, func: String) {
    *current_function = func;
}

fn codegen_current_function(Codegen::Gen(_, func, _, _, _, _): &Codegen) -> &String {
    func
}

/// Return true if the current function is the main function.
fn codegen_is_main(Codegen::Gen(_, function, _, _, _, _): &Codegen) -> bool {
    str_eq(function, "main")
}

/// Push a new empty scope onto the stack.
fn codegen_push_scope(Codegen::Gen(_, _, _, stack, _, _): &mut Codegen) {
    stringMapStack_push_empty::<STPair>(stack);
}

/// Pop the last pushed scope.
fn codegen_pop_scope(Codegen::Gen(_, _, _, stack, _, _): &mut Codegen) -> bool {
    stringMapStack_pop::<STPair>(stack)
}

/// Insert one variable slot into the current scope.
fn codegen_scope_insert(codegen: &mut Codegen, name: &String, ty: RType, pointer_name: String) {
    let Codegen::Gen(_, _, _, stack, _, _): &mut Codegen = codegen;
    stringMapStack_insert::<STPair>(stack, name, STPair::ST(pointer_name, ty));
}

/// Lookup variable slot information.
fn codegen_scope_lookup(Codegen::Gen(_, _, _, stack, _, _): &Codegen, name: &String) -> STPair {
    match stringMapStack_get::<STPair>(stack, name) {
        Option::Some(variable) => stPair_clone(variable),
        Option::None => stPair_unreachable(), // semantic analysis makes this impossible
    }
}

/// Get the current value of the SSA numbering scheme.
fn codegen_ssa_counter(Codegen::Gen(_, _, Counter::Counter(locals, _), _, _, _): &Codegen) -> usize {
    *locals
}

/// Increment the SSA numbering value by one.
fn codegen_increment_ssa_counter(Codegen::Gen(_, _, Counter::Counter(locals, _), _, _, _): &mut Codegen) {
    *locals = *locals + 1;
}

/// Reset the SSA numbering value to 0.
fn codegen_set_ssa_counter(codegen: &mut Codegen, value: usize) {
    let Codegen::Gen(_, _, Counter::Counter(counter, _), _, _, _): &mut Codegen = codegen;
    *counter = value;
}

fn codegen_check_enum_size(codegen: &mut Codegen, RAstEnum::Enum(name, _, _): &RAstEnum) -> Option<usize> {
    let Codegen::Gen(_, _, _, _, _, cached_sizes): &mut Codegen = codegen;
    match stringMap_get::<usize>(cached_sizes, name) {
        Option::Some(size) => Option::<usize>::Some(*size),
        Option::None => Option::<usize>::None,
    }
}

fn codegen_cache_enum_size(codegen: &mut Codegen, name: String, size: usize) {
    let Codegen::Gen(_, _, _, _, _, cached_sizes): &mut Codegen = codegen;
    stringMap_insert::<usize>(cached_sizes, name, size);
}

/// Instantiate the generic type parameter with the given type.
/// Returns false if the code for the given type has already been generated.
fn codegen_instantiate_generic(codegen: &mut Codegen, name: &String, ty: &RType) -> bool {
    let instance: RType = match ty {
        RType::Generic => match codegen_generic_instance(codegen, codegen_current_function(codegen)) {
            Option::Some(instance) => instance,
            _ => rType_clone(ty), // assume this does not occur
        },
        _ => rType_clone(ty),
    };
    let Codegen::Gen(_, _, _, _, generic, _): &mut Codegen = codegen;
    generic_instantiate(generic, name, &instance)
}

/// Get the type the generic type parameter is mapped to.
/// Returns None if there is no mapping for the generic type parameter.
fn codegen_generic_instance(Codegen::Gen(_, _, _, _, generic, _): &Codegen, name: &String) -> Option<RType> {
    generic_get_type(generic, name)
}

/// Undo the instantiation of the generic type parameter.
fn codegen_remove_generic_instance(Codegen::Gen(_, _, _, _, generic, _): &mut Codegen, name: &String) {
    generic_uninstantiate(generic, name);
}

/// Return true if the given item's generic parameter is instantiated.
fn codegen_is_instantiated(Codegen::Gen(_, _, _, _, generic, _): &Codegen, name: &String) -> bool {
    match generic_get_type(generic, name) {
        Option::Some(_) => true,
        _ => false,
    }
}

/// Get a unique virtual register name.
/// Returns `%t<internal counter>`.
fn codegen_next_register(codegen: &mut Codegen) -> String {
    let id: usize = codegen_ssa_counter(codegen);
    codegen_increment_ssa_counter(codegen);
    let mut name: String = string("%t");
    string_push_string(&mut name, &integer_to_string(id));
    name
}

/// Get a unique basic block label with a given suffix.
/// Returns `l<internal counter><suffix>`.
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
fn codegen_language(codegen: &mut Codegen, icg: &ICodegen) {
    let items: &Vec<RAstItem> = iCodegen_ast_items(icg);
    let mut i: usize = 0;
    let len: usize = vec_len::<RAstItem>(items);
    while i < len {
        let item: &RAstItem = vec_at::<RAstItem>(items, i);
        match item {
            RAstItem::ExternBlock(block) => codegen_extern_block(codegen, block),
            RAstItem::Function(function) => codegen_function(codegen, icg, function),
            _ => {}, // enum definitions do not generate code
        };
        i = i + 1;
    }
    codegen_builtin_functions(codegen, icg);
}

fn codegen_builtin_functions(codegen: &mut Codegen, icg: &ICodegen) {
    codegen_emit_line(codegen, string("define i64 @str..len(ptr %str) {\nentry:"));
    let len_ptr: String = codegen_emit_pointer_add(codegen, icg, &string("%str"), &RType::Usize, 1);
    let len: String = codegen_emit_load(codegen, &RType::Usize, &len_ptr);
    codegen_emit_ret_value(codegen, icg, &RType::Usize, &len);
    codegen_emit_function_end(codegen);

    let as_ptr: &str = "define ptr @str..as_ptr(ptr %str) {
entry:
  %p = load ptr, ptr %str
  ret ptr %p
}";
    codegen_emit_line(codegen, string(as_ptr));
}

/// Emit LLVM-IR for one extern block.
fn codegen_extern_block(codegen: &mut Codegen, functions: &Vec<RAstExternFn>) {
    let mut i: usize = 0;
    while i < vec_len::<RAstExternFn>(functions) {
        let function: &RAstExternFn = vec_at::<RAstExternFn>(functions, i);
        let RAstExternFn::Fn(name, parameters, return_type): &RAstExternFn = function;
        codegen_emit_declare(codegen, name, parameters, return_type);
        i = i + 1;
    }
}

/// Emit LLVM-IR for one function definition.
fn codegen_function(codegen: &mut Codegen, icg: &ICodegen, function: &RAstFunction) {
    let RAstFunction::Fn(is_generic, _, name, parameters, return_type, body): &RAstFunction = function;

    if and(*is_generic, not(codegen_is_instantiated(codegen, name))) {
        return; // do not generate generic functions unless type parameter is instantiated
    }

    codegen_set_current_function(codegen, string_clone(name));
    codegen_emit_fn_signature(codegen, name, return_type, parameters);

    codegen_push_scope(codegen);
    let mut i: usize = 0;
    while i < vec_len::<RAstVariable>(parameters) {
        let RAstVariable::Variable(pattern, param_type): &RAstVariable =
            vec_at::<RAstVariable>(parameters, i);
        let mut parameter: String = string("%");
        string_push_string(&mut parameter, &integer_to_string(i));

        codegen_bind_or_destructure(codegen, icg, pattern, &parameter, param_type);
        i = i + 1;
    }

    let STPair::ST(value_name, block_type): STPair = codegen_block(codegen, icg, body);
    match return_type {
        RType::Unit | RType::Never => {
            if codegen_is_main(codegen) {
                codegen_emit_ret_value(codegen, icg, &RType::Usize, &integer_to_string(0));
            } else {
                codegen_emit_ret_void(codegen);
            }
        },
        _ => {
            if rType_eq(&block_type, &RType::Never) {
                // there is not value, so dummy return value that is never reached anyway
                if rType_is_enum(codegen, return_type) {
                    codegen_emit_ret_void(codegen);
                } else if rType_is_pointer(return_type) {
                    let dummy_ptr: RType = RType::RawPointerMut(box_new::<RType>(RType::Usize));
                    let op: CastOperation = CastOperation::IntToPtr;
                    let val: String =
                        codegen_emit_cast(codegen, &op, &RType::Usize, &dummy_ptr, &string("0"));
                    codegen_emit_ret_value(codegen, icg, return_type, &val);
                } else {
                    codegen_emit_ret_value(codegen, icg, return_type, &string("0"));
                };
            } else {
                codegen_emit_ret_value(codegen, icg, return_type, &value_name);
            }
        },
    };
    codegen_pop_scope(codegen);
    codegen_emit_function_end(codegen);
}

/// Emit LLVM-IR for one block expression.
fn codegen_block(codegen: &mut Codegen, icg: &ICodegen, block: &RAstBlock) -> STPair {
    let RAstBlock::Block(statements, tail): &RAstBlock = block;
    codegen_push_scope(codegen);

    let mut i: usize = 0;
    let mut diverges: bool = false;
    while i < vec_len::<RAstStatement>(statements) {
        let statement: &RAstStatement = vec_at::<RAstStatement>(statements, i);
        match statement {
            RAstStatement::Let(variable, value) => {
                let diverging: bool = codegen_binding(codegen, icg, variable, box_deref::<RAstExpr>(value));
                diverges = or(diverges, diverging);
            },
            RAstStatement::Expression(expression) => {
                // expression is only used for its side-effects, so discard the result
                let STPair::ST(_, expr_type): STPair =
                    codegen_expression(codegen, icg, box_deref::<RAstExpr>(expression));
                diverges = or(diverges, rType_eq(&expr_type, &RType::Never));
            },
        };
        i = i + 1;
    }

    let STPair::ST(name, mut ty): STPair = match tail {
        Option::Some(expression) => codegen_expression(codegen, icg, box_deref::<RAstExpr>(expression)),
        Option::None => STPair::ST(string_new(), RType::Unit),
    };
    if diverges {
        ty = RType::Never; // block is diverging
    }
    codegen_pop_scope(codegen);
    STPair::ST(name, ty)
}

/// Emit LLVM-IR for one let binding.
/// Returns true if it diverges.
fn codegen_binding(codegen: &mut Codegen, icg: &ICodegen, variable: &RAstVariable, value: &RAstExpr) -> bool {
    let RAstVariable::Variable(pattern, binding_type): &RAstVariable = variable;
    let STPair::ST(rvalue_name, expr_type): STPair = codegen_expression(codegen, icg, value);
    if rType_has_value(&expr_type) {
        codegen_bind_or_destructure(codegen, icg, pattern, &rvalue_name, binding_type);
    }
    rType_eq(&expr_type, &RType::Never)
}

/// Emit LLVM-IR for one expression and return the resulting value/type pair.
fn codegen_expression(codegen: &mut Codegen, icg: &ICodegen, expression: &RAstExpr) -> STPair {
    match expression {
        RAstExpr::Return(returned) => codegen_return(codegen, icg, returned),
        RAstExpr::Assign(left, right) => codegen_assignment(
            codegen,
            icg,
            box_deref::<RAstExpr>(left),
            box_deref::<RAstExpr>(right),
        ),
        RAstExpr::Binary(operator, left, right) => codegen_binary_op(
            codegen,
            icg,
            operator,
            box_deref::<RAstExpr>(left),
            box_deref::<RAstExpr>(right),
        ),
        RAstExpr::Cast(value, to_type) => codegen_cast(codegen, icg, box_deref::<RAstExpr>(value), to_type),
        RAstExpr::Unary(operator, value) => {
            codegen_unary_op(codegen, icg, operator, box_deref::<RAstExpr>(value))
        },
        RAstExpr::Literal(literal) => codegen_literal(codegen, icg, literal),
        RAstExpr::Variable(name) => codegen_variable_use(codegen, name),
        RAstExpr::Path(path, arguments, generic) => codegen_path(codegen, icg, path, arguments, generic),
        RAstExpr::Block(_, block) => codegen_block(codegen, icg, block),
        RAstExpr::If(if_expression) => codegen_if(codegen, icg, if_expression),
        RAstExpr::While(condition, body) => {
            codegen_while(codegen, icg, box_deref::<RAstExpr>(condition), body)
        },
        RAstExpr::Match(value, arms) => codegen_match(codegen, icg, box_deref::<RAstExpr>(value), arms),
    }
}

/// Emit LLVM-IR for a return expression.
/// `return` always evaluates to type Never.
fn codegen_return(codegen: &mut Codegen, icg: &ICodegen, returned: &Option<Box<RAstExpr>>) -> STPair {
    match returned {
        Option::Some(expression) => {
            let STPair::ST(name, ty): STPair =
                codegen_expression(codegen, icg, box_deref::<RAstExpr>(expression));
            codegen_emit_ret_value(codegen, icg, &ty, &name);
        },
        Option::None => {
            if codegen_is_main(codegen) {
                codegen_emit_ret_value(codegen, icg, &RType::Usize, &string("0"));
            } else {
                codegen_emit_ret_void(codegen);
            }
        },
    };

    STPair::ST(string_new(), RType::Never)
}

/// Emit LLVM-IR for an assignment expression.
fn codegen_assignment(codegen: &mut Codegen, icg: &ICodegen, left: &RAstExpr, right: &RAstExpr) -> STPair {
    let STPair::ST(right_name, _): STPair = codegen_expression(codegen, icg, right);
    let STPair::ST(pointer_name, left_type): STPair = match left {
        RAstExpr::Variable(name) => codegen_scope_lookup(codegen, name),
        RAstExpr::Unary(op, value) => match op {
            RAstUnaryOp::Dereference => {
                let STPair::ST(pointer_name, pointer_type): STPair =
                    codegen_expression(codegen, icg, box_deref::<RAstExpr>(value));

                let inner: RType = match pointer_type {
                    RType::Reference(inner, _) => rType_clone(box_deref::<RType>(&inner)),
                    RType::RawPointerMut(inner) => rType_clone(box_deref::<RType>(&inner)),
                    _ => RType::Unit, // should be unreachable
                };
                STPair::ST(pointer_name, inner)
            },
            _ => stPair_unreachable(),
        },
        _ => stPair_unreachable(),
    };
    if rType_is_enum(codegen, &left_type) {
        codegen_emit_memcpy(codegen, icg, &pointer_name, &right_name, &left_type);
    } else {
        codegen_emit_store(codegen, &left_type, &right_name, &pointer_name);
    }
    STPair::ST(string_new(), RType::Unit)
}

/// Emit LLVM-IR for a binary expression.
fn codegen_binary_op(
    codegen: &mut Codegen,
    icg: &ICodegen,
    operator: &RAstBinaryOp,
    left: &RAstExpr,
    right: &RAstExpr,
) -> STPair {
    let STPair::ST(left_name, op_type): STPair = codegen_expression(codegen, icg, left);
    let STPair::ST(right_name, _): STPair = codegen_expression(codegen, icg, right);

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
fn codegen_cast(codegen: &mut Codegen, icg: &ICodegen, value: &RAstExpr, to_type: &RType) -> STPair {
    let STPair::ST(from_name, from_type): STPair = codegen_expression(codegen, icg, value);
    let to_type: RType = rType_clone(to_type);
    let op: CastOperation = castOperation_select_operation(&from_type, &to_type);
    let name: String = codegen_emit_cast(codegen, &op, &from_type, &to_type, &from_name);
    STPair::ST(name, to_type)
}

/// Emit LLVM-IR for a unary expression.
fn codegen_unary_op(
    codegen: &mut Codegen,
    icg: &ICodegen,
    operator: &RAstUnaryOp,
    value: &RAstExpr,
) -> STPair {
    match operator {
        RAstUnaryOp::Reference(mutable_ref) => match value {
            RAstExpr::Variable(name) => {
                let STPair::ST(pointer_name, ty): STPair = codegen_scope_lookup(codegen, name);
                STPair::ST(pointer_name, RType::Reference(box_new::<RType>(ty), *mutable_ref))
            },
            _ => {
                let STPair::ST(name, ty): STPair = codegen_expression(codegen, icg, value);
                if rType_is_enum(codegen, &ty) {
                    let new_type: RType = RType::Reference(box_new::<RType>(ty), *mutable_ref);
                    STPair::ST(name, new_type) // enum is already a pointer
                } else {
                    let reference: String = codegen_emit_alloca(codegen, icg, &ty, 1);
                    codegen_emit_store(codegen, &ty, &name, &reference);
                    let new_type: RType = RType::Reference(box_new::<RType>(ty), *mutable_ref);
                    STPair::ST(reference, new_type)
                }
            },
        },
        RAstUnaryOp::Dereference | RAstUnaryOp::DereferenceReference(_) => {
            let STPair::ST(mut name, ty): STPair = codegen_expression(codegen, icg, value);
            let inner_type: RType = match ty {
                RType::Reference(pointee, _) => rType_clone(box_deref::<RType>(&pointee)),
                RType::RawPointerMut(pointee) => rType_clone(box_deref::<RType>(&pointee)),
                _ => RType::Unit, // assume this case never occurs
            };
            match operator {
                RAstUnaryOp::Dereference => {
                    if not(rType_is_enum(codegen, &inner_type)) {
                        name = codegen_emit_load(codegen, &inner_type, &name);
                    }
                    STPair::ST(name, inner_type)
                },
                RAstUnaryOp::DereferenceReference(mutable) => {
                    STPair::ST(name, RType::Reference(box_new::<RType>(inner_type), *mutable))
                },
                _ => unreachable(),
            }
        },
    }
}

/// Emit LLVM-IR for a literal expression.
fn codegen_literal(codegen: &mut Codegen, icg: &ICodegen, literal: &RLiteral) -> STPair {
    match literal {
        RLiteral::Int(value) => STPair::ST(integer_to_string(*value), RType::Usize),
        RLiteral::Char(value) => STPair::ST(integer_to_string(*value as usize), RType::Char),
        RLiteral::Bool(value) => STPair::ST(integer_to_string(*value as usize), RType::Bool),
        RLiteral::String(value) => {
            let str_type: RType = RType::Enum(string("&str"), Option::<Box<RType>>::None);
            let string_ptr: String = codegen_emit_string(codegen, value);
            let struct_ptr: String = codegen_emit_alloca(codegen, icg, &str_type, 1);
            let string_ptr_type: RType = RType::RawPointerMut(box_new::<RType>(RType::U8));
            codegen_emit_store(codegen, &string_ptr_type, &string_ptr, &struct_ptr);
            let len_ptr: String = codegen_emit_pointer_add(codegen, icg, &struct_ptr, &string_ptr_type, 1);
            let length: String = integer_to_string(string_len(value));
            codegen_emit_store(codegen, &RType::Usize, &length, &len_ptr);
            STPair::ST(struct_ptr, str_type)
        },
    }
}

/// Emit LLVM-IR for a variable-use expression.
fn codegen_variable_use(codegen: &mut Codegen, variable_name: &String) -> STPair {
    let STPair::ST(pointer_name, ty): STPair = codegen_scope_lookup(codegen, variable_name);
    if not(rType_is_enum(codegen, &ty)) {
        let value_name: String = codegen_emit_load(codegen, &ty, &pointer_name);
        STPair::ST(value_name, ty)
    } else {
        STPair::ST(pointer_name, ty) // do not load pointer to enum
    }
}

/// Emit LLVM-IR for a path expression (either function call or enum instantiation).
fn codegen_path(
    codegen: &mut Codegen,
    icg: &ICodegen,
    path: &Vec<String>,
    values: &Vec<RAstExpr>,
    generic: &Option<RType>,
) -> STPair {
    let enum_type: &String = vec_at::<String>(path, 0);
    match iCodegen_search_global(icg, enum_type) {
        Option::Some(item) => match item {
            Item::Enum(RAstEnum::Enum(_, variants, _)) => {
                return codegen_enum(codegen, icg, path, variants, values, generic);
            },
            _ => {},
        },
        _ => {},
    }
    let function: String = rAstPath_to_string(path);
    codegen_call(codegen, icg, &function, values, generic)
}

fn codegen_enum(
    codegen: &mut Codegen,
    icg: &ICodegen,
    path: &Vec<String>,
    variants: &Vec<RAstVariant>,
    values: &Vec<RAstExpr>,
    generic: &Option<RType>,
) -> STPair {
    let enum_name: &String = vec_at::<String>(path, 0);
    let variant: &String = vec_at::<String>(path, 1);
    let tag: String = integer_to_string(variants_get_discriminator(variants, variant));

    let mapping: Option<Box<RType>> = match generic {
        Option::Some(instance) => Option::<Box<RType>>::Some(box_new::<RType>(rType_clone(instance))),
        _ => Option::<Box<RType>>::None,
    };
    let enum_type: RType = RType::Enum(string_clone(enum_name), mapping);
    let enum_ptr: String = codegen_emit_alloca(codegen, icg, &enum_type, 1);
    codegen_emit_store(codegen, &RType::Usize, &tag, &enum_ptr);

    let mut offset_ptr: String = codegen_emit_pointer_add(codegen, icg, &enum_ptr, &RType::Usize, 1);
    let mut i: usize = 0;
    while i < vec_len::<RAstExpr>(values) {
        let expression: &RAstExpr = vec_at::<RAstExpr>(values, i);
        let STPair::ST(register, ty): STPair = codegen_expression(codegen, icg, expression);
        if rType_is_enum(codegen, &ty) {
            codegen_emit_memcpy(codegen, icg, &offset_ptr, &register, &ty);
        } else {
            codegen_emit_store(codegen, &ty, &register, &offset_ptr);
        }
        if i < vec_len::<RAstExpr>(values) - 1 {
            offset_ptr = codegen_emit_pointer_add(codegen, icg, &offset_ptr, &ty, 1);
        } // only compute next address if there is another field
        i = i + 1;
    }
    STPair::ST(enum_ptr, enum_type)
}

fn codegen_call(
    codegen: &mut Codegen,
    icg: &ICodegen,
    name: &String,
    values: &Vec<RAstExpr>,
    generic: &Option<RType>,
) -> STPair {
    let mut return_type: RType = match iCodegen_search_global(icg, name) {
        Option::Some(item) => match item {
            Item::Function(return_type, _, _, _) => rType_clone(return_type),
            _ => RType::Unit, // assume this case never occurs
        },
        _ => RType::Unit, // assume this case never occurs
    };

    let generate_function: bool = match generic {
        Option::Some(instance) => {
            // If the current function is generic, then there is the possibility that `instance`
            // contains a generic type parameter. To handle this, first instantiate `instance` with
            // the generic instance of the current function, then instantiate the generic for the callee.
            // E.g.: If the current function has instance T -> usize and this function call uses
            // turbofish ::<Vec<StringMapEntry<T>>>, then before Vec<StringMapEntry<T>> can be used
            // as a generic instance, it has to be instantiated with the current function's instance
            // (in this case usize), so that the callee's instance becomes Vec<StringMapEntry<usize>>.
            let instance: RType = rType_instantiate_generic(
                instance,
                &codegen_generic_instance(codegen, codegen_current_function(codegen)),
                iCodegen_globals(icg),
            );
            let do_generate: bool = codegen_instantiate_generic(codegen, name, &instance);

            // size_of<T>() is inlined instead of generating a function
            if str_eq(name, "size_of") {
                let size: usize = rType_size(codegen, icg, &instance);
                codegen_remove_generic_instance(codegen, name);
                return STPair::ST(integer_to_string(size), RType::Usize);
            }

            return_type = rType_instantiate_generic(&return_type, generic, iCodegen_globals(icg));
            do_generate
        },
        _ => false,
    };

    let mut value_types: Vec<RType> = vec_with_capacity::<RType>(vec_len::<RAstExpr>(values) + 1);
    let mut value_names: Vec<String> = vec_with_capacity::<String>(vec_len::<RAstExpr>(values) + 1);

    if rType_is_enum(codegen, &return_type) {
        // add a special sret parameter which will hold the enum return value
        let dummy_ptr: RType = RType::RawPointerMut(box_new::<RType>(RType::Unit));
        vec_push::<RType>(&mut value_types, dummy_ptr);
        let sret: String = codegen_emit_alloca(codegen, icg, &return_type, 1);
        vec_push::<String>(&mut value_names, sret);
    }

    // compute arguments and push them onto vec
    let mut i: usize = 0;
    while i < vec_len::<RAstExpr>(values) {
        let value: &RAstExpr = vec_at::<RAstExpr>(values, i);
        let STPair::ST(value_name, value_type): STPair = codegen_expression(codegen, icg, value);
        vec_push::<RType>(&mut value_types, value_type);
        vec_push::<String>(&mut value_names, value_name);
        i = i + 1;
    }

    // emit the call and assign the result
    let result_name: String = if rType_is_enum(codegen, &return_type) {
        codegen_emit_call_side_effect(codegen, name, &RType::Unit, &value_types, &value_names);
        string_clone(vec_at::<String>(&value_names, 0)) // sret parameter holds result
    } else if rType_has_value(&return_type) {
        codegen_emit_call_assign(codegen, name, &return_type, &value_types, &value_names)
    } else {
        codegen_emit_call_side_effect(codegen, name, &return_type, &value_types, &value_names);
        string_new()
    };

    // generate callee if they are generic and were not generated yet
    let items: &Vec<RAstItem> = iCodegen_ast_items(icg);
    match generic {
        Option::Some(_) => {
            if generate_function {
                let mut i: usize = 0;
                while i < vec_len::<RAstItem>(items) {
                    match vec_at::<RAstItem>(items, i) {
                        RAstItem::Function(function) => {
                            let other: &String = rAstFunction_name(function);
                            if string_eq(name, other) {
                                // save the current function code generation index and SSA counter value
                                let caller: String = string_clone(codegen_current_function(codegen));
                                let fn_index: usize = code_current_function_index(codegen_code(codegen));
                                let ssa_counter: usize = codegen_ssa_counter(codegen);
                                codegen_set_ssa_counter(codegen, 0);

                                // recursively generate code for the generic function.
                                codegen_function(codegen, icg, function);

                                // restore the saved values
                                codegen_set_current_function(codegen, caller);
                                code_set_current_function_index(codegen_code_mut(codegen), fn_index);
                                codegen_set_ssa_counter(codegen, ssa_counter);
                                i = vec_len::<RAstItem>(items); // break
                            }
                        },
                        _ => {},
                    }
                    i = i + 1;
                }
            }
            codegen_remove_generic_instance(codegen, name)
        },
        _ => {},
    }
    STPair::ST(result_name, return_type)
}

/// Emit LLVM-IR for an if expression.
fn codegen_if(codegen: &mut Codegen, icg: &ICodegen, if_expression: &RAstIf) -> STPair {
    let RAstIf::If(condition, then_block, else_branch): &RAstIf = if_expression;

    let then_label: String = codegen_next_label(codegen, "if.then");
    let else_label: String = codegen_next_label(codegen, "if.else");
    let end_label: String = codegen_next_label(codegen, "if.end");

    let STPair::ST(cond, _): STPair = codegen_expression(codegen, icg, box_deref::<RAstExpr>(condition));

    // Allocate memory for potential result value, though size is still unknown.
    // In the event that the result type is unit, this instruction will be removed later.
    let result: String = codegen_emit_alloca(codegen, icg, &RType::Unit, 1);
    let alloca_idx: usize = codegen_code_last_index(codegen);

    codegen_emit_br_conditional(codegen, &cond, &then_label, &else_label);

    // start of the then block
    codegen_emit_label(codegen, &then_label);

    let STPair::ST(then_value, mut if_type): STPair = codegen_block(codegen, icg, then_block);

    if rType_is_enum(codegen, &if_type) {
        codegen_emit_memcpy(codegen, icg, &result, &then_value, &if_type);
    } else if rType_has_value(&if_type) {
        codegen_emit_store(codegen, &if_type, &then_value, &result);
    }

    // end of then block, so jump to the end
    codegen_emit_br(codegen, &end_label);

    // start of the else block
    codegen_emit_label(codegen, &else_label);

    match else_branch {
        Option::Some(else_branch) => {
            let STPair::ST(else_value, else_type): STPair = match else_branch {
                RAstElse::If(nested_if) => codegen_if(codegen, icg, box_deref::<RAstIf>(nested_if)),
                RAstElse::Block(block) => codegen_block(codegen, icg, block),
            };
            if rType_is_enum(codegen, &else_type) {
                codegen_emit_memcpy(codegen, icg, &result, &else_value, &else_type);
            } else if rType_has_value(&else_type) {
                codegen_emit_store(codegen, &else_type, &else_value, &result);
            }
            if_type = rType_coalesce(if_type, else_type);
        },
        _ => if_type = RType::Unit, // else is implicitly unit, so type of if must be unit
    }

    // end of else block, so jump to the end
    codegen_emit_br(codegen, &end_label);

    // start of the merge block
    codegen_emit_label(codegen, &end_label);

    // load and return the value if there is one
    let result: String = if rType_has_value(&if_type) {
        // now we know the type and thus the size to allocate on the stack
        codegen_fixup_alloca_type(codegen, icg, alloca_idx, &if_type);

        if not(rType_is_enum(codegen, &if_type)) {
            codegen_emit_load(codegen, &if_type, &result)
        } else {
            result
        }
    } else {
        codegen_fixup(codegen, alloca_idx, string_new()); // alloca was not needed
        string_new() // no value is returned, so some placeholder
    };

    STPair::ST(result, if_type)
}

/// Emit LLVM-IR for a while expression.
fn codegen_while(codegen: &mut Codegen, icg: &ICodegen, condition: &RAstExpr, body: &RAstBlock) -> STPair {
    let entry_label: String = codegen_next_label(codegen, "while.entry");
    let body_label: String = codegen_next_label(codegen, "while.body");
    let end_label: String = codegen_next_label(codegen, "while.end");

    // jump from current block to while-entry block
    codegen_emit_br(codegen, &entry_label);
    // start entry block
    codegen_emit_label(codegen, &entry_label);

    let STPair::ST(condition_name, _): STPair = codegen_expression(codegen, icg, condition);

    // conditionally execute body or skip body
    codegen_emit_br_conditional(codegen, &condition_name, &body_label, &end_label);

    // start body block
    codegen_emit_label(codegen, &body_label);

    codegen_block(codegen, icg, body);

    // jump back to entry to reevaluate condition
    codegen_emit_br(codegen, &entry_label);

    // start block of rest of instructions
    codegen_emit_label(codegen, &end_label);

    STPair::ST(string_new(), RType::Unit) // while always returns unit
}

fn codegen_match(codegen: &mut Codegen, icg: &ICodegen, scrutinee: &RAstExpr, arms: &Vec<RAstArm>) -> STPair {
    let STPair::ST(expr_name, expr_type): STPair = codegen_expression(codegen, icg, scrutinee);

    // SSA: Allocate memory for potential result value, though size is still unknown.
    let result: String = codegen_emit_alloca(codegen, icg, &RType::Unit, 1);
    let alloca_idx: usize = codegen_code_last_index(codegen);
    let mut return_type: RType = RType::Never; // start with bottom type and coalesce arm types for correct type
    let end_label: String = codegen_next_label(codegen, "match.end");

    let mut i: usize = 0;
    while i < vec_len::<RAstArm>(arms) {
        let arm: &RAstArm = vec_at::<RAstArm>(arms, i);
        let is_last_arm: bool = i == vec_len::<RAstArm>(arms) - 1;

        codegen_push_scope(codegen);
        let arm_type: RType = codegen_arm(
            codegen,
            icg,
            arm,
            is_last_arm,
            &expr_name,
            &expr_type,
            &result,
            &end_label,
        );
        codegen_pop_scope(codegen);

        return_type = rType_coalesce(return_type, arm_type);
        i = i + 1;
    }
    codegen_emit_label(codegen, &end_label); // start of merge block

    let result: String = if rType_has_value(&return_type) {
        // now we know the type and thus the size to allocate on the stack
        codegen_fixup_alloca_type(codegen, icg, alloca_idx, &return_type);

        if not(rType_is_enum(codegen, &return_type)) {
            codegen_emit_load(codegen, &return_type, &result)
        } else {
            result
        }
    } else {
        codegen_fixup(codegen, alloca_idx, string_new()); // alloca was not needed
        string_new() // some placeholder
    };
    STPair::ST(result, return_type)
}

/// Generate code for a single match arm.
fn codegen_arm(
    codegen: &mut Codegen,
    icg: &ICodegen,
    RAstArm::Arm(patterns, arm_expr): &RAstArm,
    is_last_arm: bool,
    expr_name: &String,
    expr_type: &RType,
    result: &String,
    end_label: &String,
) -> RType {
    let arm_label: String = codegen_next_label(codegen, "match.arm");
    let else_label: String = codegen_next_label(codegen, "match.else");

    let mut i: usize = 0;
    while i < vec_len::<RAstPattern>(patterns) {
        let pattern: &RAstPattern = vec_at::<RAstPattern>(patterns, i);
        let is_last_pattern: bool = i == vec_len::<RAstPattern>(patterns) - 1;
        let fail_label: &String = if is_last_pattern {
            &else_label // next arm
        } else {
            &codegen_next_label(codegen, "match.check") // next pattern
        };

        if not(is_last_arm) {
            codegen_arm_match(
                codegen, icg, pattern, expr_name, expr_type, &arm_label, fail_label,
            );
            if not(is_last_pattern) {
                codegen_emit_label(codegen, fail_label); // next pattern of arm
            }
        } // otherwise arm is executed unconditionally

        i = i + 1;
    }

    if not(is_last_arm) {
        codegen_emit_label(codegen, &arm_label); // start of arm body
    }
    // destructure only the first pattern, assuming there is only one (enforced by semantic analysis)
    let pattern: &RAstPattern = vec_at::<RAstPattern>(patterns, 0);
    codegen_bind_or_destructure(codegen, icg, pattern, expr_name, expr_type);

    let STPair::ST(arm_value, arm_type): STPair = codegen_expression(codegen, icg, arm_expr);
    if rType_is_enum(codegen, &arm_type) {
        codegen_emit_memcpy(codegen, icg, result, &arm_value, &arm_type);
    } else if rType_has_value(&arm_type) {
        codegen_emit_store(codegen, &arm_type, &arm_value, result);
    }

    codegen_emit_br(codegen, end_label); // arm evaluated, so jump to end
    if not(is_last_arm) {
        codegen_emit_label(codegen, &else_label); // start label for next arm condition
    }
    arm_type
}

/// Generate code that decides whether an arm is a match or not.
fn codegen_arm_match(
    codegen: &mut Codegen,
    icg: &ICodegen,
    pattern: &RAstPattern,
    expr_name: &String,
    expr_type: &RType,
    arm_label: &String,
    fail_label: &String,
) {
    let eq: RAstComparisonOp = RAstComparisonOp::Eq;
    let scrutinee: Scrutinee = scrutinee_from_type(expr_type);
    let is_enum_reference: bool = rType_is_enum(codegen, scrutinee_match_type(&scrutinee));
    let expr_name: &String = if and(scrutinee_is_reference(&scrutinee), not(is_enum_reference)) {
        &codegen_emit_load(codegen, scrutinee_match_type(&scrutinee), expr_name)
    } else {
        expr_name
    };
    let expr_type: &RType = scrutinee_match_type(&scrutinee);

    match pattern {
        RAstPattern::Literal(literal) => {
            let value: String = integer_to_string(rAstPatternLiteral_value(literal));
            let cond: String = codegen_emit_icmp(codegen, &eq, expr_type, expr_name, &value);
            codegen_emit_br_conditional(codegen, &cond, arm_label, fail_label);
        },
        RAstPattern::EnumVariant(name, variant, _) => {
            let tag: usize = iCodegen_get_enum_discriminator(icg, name, variant);
            let tag: String = integer_to_string(tag);
            let expr_tag: String = codegen_emit_load(codegen, &RType::Usize, expr_name);
            let cond: String = codegen_emit_icmp(codegen, &eq, &RType::Usize, &tag, &expr_tag);
            codegen_emit_br_conditional(codegen, &cond, arm_label, fail_label);
        },
        _ => {}, // catch-all patterns do not branch conditionally
    }
}

/// Generate code to bind/destructure matched values to names.
fn codegen_bind_or_destructure(
    codegen: &mut Codegen,
    icg: &ICodegen,
    pattern: &RAstPattern,
    expr_name: &String,
    expr_type: &RType,
) {
    match pattern {
        RAstPattern::Identifier(_, identifier) => {
            let register: String = if not(rType_is_enum(codegen, expr_type)) {
                let ptr: String = codegen_emit_alloca(codegen, icg, expr_type, 1);
                codegen_emit_store(codegen, expr_type, expr_name, &ptr);
                ptr
            } else {
                string_clone(expr_name) // enums are moved, so no need to copy
            };
            codegen_scope_insert(codegen, identifier, rType_clone(expr_type), register);
        },
        RAstPattern::EnumVariant(name, variant, inner_patterns) => {
            let scrutinee: Scrutinee = scrutinee_from_type(expr_type);
            // assume all inner patterns are irrefutable
            if vec_len::<RAstPattern>(inner_patterns) > 0 {
                let is_enum_reference: bool = rType_is_enum(codegen, scrutinee_match_type(&scrutinee));
                let expr_name: &String = if and(scrutinee_is_reference(&scrutinee), not(is_enum_reference)) {
                    &codegen_emit_load(codegen, scrutinee_match_type(&scrutinee), expr_name)
                } else {
                    expr_name
                };
                codegen_enum_destructure(codegen, icg, name, variant, expr_name, inner_patterns, &scrutinee);
            }
        },
        _ => {}, // do not destructure or bind values for literal or wildcard
    }
}

/// Generate code to destructure an enum with at least one field.
fn codegen_enum_destructure(
    codegen: &mut Codegen,
    icg: &ICodegen,
    name: &String,
    variant: &String,
    initial_offset: &String,
    patterns: &Vec<RAstPattern>,
    scrutinee: &Scrutinee,
) {
    let generic: Option<RType> = rType_extract_enum_generic(scrutinee_match_type(scrutinee));
    let fields: &Vec<RType> = match iCodegen_get_enum_variant_fields(icg, name, variant) {
        Option::Some(fields) => fields,
        _ => return, // assume this case does not happen
    };
    let mut offset: String = codegen_emit_pointer_add(codegen, icg, initial_offset, &RType::Usize, 1); // skip discriminant
    let mut i: usize = 0;
    while i < vec_len::<RType>(fields) {
        let ty: RType =
            rType_instantiate_generic(vec_at::<RType>(fields, i), &generic, iCodegen_globals(icg));
        let pattern: &RAstPattern = vec_at::<RAstPattern>(patterns, i);
        match pattern {
            RAstPattern::Identifier(_, name) => {
                let variable_type: RType = scrutinee_inherit_borrow(scrutinee, &ty);
                let pointer: String = codegen_emit_alloca(codegen, icg, &variable_type, 1);
                if scrutinee_is_reference(scrutinee) {
                    codegen_emit_store(codegen, &variable_type, &offset, &pointer);
                } else if rType_is_enum(codegen, &variable_type) {
                    codegen_emit_memcpy(codegen, icg, &pointer, &offset, &variable_type);
                } else {
                    let field_value: String = codegen_emit_load(codegen, &variable_type, &offset);
                    codegen_emit_store(codegen, &variable_type, &field_value, &pointer);
                }
                codegen_scope_insert(codegen, name, variable_type, pointer);
            },
            RAstPattern::EnumVariant(name, variant, patterns) => {
                if vec_len::<RAstPattern>(patterns) > 0 {
                    if rType_is_reference(&ty) {
                        // This enum is actually a reference to an enum. This means this field does
                        // not store an entire enum, but only a pointer to an enum.
                        let enum_ptr: String = codegen_emit_load(codegen, &ty, &offset);
                        let scrutinee: Scrutinee = scrutinee_from_type(&ty);
                        codegen_enum_destructure(
                            codegen, icg, name, variant, &enum_ptr, patterns, &scrutinee,
                        );
                    } else {
                        codegen_enum_destructure(codegen, icg, name, variant, &offset, patterns, scrutinee);
                    }
                }
            },
            _ => {}, // assume otherwise it is wildcard (irrefutable pattern)
        }
        if i < vec_len::<RAstPattern>(patterns) - 1 {
            offset = codegen_emit_pointer_add(codegen, icg, &offset, &ty, 1);
        }
        i = i + 1;
    }
}

// ---------------------------- Generics ---------------------------------

/// Type that tracks generic mappings and already generated functions.
enum Generic {
    /// type mappings, generated functions
    Manager(StringMap<RType>, StringMap<Vec<RType>>),
}

fn generic_new() -> Generic {
    Generic::Manager(stringMap_new::<RType>(), stringMap_new::<Vec<RType>>())
}

/// Instantiate the generic type parameter by creating a mapping for the item's type parameter.
/// Returns false (but still inserts) if the code for the given type has already been generated.
fn generic_instantiate(generic: &mut Generic, name: &String, ty: &RType) -> bool {
    let Generic::Manager(mappings, generated): &mut Generic = generic;
    match stringMap_get_mut::<Vec<RType>>(generated, name) {
        Option::Some(instances) => {
            let mut i: usize = 0;
            while i < vec_len::<RType>(instances) {
                if rType_eq(ty, vec_at::<RType>(instances, i)) {
                    stringMap_insert_or_update::<RType>(mappings, name, rType_clone(ty));
                    return false; // already generated code for the given type
                }
                i = i + 1;
            }
            vec_push::<RType>(instances, rType_clone(ty));
        },
        _ => {
            let mut instances: Vec<RType> = vec_new::<RType>();
            vec_push::<RType>(&mut instances, rType_clone(ty));
            stringMap_insert_or_update::<Vec<RType>>(generated, name, instances);
        },
    }
    stringMap_insert_or_update::<RType>(mappings, name, rType_clone(ty));
    true
}

/// Get the generic parameter's actual type for a given item.
fn generic_get_type(Generic::Manager(mappings, _): &Generic, name: &String) -> Option<RType> {
    match stringMap_get::<RType>(mappings, name) {
        Option::Some(ty) => Option::<RType>::Some(rType_clone(ty)),
        _ => Option::<RType>::None,
    }
}

fn generic_uninstantiate(Generic::Manager(mappings, _): &mut Generic, name: &String) {
    if not(stringMap_remove::<RType>(mappings, name)) {
        panic("unexpected removal of non-existent generic type parameter mapping")
    }
}

// ---------------------------- Code Emission ---------------------------------

/// The emitted LLVM-IR code.
enum Code {
    Code(Vec<Vec<String>>, Vec<String>, usize),
}

fn code_new() -> Code {
    Code::Code(vec_new::<Vec<String>>(), vec_new::<String>(), 0)
}

/// Get a shared reference to the code generated for the current function.
fn code_current_function(Code::Code(functions, _, idx): &Code) -> &Vec<String> {
    vec_at::<Vec<String>>(functions, *idx)
}
/// Get a mutable reference to the code generated for the current function.
fn code_current_function_mut(Code::Code(functions, _, idx): &mut Code) -> &mut Vec<String> {
    vec_at_mut::<Vec<String>>(functions, *idx)
}

/// Start code generation for a new function.
fn code_start_new_function(Code::Code(functions, _, idx): &mut Code) {
    vec_push::<Vec<String>>(functions, vec_new::<String>());
    *idx = vec_len::<Vec<String>>(functions) - 1;
}

/// Get the currently generated function's index.
fn code_current_function_index(Code::Code(_, _, idx): &Code) -> usize {
    *idx
}

/// Set the index of the function to generate code for.
fn code_set_current_function_index(Code::Code(_, _, old_idx): &mut Code, idx: usize) {
    *old_idx = idx;
}

/// Get the line index of the last emitted line.
fn codegen_code_last_index(codegen: &Codegen) -> usize {
    vec_len::<String>(code_current_function(codegen_code(codegen))) - 1
}

/// Fixup the emitted line at index `i` by replacing it with `line`.
fn codegen_fixup(codegen: &mut Codegen, i: usize, line: String) {
    vec_set::<String>(code_current_function_mut(codegen_code_mut(codegen)), i, line);
}

/// Get the emitted LLVM-IR from Codegen.
fn codegen_into_llvm(Codegen::Gen(Code::Code(functions, strings, _), _, _, _, _, _): Codegen) -> String {
    let mut code: String = string_new();

    let mut i: usize = 0;
    while i < vec_len::<Vec<String>>(&functions) {
        let function: &Vec<String> = vec_at::<Vec<String>>(&functions, i);
        let mut j: usize = 0;
        while j < vec_len::<String>(function) {
            let line: &String = vec_at::<String>(function, j);
            if string_len(line) > 0 {
                string_push_string(&mut code, line);
                string_push(&mut code, '\n');
            }
            j = j + 1;
        }
        string_push(&mut code, '\n');
        i = i + 1;
    }
    i = 0;
    while i < vec_len::<String>(&strings) {
        let line: &String = vec_at::<String>(&strings, i);
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
    vec_push::<String>(code_current_function_mut(codegen_code_mut(codegen)), line);
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
    string_push_string(&mut line, &rType_to_llvm_name(codegen, ty));
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
    string_push_string(&mut line, &rType_to_llvm_name(codegen, ty));
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
/// Enums are returned using sret.
fn codegen_emit_ret_value(codegen: &mut Codegen, icg: &ICodegen, ty: &RType, value: &String) {
    if not(rType_is_enum(codegen, ty)) {
        let mut line: String = string_new();
        string_push_str(&mut line, "  ");
        string_push_str(&mut line, "ret ");
        string_push_string(&mut line, &rType_to_llvm_name(codegen, ty));
        string_push(&mut line, ' ');
        string_push_string(&mut line, value);
        codegen_emit_line(codegen, line);
    } else {
        // do not directly return the enum, instead copy to sret parameter
        codegen_emit_memcpy(codegen, icg, &string("%sret"), value, ty);
        codegen_emit_line(codegen, string("  ret void"));
    }
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
/// br i1 <cond>, label %<then>, label %<else_br>
/// ```
fn codegen_emit_br_conditional(code: &mut Codegen, cond: &String, then: &String, else_br: &String) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  br i1 ");
    string_push_string(&mut line, cond);
    string_push_str(&mut line, ", label %");
    string_push_string(&mut line, then);
    string_push_str(&mut line, ", label %");
    string_push_string(&mut line, else_br);
    codegen_emit_line(code, line);
}

/// Emit a cast instruction.
/// ```llvm
/// %<name> = <op> <from> <x> to <to>
/// ```
/// Returns `%<name>`.
fn codegen_emit_cast(code: &mut Codegen, op: &CastOperation, from: &RType, to: &RType, x: &String) -> String {
    let op: &str = match op {
        CastOperation::ZeroExtend => "zext",
        CastOperation::Truncate => "trunc",
        CastOperation::IntToPtr => "inttoptr",
        CastOperation::PtrToInt => "ptrtoint",
        CastOperation::Invalid => "invalid", // assume this case never occurs
        CastOperation::None => return string_clone(x),
    };
    let name: String = codegen_next_register(code);
    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &name);
    string_push_str(&mut line, " = ");
    string_push_str(&mut line, op);
    string_push(&mut line, ' ');
    string_push_string(&mut line, &rType_to_llvm_name(code, from));
    string_push(&mut line, ' ');
    string_push_string(&mut line, x);
    string_push_str(&mut line, " to ");
    string_push_string(&mut line, &rType_to_llvm_name(code, to));
    codegen_emit_line(code, line);
    name
}

/// Emit an allocate instruction for a given Rust type.
/// ```llvm
/// %<name> = alloca <ty>
/// ```
/// Returns `%<name>`.
fn codegen_emit_alloca(codegen: &mut Codegen, icg: &ICodegen, ty: &RType, count: usize) -> String {
    if not(rType_is_enum(codegen, ty)) {
        let name: String = codegen_next_register(codegen);
        let mut line: String = string_new();
        string_push_str(&mut line, "  ");
        string_push_string(&mut line, &name);
        string_push_str(&mut line, " = alloca ");
        string_push_string(&mut line, &rType_to_llvm_name(codegen, ty));
        string_push_str(&mut line, ", i64 ");
        string_push_string(&mut line, &integer_to_string(count));
        codegen_emit_line(codegen, line);
        name
    } else {
        let size: usize = rType_size(codegen, icg, ty) * count;
        if size % 8 != 0 {
            panic("enum size should be aligned to 8 bytes");
        }
        codegen_emit_alloca(codegen, icg, &RType::Usize, size / 8)
    }
}

/// Emit a store instruction for a given Rust type.
/// ```llvm
/// store <ty> <value>, ptr <pointer>
/// ```
fn codegen_emit_store(codegen: &mut Codegen, ty: &RType, value: &String, pointer: &String) {
    let mut line: String = string_new();
    string_push_str(&mut line, "  store ");
    string_push_string(&mut line, &rType_to_llvm_name(codegen, ty));
    string_push(&mut line, ' ');
    string_push_string(&mut line, value);
    string_push(&mut line, ',');
    string_push_str(&mut line, " ptr ");
    string_push_string(&mut line, pointer);
    codegen_emit_line(codegen, line);
}

/// Emit a load instruction for a given Rust type.
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
    string_push_string(&mut line, &rType_to_llvm_name(codegen, ty));
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
fn codegen_emit_pointer_add(
    codegen: &mut Codegen,
    icg: &ICodegen,
    pointer: &String,
    ty: &RType,
    index: usize,
) -> String {
    let ptr_type: RType = RType::RawPointerMut(box_new::<RType>(RType::Unit)); // dummy type to use `ptr` type
    let addition: RAstArithmeticOp = RAstArithmeticOp::Add;
    let offset: String = integer_to_string(index * rType_size(codegen, icg, ty));

    let ptrtoint: CastOperation = CastOperation::PtrToInt;
    let inttoptr: CastOperation = CastOperation::IntToPtr;
    let t0: String = codegen_emit_cast(codegen, &ptrtoint, &ptr_type, &RType::Usize, pointer);
    let t1: String = codegen_emit_binary(codegen, &addition, &RType::Usize, &t0, &offset);
    let name: String = codegen_emit_cast(codegen, &inttoptr, &RType::Usize, &ptr_type, &t1);
    name
}

/// Emit a memcpy call which copies `size_of::<ty>()` bytes from `src` to `dest`.
fn codegen_emit_memcpy(codegen: &mut Codegen, icg: &ICodegen, dest: &String, src: &String, ty: &RType) {
    let size: usize = rType_size(codegen, icg, ty);
    let mut line: String = string("  call void @llvm.memcpy.p0.p0.i64(ptr ");
    string_push_string(&mut line, dest);
    string_push_str(&mut line, ", ptr ");
    string_push_string(&mut line, src);
    string_push_str(&mut line, ", i64 ");
    string_push_string(&mut line, &integer_to_string(size));
    string_push_str(&mut line, ", i1 0)"); // isvolatile parameter is not supported by autos
    codegen_emit_line(codegen, line);
}

/// Emit a call instruction that assigns the return value to a register.
/// ```llvm
/// %<register> = call <return_type> @<callee>(<arg_type> <arg>, ...)
/// ```
/// Returns `%<register>`.
fn codegen_emit_call_assign(
    codegen: &mut Codegen,
    callee: &String,
    return_type: &RType,
    arg_types: &Vec<RType>,
    args: &Vec<String>,
) -> String {
    let register: String = codegen_next_register(codegen);
    let mut line: String = string_new();
    string_push_str(&mut line, "  ");
    string_push_string(&mut line, &register);
    string_push_str(&mut line, " = ");
    let call: String = codegen_construct_call(codegen, callee, return_type, arg_types, args);
    string_push_string(&mut line, &call);
    codegen_emit_line(codegen, line);
    register
}

/// Emit a call instruction without assigning its (potential) return value.
/// ```llvm
/// call <return_type> @<callee>(<arg_type> <arg>, ...)
/// ```
fn codegen_emit_call_side_effect(
    codegen: &mut Codegen,
    callee: &String,
    return_type: &RType,
    arg_types: &Vec<RType>,
    args: &Vec<String>,
) {
    let mut line: String = string("  ");
    let call: String = codegen_construct_call(codegen, callee, return_type, arg_types, args);
    string_push_string(&mut line, &call);
    codegen_emit_line(codegen, line);
}

/// Mangles a function name by replacing invalid characters with valid LLVM identifier
/// characters and appending `.<type>` if the function is generic.
fn codegen_mangle_name(codegen: &Codegen, callee: &String) -> String {
    let mut name: String = string_clone(callee);
    match codegen_generic_instance(codegen, callee) {
        Option::Some(ty) => {
            string_push(&mut name, '.'); // mangle for each instantiation
            string_push_string(&mut name, &rType_to_string(&ty));
        },
        _ => {},
    }
    string_replace_all(&mut name, ":<>& *", "...$_.");
    name
}

/// Construct a call instruction and return it.
/// ```llvm
/// call <ty> @<name>(<type> <arg>, ...)
/// ```
fn codegen_construct_call(
    codegen: &mut Codegen,
    callee: &String,
    return_type: &RType,
    arg_types: &Vec<RType>,
    args: &Vec<String>,
) -> String {
    let mut line: String = string("call ");
    string_push_string(&mut line, &rType_to_llvm_name(codegen, return_type));
    string_push_str(&mut line, " @");
    let name: String = codegen_mangle_name(codegen, callee);
    string_push_string(&mut line, &name);
    string_push(&mut line, '(');

    let mut i: usize = 0;
    let len: usize = vec_len::<RType>(arg_types);
    while i < len {
        let argument_type: &RType = vec_at::<RType>(arg_types, i);
        let argument_value: &String = vec_at::<String>(args, i);
        if rType_is_enum(codegen, argument_type) {
            string_push_str(&mut line, "ptr"); // pass enums by reference
        } else {
            string_push_string(&mut line, &rType_to_llvm_name(codegen, argument_type));
        }
        string_push(&mut line, ' ');
        string_push_string(&mut line, argument_value);

        i = i + 1;
        if i < len {
            string_push_str(&mut line, ", ");
        }
    }
    string_push_str(&mut line, ")");
    line
}

/// Emit a function header with an entry label.
/// ```llvm
/// define <return_type> @<fn_name>(<param_type> %<param_name>, ...) {
/// entry:
/// ```
fn codegen_emit_fn_signature(
    codegen: &mut Codegen,
    fn_name: &String,
    return_type: &RType,
    parameters: &Vec<RAstVariable>,
) {
    code_start_new_function(codegen_code_mut(codegen)); // Start code generation for new function

    let return_type_name: &String = if rType_is_enum(codegen, return_type) {
        &string("void") // enums are returned using sret
    } else if and(str_eq(fn_name, "main"), rType_eq(return_type, &RType::Unit)) {
        &string("i64") // fn main() -> () should return i64 0 as exit code
    } else {
        &rType_to_llvm_name(codegen, return_type)
    };

    let mut line: String = string_new();
    string_push_str(&mut line, "define ");
    string_push_string(&mut line, return_type_name);
    string_push_str(&mut line, " @");
    let name: String = codegen_mangle_name(codegen, fn_name);
    string_push_string(&mut line, &name);
    string_push_str(&mut line, "(");

    // add hidden first parameter sret if this function returns enum
    if rType_is_enum(codegen, return_type) {
        string_push_str(&mut line, "ptr %sret");
        if vec_len::<RAstVariable>(parameters) > 0 {
            string_push(&mut line, ',');
        }
    }

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstVariable>(parameters);
    while i < len {
        let RAstVariable::Variable(_, parameter_type): &RAstVariable = vec_at::<RAstVariable>(parameters, i);
        if rType_is_enum(codegen, parameter_type) {
            string_push_str(&mut line, "ptr"); // pass enums by reference
        } else {
            string_push_string(&mut line, &rType_to_llvm_name(codegen, parameter_type));
        }
        string_push_str(&mut line, " %");
        string_push_string(&mut line, &integer_to_string(i));

        i = i + 1;
        if i < len {
            string_push_str(&mut line, ", ");
        }
    }
    string_push_str(&mut line, ") {\nentry:");

    codegen_emit_line(codegen, line);
}

/// Emit the end of a function and reset the numbering scheme.
fn codegen_emit_function_end(codegen: &mut Codegen) {
    codegen_emit_line(codegen, string("}"));
    codegen_set_ssa_counter(codegen, 0);
}

/// Emit an LLVM `declare` for an extern function.
/// ```llvm
/// declare <return_type> @<fn_name>(<param_type>, ...)
/// ```
/// Does not return a value.
fn codegen_emit_declare(codegen: &mut Codegen, name: &String, params: &Vec<RAstVariable>, return_ty: &RType) {
    let mut line: String = string_new();
    string_push_str(&mut line, "declare ");
    string_push_string(&mut line, &rType_to_llvm_name(codegen, return_ty));
    string_push_str(&mut line, " @");
    string_push_string(&mut line, name);
    string_push_str(&mut line, "(");

    let mut i: usize = 0;
    let len: usize = vec_len::<RAstVariable>(params);
    while i < len {
        let RAstVariable::Variable(_, parameter_type): &RAstVariable = vec_at::<RAstVariable>(params, i);

        string_push_string(&mut line, &rType_to_llvm_name(codegen, parameter_type));

        i = i + 1;
        if i < len {
            string_push_str(&mut line, ", ");
        }
    }
    string_push_str(&mut line, ")");

    codegen_emit_line(codegen, line);
}

/// Emit a string allocated in global data.
/// ```llvm
/// @<name> = constant [<length> x i8] c"<value>"
/// ```
/// Returns `%<name>`.
fn codegen_emit_string(
    Codegen::Gen(Code::Code(_, strings, _), _, Counter::Counter(_, counter), _, _, _): &mut Codegen,
    value: &String,
) -> String {
    let mut name: String = string("@str");
    string_push_string(&mut name, &integer_to_string(*counter));
    *counter = *counter + 1;
    let mut line: String = string_clone(&name);
    string_push_str(&mut line, " = constant [");
    string_push_string(&mut line, &integer_to_string(string_len(value)));
    string_push_str(&mut line, " x i8] c\"");
    let mut i: usize = 0;
    while i < string_len(value) {
        let c: char = string_at(value, i);
        if or(c < ' ', or(c as usize >= 128, or(c == '"', c == '\\'))) {
            string_push(&mut line, '\\');
            let byte: String = string_integer_extend(&integer_to_string_base(c as usize, 16), 2);
            string_push_string(&mut line, &byte);
        } else {
            string_push(&mut line, c);
        }
        i = i + 1;
    }
    string_push(&mut line, '"');
    vec_push::<String>(strings, line);
    name
}

/// Fixup a previously emitted alloca instruction without changing the destination register.
/// Always modifies the alloca to only allocate one element.
// TODO: assumes a lot about the emitted LLVM-IR, make this more robust.
fn codegen_fixup_alloca_type(codegen: &mut Codegen, icg: &ICodegen, index: usize, new_type: &RType) {
    let lines: &mut Vec<String> = code_current_function_mut(codegen_code_mut(codegen));

    let old_alloca: &String = vec_at::<String>(lines, index);
    let mut new_alloca: String = string_with_capacity(5);

    let mut space_count: usize = 0;
    let mut i: usize = 0;

    // "  <register> = alloca " has 5 spaces.
    while space_count < 5 {
        let c: char = string_at(old_alloca, i);

        if is_whitespace(c) {
            space_count = space_count + 1;
        }

        string_push(&mut new_alloca, c);
        i = i + 1;
    }

    if rType_is_enum(codegen, new_type) {
        let size: usize = rType_size(codegen, icg, new_type);
        if size % 8 != 0 {
            panic("fixed up enum size should be aligned to 8 bytes");
        }
        string_push_str(&mut new_alloca, "i64, i64 ");
        string_push_string(&mut new_alloca, &integer_to_string(size / 8));
    } else {
        string_push_string(&mut new_alloca, &rType_to_llvm_name(codegen, new_type));
        string_push_str(&mut new_alloca, ", i64 1");
    }

    codegen_fixup(codegen, index, new_alloca);
}

// -----------------------------------------------------------------
// -----------------------------------------------------------------
// ---------------------- LLLVM-IR Emulator ------------------------
// -----------------------------------------------------------------
// -----------------------------------------------------------------

// -----------------------------------------------------------------
// ---------------------- Lexical Analysis -------------------------
// -----------------------------------------------------------------

/// Tokens produced by the LLVM lexer.
enum LToken {
    Define,             // "define"
    Declare,            // "declare"
    Ret,                // "ret"
    IntToPtr,           // "inttoptr"
    PtrToInt,           // "ptrtoint"
    Br,                 // "br"
    Label,              // "label"
    Add,                // "add"
    Sub,                // "sub"
    Mul,                // "mul"
    Udiv,               // "udiv"
    Urem,               // "urem"
    Icmp,               // "icmp"
    Zext,               // "zext"
    Trunc,              // "trunc"
    Alloca,             // "alloca"
    Store,              // "store"
    Load,               // "load"
    To,                 // "to"
    Call,               // "call"
    Constant,           // "constant"
    Eq,                 // "eq"
    Ne,                 // "ne"
    Ugt,                // "ugt"
    Uge,                // "uge"
    Ult,                // "ult"
    Ule,                // "ule"
    Ptr,                // "ptr"
    I64,                // "i64"
    I8,                 // "i8"
    I1,                 // "i1"
    Void,               // "void"
    X,                  // "x"
    LParen,             // "("
    RParen,             // ")"
    LBrace,             // "{"
    RBrace,             // "}"
    LBracket,           // "["
    RBracket,           // "]"
    Comma,              // ","
    Assign,             // "="
    CString(String),    // c"..."
    Local(String),      // %...
    LabelIdent(String), // ...:
    Global(String),     // @...
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
fn lLexer_next_token(lexer: &mut LLexer) {
    lLexer_skip_whitespace_and_comments(lexer);

    let token: LToken = match lLexer_peek_char(lexer) {
        Option::Some(ch) => {
            if and(ch == 'c', lLexer_next_char_eq(lexer, '"')) {
                let value: String = lLexer_scan_cstring(lexer);
                LToken::CString(value)
            } else if is_alpha(ch) {
                lLexer_scan_keyword_or_label(lexer)
            } else if is_digit(ch) {
                let value: usize = lLexer_scan_integer(lexer);
                LToken::Integer(value)
            } else {
                lLexer_scan_symbol(lexer)
            }
        },
        Option::None => LToken::Eof,
    };
    lLexer_set_current_token(lexer, token);
}

/// Scan and return a c"..." string literal.
fn lLexer_scan_cstring(lexer: &mut LLexer) -> String {
    let mut literal: String = string_new();
    lLexer_expect_char(lexer, 'c');
    lLexer_expect_char(lexer, '"');

    while true {
        match lLexer_consume_char(lexer) {
            Option::Some(c) => {
                if c == '"' {
                    return literal;
                } else if c == '\\' {
                    let character: char = lLexer_scan_escape(lexer);
                    string_push(&mut literal, character);
                } else {
                    string_push(&mut literal, c)
                }
            },
            Option::None => panic("unterminated LLVM c-string"),
        }
    }
    unreachable()
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

fn lLexer_scan_identifier(lexer: &mut LLexer) -> String {
    let mut identifier: String = string_new();
    while true {
        match lLexer_peek_char(lexer) {
            Option::Some(ch) => {
                if is_llvm_identifier(ch) {
                    lLexer_consume_char(lexer);
                    string_push(&mut identifier, ch);
                } else {
                    return identifier;
                }
            },
            Option::None => return identifier,
        }
    }
    unreachable()
}

fn lLexer_scan_keyword_or_label(lexer: &mut LLexer) -> LToken {
    let identifier: String = lLexer_scan_identifier(lexer);

    if str_eq(&identifier, "define") {
        LToken::Define
    } else if str_eq(&identifier, "declare") {
        LToken::Declare
    } else if str_eq(&identifier, "ret") {
        LToken::Ret
    } else if str_eq(&identifier, "inttoptr") {
        LToken::IntToPtr
    } else if str_eq(&identifier, "ptrtoint") {
        LToken::PtrToInt
    } else if str_eq(&identifier, "br") {
        LToken::Br
    } else if str_eq(&identifier, "label") {
        LToken::Label
    } else if str_eq(&identifier, "add") {
        LToken::Add
    } else if str_eq(&identifier, "sub") {
        LToken::Sub
    } else if str_eq(&identifier, "mul") {
        LToken::Mul
    } else if str_eq(&identifier, "udiv") {
        LToken::Udiv
    } else if str_eq(&identifier, "urem") {
        LToken::Urem
    } else if str_eq(&identifier, "icmp") {
        LToken::Icmp
    } else if str_eq(&identifier, "zext") {
        LToken::Zext
    } else if str_eq(&identifier, "trunc") {
        LToken::Trunc
    } else if str_eq(&identifier, "alloca") {
        LToken::Alloca
    } else if str_eq(&identifier, "store") {
        LToken::Store
    } else if str_eq(&identifier, "load") {
        LToken::Load
    } else if str_eq(&identifier, "to") {
        LToken::To
    } else if str_eq(&identifier, "call") {
        LToken::Call
    } else if str_eq(&identifier, "constant") {
        LToken::Constant
    } else if str_eq(&identifier, "eq") {
        LToken::Eq
    } else if str_eq(&identifier, "ne") {
        LToken::Ne
    } else if str_eq(&identifier, "ugt") {
        LToken::Ugt
    } else if str_eq(&identifier, "uge") {
        LToken::Uge
    } else if str_eq(&identifier, "ult") {
        LToken::Ult
    } else if str_eq(&identifier, "ule") {
        LToken::Ule
    } else if str_eq(&identifier, "ptr") {
        LToken::Ptr
    } else if str_eq(&identifier, "i64") {
        LToken::I64
    } else if str_eq(&identifier, "i8") {
        LToken::I8
    } else if str_eq(&identifier, "i1") {
        LToken::I1
    } else if str_eq(&identifier, "void") {
        LToken::Void
    } else if str_eq(&identifier, "x") {
        LToken::X
    } else {
        match lLexer_peek_char(lexer) {
            Option::Some(c) => {
                if c == ':' {
                    lLexer_consume_char(lexer);
                    LToken::LabelIdent(identifier)
                } else {
                    let mut msg: String = string("unexpected identifier: ");
                    string_push_string(&mut msg, &identifier);
                    lLexer_error(lexer, &msg)
                }
            },
            _ => {
                let mut msg: String = string("unexpected identifier: ");
                string_push_string(&mut msg, &identifier);
                lLexer_error(lexer, &msg);
            },
        }
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
        '@' => {
            let ident: String = lLexer_scan_identifier(lexer);
            LToken::Global(ident)
        },
        '%' => {
            let ident: String = lLexer_scan_identifier(lexer);
            LToken::Local(ident)
        },
        '(' => LToken::LParen,
        ')' => LToken::RParen,
        '{' => LToken::LBrace,
        '}' => LToken::RBrace,
        '[' => LToken::LBracket,
        ']' => LToken::RBracket,
        ',' => LToken::Comma,
        '=' => LToken::Assign,
        c => {
            let mut msg: String = string("unexpected character in LLVM-IR input: ");
            string_push(&mut msg, c);
            lLexer_error(lexer, &msg);
        },
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
            Option::Some(c) => {
                if c == '\n' {
                    return;
                }
            },
            Option::None => return,
        }
    }
}

// -----------------------------------------------------------------
// ------------------------- Parser --------------------------------
// -----------------------------------------------------------------

/// The parser state for a LLLVM program.
enum LParser {
    /// lexer, result AST, symbol table, number of registers
    Parser(LLexer, LAst, StringMap<LType>, usize),
}

/// Create an LLVM parser and prime the first token.
fn lparser_new(source: String) -> LParser {
    LParser::Parser(lLexer_new(source), lAst_new(), stringMap_new::<LType>(), 0)
}

/// Get immutable parser lexer access.
fn lparser_lexer(LParser::Parser(lexer, _, _, _): &LParser) -> &LLexer {
    lexer
}

/// Get mutable parser AST access.
fn lparser_ast_mut(LParser::Parser(_, ast, _, _): &mut LParser) -> &mut LAst {
    ast
}

/// Insert register name. Returns false on duplicate.
fn lparser_symtable_insert(LParser::Parser(_, _, locals, _): &mut LParser, name: &String, ty: LType) -> bool {
    stringMap_insert_or_update::<LType>(locals, name, ty)
}

/// Lookup a register type in the local symbol table.
fn lparser_symtable_get<'a>(
    LParser::Parser(_, _, registers, _): &'a LParser,
    name: &String,
) -> Option<&'a LType> {
    stringMap_get::<LType>(registers, name)
}

/// Clear local register table buckets.
fn lparser_symtable_reset(LParser::Parser(_, _, registers, _): &mut LParser) {
    *registers = stringMap_new::<LType>();
}

fn lparser_register_count(LParser::Parser(_, _, _, counter): &LParser) -> usize {
    *counter
}

fn lparser_increment_register_count(LParser::Parser(_, _, _, counter): &mut LParser) {
    *counter = *counter + 1;
}

fn lparser_set_register_count(LParser::Parser(_, _, _, counter): &mut LParser, value: usize) {
    *counter = value;
}

/// Parse LLVM source into LLVM AST.
fn lparse_to_ast(source: String) -> LAst {
    let mut parser: LParser = lparser_new(source);
    lparse_language(&mut parser);
    lparse_insert_intrinsics(&mut parser);
    let LParser::Parser(_, ast, _, _): LParser = parser;
    ast
}

/// Get current LLVM parser token.
fn lparser_current_token(parser: &LParser) -> &LToken {
    lLexer_current_token(lparser_lexer(parser))
}

/// Consume and return the current LLVM parser token.
fn lparser_consume_current_token(parser: &mut LParser) -> LToken {
    let lexer: &LLexer = lparser_lexer(parser);
    let token: LToken = lToken_clone(lLexer_current_token(lexer));
    lparser_next_token(parser);
    token
}

/// Advance and return next LLVM parser token.
fn lparser_next_token(LParser::Parser(lexer, _, _, _): &mut LParser) {
    lLexer_next_token(lexer);
}

/// Check whether parser current token equals expected token.
fn lparser_current_token_eq(parser: &LParser, token: &LToken) -> bool {
    lToken_eq(lparser_current_token(parser), token)
}

/// Try consuming one token and report success.
fn lparser_try_consume(parser: &mut LParser, token: &LToken) -> bool {
    if lparser_current_token_eq(parser, token) {
        lparser_next_token(parser);
        true
    } else {
        false
    }
}

/// Require and consume one token.
fn lparser_expect_token(parser: &mut LParser, token: &LToken) {
    if not(lparser_try_consume(parser, token)) {
        let message: String = lparser_expected_message(parser, &lToken_to_string(token));
        lparser_error(parser, &message);
    }
}

/// Read and consume one identifier token.
fn lparser_expect_identifier(parser: &mut LParser, is_local: bool) -> String {
    if is_local {
        match lparser_current_token(parser) {
            LToken::Local(identifier) => {
                let value: String = string_clone(identifier);
                lparser_next_token(parser);
                value
            },
            LToken::LabelIdent(identifier) => {
                let value: String = string_clone(identifier);
                lparser_next_token(parser);
                value
            },
            _ => {
                let message: String = lparser_expected_message(parser, &string("local LLVM identifier"));
                lparser_error(parser, &message)
            },
        }
    } else {
        match lparser_current_token(parser) {
            LToken::Global(identifier) => {
                let value: String = string_clone(identifier);
                lparser_next_token(parser);
                value
            },
            _ => {
                let message: String = lparser_expected_message(parser, &string("global LLVM identifier"));
                lparser_error(parser, &message)
            },
        }
    }
}

fn lparser_expect_type(parser: &LParser, actual: &LType, expected: &LType) {
    if not(lType_eq(actual, expected)) {
        let mut msg: String = string("expected ");
        string_push_str(&mut msg, lType_to_str(expected));
        string_push_str(&mut msg, ", but got: ");
        string_push_str(&mut msg, lType_to_str(actual));
        lparser_warning(parser, &msg);
    }
}

fn lparser_expect_value_type(parser: &LParser, value: &LValue, expected: &LType) {
    let matches: bool = match value {
        LValue::Register(name) => match lparser_symtable_get(parser, name) {
            Option::Some(actual) => lType_eq(actual, expected),
            Option::None => false,
        },
        LValue::Literal(_) => match expected {
            LType::I1 | LType::I8 | LType::I64 => true, // allow overflows
            _ => false,
        },
        LValue::Global(_) => lType_eq(&LType::Ptr, expected),
    };
    if not(matches) {
        lparser_warning(parser, &string("LLLVM value does not match expected type"));
    }
}

/// Return true if the current token indicates the start of a new instruction.
fn lparser_is_instruction_start(parser: &mut LParser) -> bool {
    match lparser_current_token(parser) {
        LToken::Ret | LToken::Br | LToken::Local(_) | LToken::Store | LToken::Call => true,
        _ => false,
    }
}

/// Abstract syntax tree of a LLVM-IR module.
enum LAst {
    AST(Vec<CString>, StringMap<LFunction>),
}

/// Top-level LLVM global data.
enum CString {
    /// name, bytes
    String(String, String),
}

/// Create an empty LLVM AST.
fn lAst_new() -> LAst {
    LAst::AST(vec_new::<CString>(), stringMap_new::<LFunction>())
}

/// Get immutable access to the top-level globals list.
fn lAst_globals(LAst::AST(cstrings, _): &LAst) -> &Vec<CString> {
    cstrings
}

/// Get mutable access to the top-level globals list.
fn lAst_globals_mut(LAst::AST(cstrings, _): &mut LAst) -> &mut Vec<CString> {
    cstrings
}

/// Insert a global entry into the AST. Returns false on duplicate name.
fn lAst_insert_global(ast: &mut LAst, name: String, global: CString) -> bool {
    let globals: &Vec<CString> = lAst_globals(ast);

    let mut i: usize = 0;
    while i < vec_len::<CString>(globals) {
        let CString::String(existing_name, _): &CString = vec_at::<CString>(globals, i);
        if string_eq(existing_name, &name) {
            return false;
        }
        i = i + 1;
    }

    vec_push::<CString>(lAst_globals_mut(ast), global);
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
fn parser_lAst_insert_function(parser: &mut LParser, name: &String, function: LFunction) {
    let LParser::Parser(_, ast, _, _): &mut LParser = parser;
    let functions: &mut StringMap<LFunction> = lAst_functions_mut(ast);
    if not(stringMap_insert_or_update::<LFunction>(functions, name, function)) {
        lparser_error(parser, &string("duplicate LLVM function definition"));
    }
}

/// Lookup a function in the AST by name.
fn lAst_lookup_function<'a>(ast: &'a LAst, name: &String) -> &'a LFunction {
    match stringMap_get::<LFunction>(lAst_functions(ast), name) {
        Option::Some(function) => function,
        Option::None => panic("unknown LLVM function"),
    }
}

/// An executable LLVM-IR function.
enum LFunction {
    /// parameters, first label, basic blocks, instruction count
    Function(Vec<LParameter>, String, StringMap<Vec<Instruction>>, usize),
    /// return type
    BuiltIn(BuiltIn),
}

/// Supported LLLVM-IR declared or intrinsic functions.
enum BuiltIn {
    Exit,
    Malloc,
    Free,
    Open,
    Read,
    Write,
    Memcpy, // intrinsic for performance
}

/// Represents a parameter of an LLVM function.
enum LParameter {
    /// identifier, type
    Parameter(String, LType),
}

/// Supported LLVM types in the subset.
enum LType {
    I1,
    I8,
    I64,
    Ptr,
    Void,
}

fn lType_bitwidth(ty: &LType) -> usize {
    match ty {
        LType::I1 => 1,
        LType::I8 => 8,
        LType::I64 | LType::Ptr => 64,
        LType::Void => 0,
    }
}

/// Return the size of an LLVM type in bytes.
fn lType_size(ty: &LType) -> usize {
    max(1, lType_bitwidth(ty) / 8)
}

fn lType_is_integer(ty: &LType) -> bool {
    match ty {
        LType::I1 | LType::I8 | LType::I64 => true,
        _ => false,
    }
}

fn lType_is_pointer(ty: &LType) -> bool {
    match ty {
        LType::Ptr => true,
        _ => false,
    }
}

fn lType_is_void(ty: &LType) -> bool {
    match ty {
        LType::Void => true,
        _ => false,
    }
}

/// Normalize a value so it wraps around according to the given type.
fn ltype_overflow_value(value: usize, ty: &LType) -> usize {
    match ty {
        LType::I1 => value % 2,
        LType::I8 => value % 256,
        _ => value,
    }
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
    /// target type, value
    Cast(LType, LValue),
    /// allocated type, count
    Alloca(LType, usize),
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
        AssignOp::Binary(_, ty, _, _) => lType_clone(ty),
        AssignOp::Icmp(_, _, _, _) => LType::I1,
        AssignOp::Call(Call::Call(ty, _, _)) => lType_clone(ty),
        AssignOp::Cast(ty, _) => lType_clone(ty),
        AssignOp::Alloca(_, _) => LType::Ptr,
        AssignOp::Load(ty, _) => lType_clone(ty),
    }
}

/// Represents an LLVM value operand.
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

/// Insert intrinsics into the AST.
fn lparse_insert_intrinsics(parser: &mut LParser) {
    let name: String = string("llvm.memcpy.p0.p0.i64");
    parser_lAst_insert_function(parser, &name, LFunction::BuiltIn(BuiltIn::Memcpy));
}

fn lparse_language(parser: &mut LParser) {
    while not(lparser_current_token_eq(parser, &LToken::Eof)) {
        match lparser_current_token(parser) {
            LToken::Global(_) => lparse_string(parser),
            LToken::Define => lparse_function(parser),
            LToken::Declare => lparse_declare(parser),
            _ => {
                let message: String = lparser_expected_message(parser, &string("LLVM top-level item"));
                lparser_error(parser, &message)
            },
        }
    }
}

fn lparse_string(parser: &mut LParser) {
    let name: String = lparser_expect_identifier(parser, false);
    lparser_expect_token(parser, &LToken::Assign);
    lparser_expect_token(parser, &LToken::Constant);

    lparser_expect_token(parser, &LToken::LBracket);
    let len: usize = lparse_integer(parser);
    lparser_expect_token(parser, &LToken::X);
    lparser_expect_token(parser, &LToken::I8);
    lparser_expect_token(parser, &LToken::RBracket);

    match lparser_current_token(parser) {
        LToken::CString(value) => {
            let string_value: String = string_clone(value);
            if string_len(&string_value) != len {
                lparser_error(
                    parser,
                    &string("c-string length does not match with declared length"),
                )
            }
            lparser_next_token(parser);
            if not(lAst_insert_global(
                lparser_ast_mut(parser),
                string_clone(&name),
                CString::String(name, string_value),
            )) {
                lparser_error(parser, &string("duplicate LLVM global string"));
            }
        },
        _ => {
            let message: String = lparser_expected_message(parser, &string("LLVM c-string literal"));
            lparser_error(parser, &message)
        },
    }
}

fn lparse_function(parser: &mut LParser) {
    lparser_expect_token(parser, &LToken::Define);
    let return_type: LType = lparse_type(parser); // TODO: check type of ret using this
    let function_name: String = lparser_expect_identifier(parser, false);

    lparser_symtable_reset(parser);
    let params: Vec<LParameter> = lparse_parameters(parser, true);
    lparser_set_register_count(parser, vec_len::<LParameter>(&params));

    lparser_expect_token(parser, &LToken::LBrace);
    let mut blocks: StringMap<Vec<Instruction>> = stringMap_new::<Vec<Instruction>>();
    let first_label: String = lparser_expect_identifier(parser, true);
    let block: Vec<Instruction> = lparse_instructions(parser);
    stringMap_insert_or_update::<Vec<Instruction>>(&mut blocks, &first_label, block);
    while not(lparser_current_token_eq(parser, &LToken::RBrace)) {
        let label: String = lparser_expect_identifier(parser, true);
        let block: Vec<Instruction> = lparse_instructions(parser);
        if not(stringMap_insert_or_update::<Vec<Instruction>>(
            &mut blocks,
            &label,
            block,
        )) {
            lparser_error(parser, &string("Duplicate basic block labels detected"));
        }
    }
    lparser_expect_token(parser, &LToken::RBrace);

    let function: LFunction =
        LFunction::Function(params, first_label, blocks, lparser_register_count(parser));
    parser_lAst_insert_function(parser, &function_name, function);
}

fn lparse_declare(parser: &mut LParser) {
    lparser_expect_token(parser, &LToken::Declare);
    let return_type: LType = lparse_type(parser);
    let name: String = lparser_expect_identifier(parser, false);

    lparser_symtable_reset(parser);
    let parameters: Vec<LParameter> = lparse_parameters(parser, false);

    let mut is_incorrect: bool = false;
    let builtin: BuiltIn = if str_eq(&name, "exit") {
        is_incorrect = not(vec_len::<LParameter>(&parameters) == 1);
        if not(is_incorrect) {
            let LParameter::Parameter(_, ty): &LParameter = vec_at::<LParameter>(&parameters, 0);
            is_incorrect = not(and(lType_is_integer(ty), lType_is_void(&return_type)));
        }
        BuiltIn::Exit
    } else if str_eq(&name, "malloc") {
        is_incorrect = not(vec_len::<LParameter>(&parameters) == 1);
        if not(is_incorrect) {
            let LParameter::Parameter(_, ty): &LParameter = vec_at::<LParameter>(&parameters, 0);
            is_incorrect = not(and(lType_is_integer(ty), lType_is_pointer(&return_type)));
        }
        BuiltIn::Malloc
    } else if str_eq(&name, "free") {
        is_incorrect = not(vec_len::<LParameter>(&parameters) == 1);
        if not(is_incorrect) {
            let LParameter::Parameter(_, ty): &LParameter = vec_at::<LParameter>(&parameters, 0);
            is_incorrect = not(and(lType_is_pointer(ty), lType_is_void(&return_type)));
        }
        BuiltIn::Free
    } else if str_eq(&name, "open") {
        is_incorrect = not(vec_len::<LParameter>(&parameters) == 3);
        if not(is_incorrect) {
            let LParameter::Parameter(_, ty1): &LParameter = vec_at::<LParameter>(&parameters, 0);
            let LParameter::Parameter(_, ty2): &LParameter = vec_at::<LParameter>(&parameters, 1);
            let LParameter::Parameter(_, ty3): &LParameter = vec_at::<LParameter>(&parameters, 2);
            is_incorrect = not(and(
                lType_is_pointer(ty1),
                and(
                    lType_is_integer(ty2),
                    and(lType_is_integer(ty3), lType_is_integer(&return_type)),
                ),
            ));
        }
        BuiltIn::Open
    } else if or(str_eq(&name, "read"), str_eq(&name, "write")) {
        is_incorrect = not(vec_len::<LParameter>(&parameters) == 3);
        if not(is_incorrect) {
            let LParameter::Parameter(_, ty1): &LParameter = vec_at::<LParameter>(&parameters, 0);
            let LParameter::Parameter(_, ty2): &LParameter = vec_at::<LParameter>(&parameters, 1);
            let LParameter::Parameter(_, ty3): &LParameter = vec_at::<LParameter>(&parameters, 2);
            is_incorrect = not(and(
                lType_is_integer(ty1),
                and(
                    lType_is_pointer(ty2),
                    and(lType_is_integer(ty3), lType_is_integer(&return_type)),
                ),
            ));
        }
        if str_eq(&name, "read") {
            BuiltIn::Read
        } else {
            BuiltIn::Write
        }
    } else {
        lparser_error(parser, &string("unknown declared function"));
    };
    if is_incorrect {
        let mut msg: String = string("signature of ");
        string_push_string(&mut msg, &name);
        string_push_str(&mut msg, " is incorrect");
        lparser_error(parser, &msg);
    }

    let function: LFunction = LFunction::BuiltIn(builtin);
    parser_lAst_insert_function(parser, &name, function);
}

/// Parse parameters of a function.
///
/// * `parser`: The parser state
/// * `require_names`: True, if the parameters are named (function definition). False, if they are not (function declaration).
fn lparse_parameters(parser: &mut LParser, named: bool) -> Vec<LParameter> {
    let mut parameters: Vec<LParameter> = vec_new::<LParameter>();

    lparser_expect_token(parser, &LToken::LParen);

    if not(lparser_current_token_eq(parser, &LToken::RParen)) {
        let parameter_type: LType = lparse_type(parser);
        let param_name: String = lparse_parameter_name(parser, 0);
        lparser_symtable_insert(parser, &param_name, lType_clone(&parameter_type));

        let parameter: LParameter = LParameter::Parameter(param_name, parameter_type);
        vec_push::<LParameter>(&mut parameters, parameter);

        while lparser_current_token_eq(parser, &LToken::Comma) {
            lparser_next_token(parser);

            let parameter_type: LType = lparse_type(parser);
            let param_name: String = lparse_parameter_name(parser, vec_len::<LParameter>(&parameters));

            if and(
                named,
                not(lparser_symtable_insert(
                    parser,
                    &param_name,
                    lType_clone(&parameter_type),
                )),
            ) {
                lparser_error(parser, &string("duplicate parameters in LLVM function"));
            }

            let parameter: LParameter = LParameter::Parameter(param_name, parameter_type);
            vec_push::<LParameter>(&mut parameters, parameter);
        }
    }
    lparser_expect_token(parser, &LToken::RParen);
    parameters
}

fn lparse_parameter_name(parser: &mut LParser, index: usize) -> String {
    match lparser_current_token(parser) {
        LToken::Local(_) => lparser_expect_identifier(parser, true),
        _ => {
            let mut name: String = string("arg");
            string_push_string(&mut name, &integer_to_string(index));
            name
        },
    }
}

fn lparse_instructions(parser: &mut LParser) -> Vec<Instruction> {
    let mut block: Vec<Instruction> = vec_new::<Instruction>();
    while lparser_is_instruction_start(parser) {
        let instruction: Instruction = lparse_instruction(parser);
        vec_push::<Instruction>(&mut block, instruction);
    }
    block
}

fn lparse_instruction(parser: &mut LParser) -> Instruction {
    match lparser_current_token(parser) {
        LToken::Ret => lparse_return(parser),
        LToken::Br => lparse_branch(parser),
        LToken::Local(_) => Instruction::Assignment(lparse_assignment(parser)),
        LToken::Store => lparse_store(parser),
        LToken::Call => {
            lparser_next_token(parser);
            Instruction::Call(lparse_call(parser))
        },
        _ => {
            let message: String = lparser_expected_message(parser, &string("LLVM instruction"));
            lparser_error(parser, &message)
        },
    }
}

fn lparse_return(parser: &mut LParser) -> Instruction {
    lparser_expect_token(parser, &LToken::Ret);
    let returned_type: LType = lparse_type(parser);
    let return_value: Option<LValue> = if lType_eq(&returned_type, &LType::Void) {
        Option::<LValue>::None
    } else {
        Option::<LValue>::Some(lparse_value(parser))
    };
    Instruction::Ret(returned_type, return_value)
}

fn lparse_branch(parser: &mut LParser) -> Instruction {
    lparser_expect_token(parser, &LToken::Br);
    let branch: Branch = if lparser_try_consume(parser, &LToken::Label) {
        let target_label: String = lparser_expect_identifier(parser, true);
        Branch::Unconditional(target_label)
    } else {
        lparser_expect_token(parser, &LToken::I1);
        let condition: LValue = lparse_value(parser);
        lparser_expect_token(parser, &LToken::Comma);

        lparser_expect_token(parser, &LToken::Label);
        let then_label: String = lparser_expect_identifier(parser, true);
        lparser_expect_token(parser, &LToken::Comma);

        lparser_expect_token(parser, &LToken::Label);
        let else_label: String = lparser_expect_identifier(parser, true);

        Branch::Conditional(condition, then_label, else_label)
    };
    Instruction::Br(branch)
}

fn lparse_assignment(parser: &mut LParser) -> AssignInstruction {
    let target_register: String = lparser_expect_identifier(parser, true);

    lparser_expect_token(parser, &LToken::Assign);
    let operation: AssignOp = match lparser_consume_current_token(parser) {
        LToken::Add => lparse_binary_assign(parser, BinaryOp::Add),
        LToken::Sub => lparse_binary_assign(parser, BinaryOp::Sub),
        LToken::Mul => lparse_binary_assign(parser, BinaryOp::Mul),
        LToken::Udiv => lparse_binary_assign(parser, BinaryOp::Udiv),
        LToken::Urem => lparse_binary_assign(parser, BinaryOp::Urem),
        LToken::Icmp => lparse_icmp_assign(parser),
        LToken::Zext => lparse_cast_assign(parser, CastOp::Zext),
        LToken::Trunc => lparse_cast_assign(parser, CastOp::Trunc),
        LToken::IntToPtr => lparse_cast_assign(parser, CastOp::IntToPtr),
        LToken::PtrToInt => lparse_cast_assign(parser, CastOp::PtrToInt),
        LToken::Alloca => lparse_alloca_assign(parser),
        LToken::Load => lparse_load_assign(parser),
        LToken::Call => lparse_call_assign(parser),
        _ => {
            let message: String = lparser_expected_message(parser, &string("LLVM assignment operation"));
            lparser_error(parser, &message)
        },
    };

    if not(lparser_symtable_insert(
        parser,
        &target_register,
        assignOp_get_type(&operation),
    )) {
        lparser_warning(parser, &string("SSA: duplicate register assignment"));
    }

    lparser_increment_register_count(parser);
    AssignInstruction::Assign(target_register, operation)
}

fn lparse_binary_assign(parser: &mut LParser, operator: BinaryOp) -> AssignOp {
    let ty: LType = lparse_type(parser);
    let left: LValue = lparse_value(parser);
    lparser_expect_value_type(parser, &left, &ty);

    lparser_expect_token(parser, &LToken::Comma);
    let right: LValue = lparse_value(parser);
    lparser_expect_value_type(parser, &right, &ty);

    AssignOp::Binary(operator, ty, left, right)
}

fn lparse_icmp_assign(parser: &mut LParser) -> AssignOp {
    let predicate: IcmpOp = match lparser_consume_current_token(parser) {
        LToken::Eq => IcmpOp::Eq,
        LToken::Ne => IcmpOp::Ne,
        LToken::Ugt => IcmpOp::Ugt,
        LToken::Uge => IcmpOp::Uge,
        LToken::Ult => IcmpOp::Ult,
        LToken::Ule => IcmpOp::Ule,
        _ => {
            let message: String = lparser_expected_message(parser, &string("LLVM icmp operator"));
            lparser_error(parser, &message)
        },
    };

    let ty: LType = lparse_type(parser);
    let left: LValue = lparse_value(parser);
    lparser_expect_value_type(parser, &left, &ty);

    lparser_expect_token(parser, &LToken::Comma);
    let right: LValue = lparse_value(parser);
    lparser_expect_value_type(parser, &right, &ty);

    AssignOp::Icmp(predicate, ty, left, right)
}

fn lparse_call_assign(parser: &mut LParser) -> AssignOp {
    let call: Call = lparse_call(parser);

    let Call::Call(return_type, _, _): &Call = &call;
    if lType_eq(return_type, &LType::Void) {
        lparser_error(parser, &string("cannot assign void to a register"));
    }

    AssignOp::Call(call)
}

fn lparse_cast_assign(parser: &mut LParser, operator: CastOp) -> AssignOp {
    let from_type: LType = lparse_type(parser);

    let value: LValue = lparse_value(parser);
    lparser_expect_value_type(parser, &value, &from_type);

    lparser_expect_token(parser, &LToken::To);
    let to_type: LType = lparse_type(parser);

    match operator {
        CastOp::Zext => {
            let from_bits: usize = lType_bitwidth(&from_type);
            let to_bits: usize = lType_bitwidth(&to_type);
            if not(from_bits < to_bits) {
                lparser_warning(parser, &string("zext: source is not smaller than target"));
            }
        },
        CastOp::Trunc => {
            let from_bits: usize = lType_bitwidth(&from_type);
            let to_bits: usize = lType_bitwidth(&to_type);
            if not(from_bits > to_bits) {
                lparser_warning(parser, &string("zext: source is not larger than target"));
            }
        },
        CastOp::IntToPtr => {
            lparser_expect_type(parser, &from_type, &LType::I64);
            lparser_expect_type(parser, &to_type, &LType::Ptr);
        },
        CastOp::PtrToInt => {
            lparser_expect_type(parser, &from_type, &LType::Ptr);
            lparser_expect_type(parser, &to_type, &LType::I64);
        },
    }
    AssignOp::Cast(to_type, value)
}

fn lparse_alloca_assign(parser: &mut LParser) -> AssignOp {
    let allocated_type: LType = lparse_type(parser);
    lparser_expect_token(parser, &LToken::Comma);
    lparser_expect_token(parser, &LToken::I64);
    match lparser_consume_current_token(parser) {
        LToken::Integer(count) => AssignOp::Alloca(allocated_type, count),
        _ => lparser_error(parser, &string("expected integer after , in alloca ")),
    }
}

fn lparse_load_assign(parser: &mut LParser) -> AssignOp {
    let loaded_type: LType = lparse_type(parser);
    lparser_expect_token(parser, &LToken::Comma);

    lparser_expect_token(parser, &LToken::Ptr);
    let address: LValue = lparse_value(parser);
    lparser_expect_value_type(parser, &address, &LType::Ptr);

    AssignOp::Load(loaded_type, address)
}

fn lparse_store(parser: &mut LParser) -> Instruction {
    lparser_expect_token(parser, &LToken::Store);

    let store_type: LType = lparse_type(parser);
    let value: LValue = lparse_value(parser);
    lparser_expect_value_type(parser, &value, &store_type);

    lparser_expect_token(parser, &LToken::Comma);
    lparser_expect_token(parser, &LToken::Ptr);

    let address: LValue = lparse_value(parser);
    lparser_expect_value_type(parser, &address, &LType::Ptr);

    Instruction::Store(store_type, value, address)
}

fn lparse_call(parser: &mut LParser) -> Call {
    let return_type: LType = lparse_type(parser);
    let callee: String = lparser_expect_identifier(parser, false);

    lparser_expect_token(parser, &LToken::LParen);
    let mut arguments: Vec<LTypedValue> = vec_new::<LTypedValue>();
    if not(lparser_current_token_eq(parser, &LToken::RParen)) {
        let arg_type: LType = lparse_type(parser);
        let arg_value: LValue = lparse_value(parser);
        lparser_expect_value_type(parser, &arg_value, &arg_type);
        vec_push::<LTypedValue>(&mut arguments, LTypedValue::Pair(arg_type, arg_value));

        while lparser_current_token_eq(parser, &LToken::Comma) {
            lparser_next_token(parser);

            let arg_type: LType = lparse_type(parser);
            let arg_value: LValue = lparse_value(parser);
            lparser_expect_value_type(parser, &arg_value, &arg_type);
            vec_push::<LTypedValue>(&mut arguments, LTypedValue::Pair(arg_type, arg_value));
        }
    }
    lparser_expect_token(parser, &LToken::RParen);

    Call::Call(return_type, callee, arguments)
}

fn lparse_type(parser: &mut LParser) -> LType {
    match lparser_consume_current_token(parser) {
        LToken::I1 => LType::I1,
        LToken::I8 => LType::I8,
        LToken::I64 => LType::I64,
        LToken::Void => LType::Void,
        LToken::Ptr => LType::Ptr,
        _ => {
            let message: String = lparser_expected_message(parser, &string("LLVM type"));
            lparser_error(parser, &message)
        },
    }
}

fn lparse_value(parser: &mut LParser) -> LValue {
    match lparser_consume_current_token(parser) {
        LToken::Global(ident) => LValue::Global(ident),
        LToken::Local(ident) => LValue::Register(ident),
        LToken::Integer(value) => LValue::Literal(value),
        _ => {
            let message: String = lparser_expected_message(parser, &string("LLVM value"));
            lparser_error(parser, &message)
        },
    }
}

fn lparse_integer(parser: &mut LParser) -> usize {
    match lparser_consume_current_token(parser) {
        LToken::Integer(value) => value,
        _ => {
            let message: String = lparser_expected_message(parser, &string("LLVM integer literal"));
            lparser_error(parser, &message)
        },
    }
}

// -------------------------------------------------------------------
// ------------------------- Interpreter -----------------------------
// -------------------------------------------------------------------

/// Execution control flow after one LLVM-IR instruction.
enum ExecFlow<'a> {
    Continue,
    /// label
    Jump(&'a String),
    /// return value
    Return(usize),
}

/// Type that encapsulates the state of the LLVM emulator.
enum Emu {
    Emu(
        /// map of global names to their addresses
        StringMap<usize>,
        /// byte-addressed memory (data, heap, stack)
        Vec<u8>,
        /// stack pointer
        usize,
        /// current frame size,
        usize,
        /// bump pointer (points to top of heap)
        usize,
        /// head of free-list (address)
        usize,
        /// exit code, if the program exited
        Option<usize>,
    ),
}

/// Create a new emulator state with `memory_size` bytes of main memory and data segment initialised
/// with the globals found in the AST.
fn emu_new(memory_size: usize, ast: &LAst) -> Emu {
    let stack_pointer: usize = memory_size;
    let memory: Vec<u8> = unsafe { vec_with_len::<u8>(memory_size) };
    let globals: StringMap<usize> = stringMap_new::<usize>();
    let mut emulator: Emu = Emu::Emu(globals, memory, stack_pointer, 0, 0, 0, Option::<usize>::None);
    let heap_start: usize = emu_initialise_global_data(&mut emulator, ast);
    let Emu::Emu(_, _, _, _, bump_pointer, _, _): &mut Emu = &mut emulator;
    *bump_pointer = heap_start;
    emulator
}

/// Get a shared reference to the global values.
fn emu_globals(Emu::Emu(globals, _, _, _, _, _, _): &Emu) -> &StringMap<usize> {
    globals
}

/// Get mutable access to the global values.
fn emu_globals_mut(Emu::Emu(globals, _, _, _, _, _, _): &mut Emu) -> &mut StringMap<usize> {
    globals
}

/// Get the current value of the stack pointer.
fn emu_get_sp(Emu::Emu(_, _, stack_pointer, _, _, _, _): &Emu) -> usize {
    *stack_pointer
}

/// Set the value of the stack pointer.
fn emu_set_sp(Emu::Emu(_, _, stack_pointer, _, _, _, _): &mut Emu, value: usize) {
    *stack_pointer = value;
}

/// Get the size of the active stack frame in bytes.
fn emu_get_frame_size(Emu::Emu(_, _, _, frame_size, _, _, _): &Emu) -> usize {
    *frame_size
}

/// Set the size of the active stack frame.
fn emu_set_frame_size(Emu::Emu(_, _, _, frame_size, _, _, _): &mut Emu, value: usize) {
    *frame_size = value;
}

/// Get the current value of the allocator's bump pointer.
fn emu_get_bump_pointer(Emu::Emu(_, _, _, _, bump_pointer, _, _): &Emu) -> usize {
    *bump_pointer
}

/// Increases the allocator's bump pointer by `value`.
fn emu_increase_bump_pointer(Emu::Emu(_, _, _, _, bump_pointer, _, _): &mut Emu, value: usize) {
    *bump_pointer = *bump_pointer + value;
}

/// Get the address of the first free-list node.
fn emu_get_freelist_head(Emu::Emu(_, _, _, _, _, freelist, _): &Emu) -> usize {
    *freelist
}

/// Set the head of the free-list to the given address.
fn emu_set_freelist_head(Emu::Emu(_, _, _, _, _, freelist, _): &mut Emu, address: usize) {
    *freelist = address;
}

/// Return true if exit was requested and return the code.
fn emu_exit_code(Emu::Emu(_, _, _, _, _, _, exit_code): &Emu) -> Option<usize> {
    match exit_code {
        Option::Some(code) => Option::<usize>::Some(*code),
        Option::None => Option::<usize>::None,
    }
}

/// Set the exit code and mark the program as exited.
fn emu_set_exit_code(Emu::Emu(_, _, _, _, _, _, exit_code): &mut Emu, code: usize) {
    *exit_code = Option::<usize>::Some(code);
}

/// Allocate `size` many bytes on the stack and return the address.
fn emu_stack_alloc(emulator: &mut Emu, size: usize) -> Option<usize> {
    let bytes: usize = round_to_next_multiple(size, size_of::<usize>());
    let stack_pointer: usize = emu_get_sp(emulator);
    let frame_size: usize = emu_get_frame_size(emulator);

    let new_sp: usize = stack_pointer - bytes;
    if new_sp <= emu_get_bump_pointer(emulator) {
        Option::<usize>::None
    } else {
        emu_set_sp(emulator, new_sp);
        emu_set_frame_size(emulator, frame_size + bytes);
        Option::<usize>::Some(new_sp)
    }
}

/// Allocate `size` bytes on the heap and return the address. (Actually, it allocates `size + 16`
/// bytes to store size and free-list pointer of the block and returns the address offsetted by 16).
fn emu_malloc(emulator: &mut Emu, mut size: usize) -> Option<usize> {
    if size == 0 {
        return Option::<usize>::Some(0); // malloc() can return NULL in this case
    }

    size = size + size_of::<usize>() * 2; // 16 bytes for block metadata (size & next pointer)
    let aligned_size: usize = round_to_next_multiple(size, size_of::<usize>());

    let free_block: usize = emu_reuse_free_block_first_fit(emulator, size);
    if free_block != 0 {
        // entire block reused => metadata is not modified
        return Option::<usize>::Some(free_block + size_of::<usize>() * 2);
    }

    // reusing free block failed => increase bump pointer to allocate new block
    let mut bump_pointer: usize = emu_get_bump_pointer(emulator);
    if bump_pointer == 0 {
        bump_pointer = bump_pointer + size_of::<usize>();
        emu_increase_bump_pointer(emulator, size_of::<usize>()); // do not use address 0
    }
    if bump_pointer + aligned_size >= emu_get_sp(emulator) {
        return Option::<usize>::None; // OOM
    }

    emu_increase_bump_pointer(emulator, aligned_size); // allocation
    // store metadata (free-list pointer is not yet needed)
    emu_store_bytes(emulator, bump_pointer, aligned_size, 8);
    Option::<usize>::Some(bump_pointer + size_of::<usize>() * 2) // return address after block size
}

/// Given the address of a memory block, returns the size of it.
fn emu_mem_block_size(emulator: &Emu, block_address: usize) -> usize {
    emu_load_bytes(emulator, block_address, size_of::<usize>())
}

/// Given the address of a memory block, returns the address of the next memory block in the freelist.
fn emu_mem_block_next(emulator: &Emu, block_address: usize) -> usize {
    emu_load_bytes(emulator, block_address + size_of::<usize>(), size_of::<usize>())
}

/// Given the address of a memory block, updates the next-pointer to point to `next`.
fn emu_mem_block_set_next(emulator: &mut Emu, block: usize, next: usize) {
    emu_store_bytes(emulator, block + size_of::<usize>(), next, size_of::<usize>());
}

/// Returns address of free memory block using first-fit, if it exists, else NULL.
fn emu_reuse_free_block_first_fit(emulator: &mut Emu, size: usize) -> usize {
    let mut block: usize = emu_get_freelist_head(emulator);
    let mut block_size: usize = 0;
    let mut predecessor: usize = 0;
    while block != 0 {
        block_size = emu_mem_block_size(emulator, block);
        if size <= block_size {
            let next: usize = emu_mem_block_next(emulator, block);

            if predecessor != 0 {
                emu_mem_block_set_next(emulator, predecessor, next);
            } else {
                emu_set_freelist_head(emulator, next);
            }

            return block;
        }

        predecessor = block;
        block = emu_mem_block_next(emulator, block);
    }
    0
}

/// Free the memory block.
fn emu_free(emulator: &mut Emu, pointer: usize) {
    if pointer == 0 {
        return; // free() performs no operation if the pointer is NULL
    }

    let block_start: usize = pointer - size_of::<usize>() * 2;
    let next_address: usize = block_start + size_of::<usize>();

    let head: usize = emu_get_freelist_head(emulator);
    emu_store_bytes(emulator, next_address, head, size_of::<usize>());
    emu_set_freelist_head(emulator, block_start);
}

/// Push the given args onto the stack and return a `argv` pointer.
fn emu_push_argv(emulator: &mut Emu, args: &Args) -> usize {
    // Allocate space for argv on stack
    let mut size: usize = 0;
    let mut i: usize = 0;
    while i < args_len(args) {
        size = size + string_len(args_at(args, i)) + 1; // + 1 for NULL-termination
        i = i + 1;
    }
    size = size + args_len(args) * size_of::<usize>(); // pointer array
    let new_sp: usize = emu_get_sp(emulator) - size;
    emu_set_sp(emulator, new_sp);

    let NULL: usize = 0;
    let mut offset: usize = new_sp + args_len(args) * size_of::<usize>();
    i = 0;
    while i < args_len(args) {
        // store pointer to argument in pointer array
        let pointer_address: usize = new_sp + i * size_of::<usize>();
        emu_store_bytes(emulator, pointer_address, offset, size_of::<usize>());

        let arg: &String = args_at(args, i);
        let mut j: usize = 0;
        while j < string_len(arg) {
            let byte: usize = string_at(arg, j) as usize;
            emu_store_bytes(emulator, offset + j, byte, 1);
            j = j + 1;
        }
        emu_store_bytes(emulator, offset + string_len(arg), NULL, 1);
        offset = offset + string_len(arg) + 1;
        i = i + 1;
    }
    new_sp // pointer to pointer array
}

/// Load LLVM-IR C-Strings into the data segment and return the next available address (= start of the
/// heap). Assumes that the data segment starts at address 0 (since the AST serves as the code).
fn emu_initialise_global_data(emulator: &mut Emu, ast: &LAst) -> usize {
    let mut data_pointer: usize = 0;
    let mut i: usize = 0;
    while i < vec_len::<CString>(lAst_globals(ast)) {
        let CString::String(name, value): &CString = vec_at::<CString>(lAst_globals(ast), i);

        let alloc_size: usize = round_to_next_multiple(string_len(value), size_of::<usize>());
        let address: usize = data_pointer;

        let mut j: usize = 0;
        while j < string_len(value) {
            let character: usize = string_at(value, j) as usize;
            emu_store_bytes(emulator, address + j, character, 1);
            j = j + 1;
        }

        stringMap_insert_or_update::<usize>(emu_globals_mut(emulator), name, address);
        data_pointer = data_pointer + alloc_size;
        i = i + 1;
    }
    data_pointer
}

/// Deallocates the top stack frame by resetting the frame size to 0 and moving the stack pointer up
/// by the frame size.
fn emu_deallocate_stack_frame(emulator: &mut Emu) {
    let stack_pointer: usize = emu_get_sp(emulator);
    let frame_size: usize = emu_get_frame_size(emulator);
    emu_set_sp(emulator, stack_pointer + frame_size);
    emu_set_frame_size(emulator, 0);
}

/// Get a raw pointer to the memory the given address points to.
fn emu_get_memory_pointer(Emu::Emu(_, memory, _, _, _, _, _): &mut Emu, address: usize) -> Option<*mut u8> {
    match vec_get_mut::<u8>(memory, address) {
        Option::Some(address) => Option::<*mut u8>::Some(address as *mut u8),
        Option::None => Option::<*mut u8>::None,
    }
}

/// Store a little-endian integer value at `address` using `byte_count` bytes.
fn emu_store_bytes(emulator: &mut Emu, address: usize, value: usize, byte_count: usize) -> bool {
    let Emu::Emu(_, memory, _, _, _, _, _): &mut Emu = emulator;
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
fn emu_load_bytes(emulator: &Emu, address: usize, byte_count: usize) -> usize {
    let Emu::Emu(_, memory, _, _, _, _, _): &Emu = emulator;
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
            _ => panic("invalid address for load"),
        }
        i = i + 1;
    }
    value
}

/// Parse and emulate LLVM source and return the return value of `@main`.
fn emulate(source: String, memory_size: usize, args: &Args) -> usize {
    let ast: LAst = lparse_to_ast(source);
    let mut emulator: Emu = emu_new(memory_size, &ast);

    let argc: usize = args_len(args);
    let argv: usize = emu_push_argv(&mut emulator, args);
    let mut main_args: Vec<usize> = vec_with_capacity::<usize>(2);
    vec_push::<usize>(&mut main_args, argc);
    vec_push::<usize>(&mut main_args, argv);

    print_str("[Starting Emulation]\n");
    let exit_code: usize = emu_execute_function(&mut emulator, &ast, &string("main"), &main_args) % 256; // exit code is 16-bit integer
    print_str("=> Exited with exit code ");
    print_string(&integer_to_string(exit_code));
    println();
    exit_code
}

/// Execute the given function's body.
fn emu_execute_function(emulator: &mut Emu, ast: &LAst, name: &String, args: &Vec<usize>) -> usize {
    match lAst_lookup_function(ast, name) {
        LFunction::BuiltIn(builtin) => emu_execute_builtin(emulator, builtin, args),
        LFunction::Function(parameters, first_label, blocks, register_count) => {
            let previous_frame_size: usize = emu_get_frame_size(emulator);
            emu_set_frame_size(emulator, 0);
            let mut locals: StringMap<usize> = stringMap_with_len::<usize>(*register_count);

            let mut i: usize = 0;
            while i < vec_len::<LParameter>(parameters) {
                let LParameter::Parameter(name, _): &LParameter = vec_at::<LParameter>(parameters, i);
                let value: usize = *vec_at::<usize>(args, i);
                stringMap_insert_or_update::<usize>(&mut locals, name, value);
                i = i + 1;
            }

            let mut current_label: &String = first_label;
            while true {
                let instructions: &Vec<Instruction> =
                    match stringMap_get::<Vec<Instruction>>(blocks, current_label) {
                        Option::Some(block) => block,
                        Option::None => panic("unknown LLVM block label"),
                    };
                match emu_execute_instructions(emulator, ast, &mut locals, instructions) {
                    ExecFlow::Continue => panic("LLVM block did not terminate"),
                    ExecFlow::Jump(next_label) => current_label = next_label,
                    ExecFlow::Return(value) => {
                        drop_stringValueMap(locals);
                        emu_deallocate_stack_frame(emulator);
                        emu_set_frame_size(emulator, previous_frame_size);
                        return value;
                    },
                }
            }
            unreachable()
        },
    }
}

/// Execute one builtin function and return its value.
fn emu_execute_builtin(emulator: &mut Emu, builtin: &BuiltIn, arguments: &Vec<usize>) -> usize {
    match builtin {
        BuiltIn::Exit => {
            let exit_code: usize = *vec_at::<usize>(arguments, 0);
            emu_set_exit_code(emulator, exit_code);
            exit_code
        },
        BuiltIn::Malloc => {
            let size: usize = *vec_at::<usize>(arguments, 0);
            match emu_malloc(emulator, size) {
                Option::Some(address) => address,
                Option::None => 0, // = NULL
            }
        },
        BuiltIn::Free => {
            let pointer: usize = *vec_at::<usize>(arguments, 0);
            emu_free(emulator, pointer);
            0 // free() returns void, so this is ignored
        },
        BuiltIn::Open => {
            let path: usize = *vec_at::<usize>(arguments, 0);
            let flags: usize = *vec_at::<usize>(arguments, 1);
            let mode: usize = *vec_at::<usize>(arguments, 2);
            match emu_get_memory_pointer(emulator, path) {
                Option::Some(ptr) => unsafe { open(ptr, flags, mode) },
                _ => panic("trying to pass an out-of-bounds address to open()"),
            }
        },
        BuiltIn::Read => {
            let fd: usize = *vec_at::<usize>(arguments, 0);
            let buf: usize = *vec_at::<usize>(arguments, 1);
            let count: usize = *vec_at::<usize>(arguments, 2);
            match emu_get_memory_pointer(emulator, buf) {
                Option::Some(ptr) => unsafe { read(fd, ptr, count) },
                _ => panic("trying to read() at an out-of-bound address"),
            }
        },
        BuiltIn::Write => {
            let fd: usize = *vec_at::<usize>(arguments, 0);
            let buf: usize = *vec_at::<usize>(arguments, 1);
            let count: usize = *vec_at::<usize>(arguments, 2);
            match emu_get_memory_pointer(emulator, buf) {
                Option::Some(ptr) => unsafe { write(fd, ptr, count) },
                _ => panic("trying to write() at an out-of-bound address"),
            }
        },
        BuiltIn::Memcpy => {
            let dest: usize = *vec_at::<usize>(arguments, 0);
            let src: usize = *vec_at::<usize>(arguments, 1);
            let len: usize = *vec_at::<usize>(arguments, 2);
            // autos does not support the isvolatile parameter, so it's ignored
            let dest: *mut u8 = match emu_get_memory_pointer(emulator, dest) {
                Option::Some(address) => address,
                Option::None => panic("trying to call the intrinsic memcpy() with an invalid dest pointer"),
            };
            let src: *mut u8 = match emu_get_memory_pointer(emulator, src) {
                Option::Some(address) => address,
                Option::None => panic("trying to call the intrinsic memcpy() with an invalid src pointer"),
            };
            unsafe { memcopy::<u8>(dest, src, len) };
            0 // returns void, so this is ignored
        },
    }
}

/// Execute a given list of instructions.
fn emu_execute_instructions<'a>(
    emulator: &mut Emu,
    ast: &LAst,
    locals: &mut StringMap<usize>,
    instructions: &'a Vec<Instruction>,
) -> ExecFlow<'a> {
    let mut i: usize = 0;
    while i < vec_len::<Instruction>(instructions) {
        let instruction: &Instruction = vec_at::<Instruction>(instructions, i);

        match instruction {
            Instruction::Assignment(assign) => emu_execute_assignment(emulator, ast, locals, assign),
            Instruction::Store(ty, value, address) => emu_execute_store(emulator, locals, ty, value, address),
            Instruction::Call(Call::Call(return_type, callee, arguments)) => {
                emu_execute_call(emulator, ast, locals, return_type, callee, arguments);
            },
            Instruction::Ret(return_type, return_value) => {
                return ExecFlow::Return(match return_value {
                    Option::Some(value) => {
                        let value: usize = emu_eval_value(emulator, locals, value);
                        ltype_overflow_value(value, return_type)
                    },
                    Option::None => 0,
                });
            },
            Instruction::Br(branch) => {
                return match branch {
                    Branch::Unconditional(target_label) => ExecFlow::Jump(target_label),
                    Branch::Conditional(condition, then_label, else_label) => {
                        let condition_value: usize = emu_eval_value(emulator, locals, condition);

                        if condition_value == 1 {
                            ExecFlow::Jump(then_label)
                        } else {
                            ExecFlow::Jump(else_label)
                        }
                    },
                };
            },
        }

        match emu_exit_code(emulator) {
            Option::Some(code) => return ExecFlow::Return(code),
            Option::None => {},
        };

        i = i + 1;
    }
    ExecFlow::Continue
}

/// Execute the given assignment instruction.
fn emu_execute_assignment(
    emulator: &mut Emu,
    ast: &LAst,
    locals: &mut StringMap<usize>,
    AssignInstruction::Assign(target, operation): &AssignInstruction,
) {
    let value: usize = emu_evaluate_assign_op(emulator, ast, locals, operation);
    stringMap_insert_or_update::<usize>(locals, target, value);
}

/// Evaluate the value of the assignment operation.
fn emu_evaluate_assign_op(emulator: &mut Emu, ast: &LAst, locals: &StringMap<usize>, op: &AssignOp) -> usize {
    match op {
        AssignOp::Binary(operator, result_type, left, right) => {
            let lhs: usize = ltype_overflow_value(emu_eval_value(emulator, locals, left), result_type);
            let rhs: usize = ltype_overflow_value(emu_eval_value(emulator, locals, right), result_type);
            match operator {
                BinaryOp::Add => lhs + rhs,
                BinaryOp::Sub => lhs - rhs,
                BinaryOp::Mul => lhs * rhs,
                BinaryOp::Udiv => lhs / rhs,
                BinaryOp::Urem => lhs % rhs,
            }
        },
        AssignOp::Icmp(predicate, operand_type, left, right) => {
            let lhs: usize = ltype_overflow_value(emu_eval_value(emulator, locals, left), operand_type);
            let rhs: usize = ltype_overflow_value(emu_eval_value(emulator, locals, right), operand_type);
            (match predicate {
                IcmpOp::Eq => lhs == rhs,
                IcmpOp::Ne => lhs != rhs,
                IcmpOp::Ugt => lhs > rhs,
                IcmpOp::Uge => lhs >= rhs,
                IcmpOp::Ult => lhs < rhs,
                IcmpOp::Ule => lhs <= rhs,
            }) as usize
        },
        AssignOp::Cast(to_type, value) => {
            // inttoptr/ptrtoint are reinterpretation (no-op)
            // zext is extending with zeros (assign larger type)
            // trunc is truncating most significant bits, achieved here by computing modulo
            let evaluated_value: usize = emu_eval_value(emulator, locals, value);
            ltype_overflow_value(evaluated_value, to_type)
        },
        AssignOp::Alloca(allocated_type, count) => {
            let space: usize = *count * lType_size(allocated_type);
            match emu_stack_alloc(emulator, space) {
                Option::Some(address) => address,
                Option::None => panic("Stack overflow encountered during emulation"),
            }
        },
        AssignOp::Load(loaded_type, address_value) => {
            let address: usize = emu_eval_value(emulator, locals, address_value);
            let value: usize = emu_load_bytes(emulator, address, lType_size(loaded_type));
            ltype_overflow_value(value, loaded_type)
        },
        AssignOp::Call(Call::Call(call_type, callee, arguments)) => {
            emu_execute_call(emulator, ast, locals, call_type, callee, arguments)
        },
    }
}

/// Execute an LLVM call and return the raw result value.
fn emu_execute_call(
    emulator: &mut Emu,
    ast: &LAst,
    locals: &StringMap<usize>,
    call_type: &LType,
    callee: &String,
    arguments: &Vec<LTypedValue>,
) -> usize {
    let mut arg_values: Vec<usize> = vec_with_capacity::<usize>(vec_len::<LTypedValue>(arguments));
    let mut i: usize = 0;
    while i < vec_len::<LTypedValue>(arguments) {
        let LTypedValue::Pair(ty, argument_value): &LTypedValue = vec_at::<LTypedValue>(arguments, i);
        let value: usize = emu_eval_value(emulator, locals, argument_value);
        vec_push::<usize>(&mut arg_values, ltype_overflow_value(value, ty));
        i = i + 1;
    }

    let value: usize = emu_execute_function(emulator, ast, callee, &arg_values);
    drop_vec::<usize>(arg_values);
    ltype_overflow_value(value, call_type)
}

/// Execute the given store instruction.
fn emu_execute_store(
    emulator: &mut Emu,
    locals: &StringMap<usize>,
    store_type: &LType,
    value: &LValue,
    address: &LValue,
) {
    let value: usize = ltype_overflow_value(emu_eval_value(emulator, locals, value), store_type);
    let target_address: usize = emu_eval_value(emulator, locals, address);
    let byte_count: usize = lType_size(store_type);

    if not(emu_store_bytes(emulator, target_address, value, byte_count)) {
        panic("invalid LLVM store address");
    }
}

/// Evaluate the value of a virtual register, global name or literal.
fn emu_eval_value(emulator: &Emu, locals: &StringMap<usize>, value: &LValue) -> usize {
    match value {
        LValue::Literal(number) => *number,
        LValue::Register(name) => match stringMap_get::<usize>(locals, name) {
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

/// Rounds `n` up to the next multiple of `multiple`
fn round_to_next_multiple(n: usize, multiple: usize) -> usize {
    if n % 8 == 0 {
        n
    } else {
        n + (multiple - (n % multiple))
    }
}

// -------------------------- Args ---------------------------------

enum Args {
    /// arguments, cursor
    Args(Vec<String>),
}

fn args_new(argc: usize, argv: *mut *mut u8) -> Args {
    let mut args: Vec<String> = vec_with_capacity::<String>(argc);
    unsafe {
        let mut i: usize = 0;
        while i < argc {
            let mut string: String = string_new();
            let arg: *mut u8 = *ptr_add::<*mut u8>(argv, i);

            let mut j: usize = 0;
            while *ptr_add::<u8>(arg, j) != 0 as u8 {
                string_push(&mut string, *ptr_add::<u8>(arg, j) as char);
                j = j + 1;
            }

            vec_push::<String>(&mut args, string);
            i = i + 1;
        }
    }
    Args::Args(args)
}

fn args_len(Args::Args(args): &Args) -> usize {
    vec_len::<String>(args)
}

/// Get the argument at index `index`.
fn args_at(Args::Args(args): &Args, index: usize) -> &String {
    vec_at::<String>(args, index)
}

/// Return true if the argument at index `index` matches `other` with bounds-checking.
fn arg_eq(args: &Args, index: usize, other: &str) -> bool {
    if index < args_len(args) {
        str_eq(args_at(args, index), other)
    } else {
        false
    }
}

/// Create a new Args using the arguments from index `i` onwards.
fn args_subargs(args: &Args, i: usize) -> Args {
    let mut name: String = string_new();
    let mut j: usize = 0;
    while j < i {
        string_push_string(&mut name, args_at(args, j));
        j = j + 1;
        if j < i {
            string_push(&mut name, ' ');
        }
    }
    let mut arguments: Vec<String> = vec_with_capacity::<String>(args_len(args) - i);
    vec_push::<String>(&mut arguments, name);
    while j < args_len(args) {
        let argument: &String = args_at(args, j);
        vec_push::<String>(&mut arguments, string_clone(argument));
        j = j + 1;
    }
    Args::Args(arguments)
}

// -------------------------- Error --------------------------------

/// Panic by printing a message and exiting the program.
fn panic(message: &str) -> ! {
    print_str("panic: ");
    print_str(message);
    println();
    exit_process(1);
}

/// Used to indicate unreachable code.
fn unreachable() -> ! {
    panic("unreachable code was reached")
}

/// Report an error message with source location and exit.
fn report_error(file: &SourceFile, message: &String) -> ! {
    let line: usize = sourceFile_current_line(file);
    let col: usize = sourceFile_current_column(file);

    print_str("Error at ");
    print_string(&integer_to_string(line));
    print_str(":");
    print_string(&integer_to_string(col));
    print_str(":\n");

    let mut line: String = string_new();
    let mut start: usize = sourceFile_current_line_start(file);
    let mut reached_end: bool = false;
    while not(reached_end) {
        match sourceFile_get_char(file, start) {
            Option::Some(c) => {
                if c == '\n' {
                    reached_end = true;
                } else {
                    string_push(&mut line, c);
                }
            },
            Option::None => reached_end = true,
        }
        start = start + 1;
    }
    print_string(&line);
    println();

    let mut i: usize = 1;
    while i < col {
        print_str(" ");
        i = i + 1;
    }
    print_str("^ ");
    print_string(message);
    println();
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
    print_string(&header);

    let mut start: usize = sourceFile_current_line_start(file);
    let mut reached_end: bool = false;
    let mut line_content: String = string_new();
    while not(reached_end) {
        match sourceFile_get_char(file, start) {
            Option::Some(c) => {
                if c == '\n' {
                    reached_end = true;
                } else {
                    string_push(&mut line_content, c);
                }
            },
            Option::None => reached_end = true,
        }
        start = start + 1;
    }
    print_string(&line_content);
    print_str("\n");

    let mut i: usize = 1;
    while i < col {
        print_str(" ");
        i = i + 1;
    }
    print_str("^ ");
    print_string(message);
    print_str("\n");
}

fn rLexer_error(lexer: &RLexer, message: &String) -> ! {
    report_error(rLexer_sourcefile(lexer), message)
}

/// Emit an error at the parser current location and abort.
fn parse_error(lexer: &RLexer, message: &String) -> ! {
    rLexer_error(lexer, message)
}

fn semantic_error(message: &String) -> ! {
    print_str("Semantic error: ");
    print_string(message);
    println();
    exit_process(1)
}

fn lLexer_error(lexer: &LLexer, message: &String) -> ! {
    report_error(lLexer_sourcefile(lexer), message)
}

/// Emit an LLVM parser error and panic.
fn lparser_error(parser: &LParser, message: &String) -> ! {
    let file: &SourceFile = lLexer_sourcefile(lparser_lexer(parser));
    report_error(file, message)
}

/// Emit an LLVM parser warning and continue.
fn lparser_warning(parser: &LParser, message: &String) {
    let file: &SourceFile = lLexer_sourcefile(lparser_lexer(parser));
    report_warning(file, message)
}

fn lparser_expected_message(parser: &LParser, expected: &String) -> String {
    let mut message: String = string("expected ");
    string_push_string(&mut message, expected);
    let token: &LToken = lparser_current_token(parser);
    string_push_str(&mut message, ", but got: ");
    string_push_string(&mut message, &lToken_to_string(token));
    message
}

// -------------------------- bool ---------------------------------

/// Logical AND of two booleans. Not lazy due to parameters being pass by value.
fn and(a: bool, b: bool) -> bool {
    a as usize + b as usize == 2
}

/// Logical OR of two booleans. Not lazy due to parameters being pass by value.
fn or(a: bool, b: bool) -> bool {
    a as usize + b as usize > 0
}

/// Logical NOT of one boolean.
fn not(a: bool) -> bool {
    a as usize == 0
}

// -------------------------- char ---------------------------------

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

/// Check whether the given character can be used in an LLLVM-IR identifier.
fn is_llvm_identifier(ch: char) -> bool {
    or(is_alphanumeric(ch), or(ch == '.', ch == '$'))
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

/// Convert a digit into an ascii digit.
fn digit_to_ascii(digit: u8) -> char {
    if digit < 10 as u8 {
        ('0' as u8 + digit) as char
    } else {
        ('A' as u8 + (digit - 10 as u8)) as char
    }
}

// ------------------------ Option<T> ------------------------------

/// Optional type that can contain some value with type T or no value.
enum Option<T> {
    Some(T),
    None,
}

/// Return true if the Option is of variant Some
fn is_some<T>(opt: &Option<T>) -> bool {
    match opt {
        Option::Some(_) => true,
        _ => false,
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

// -------------------------- Box<T> ------------------------------

/// Pointer to heap that owns its value.
enum Box<T> {
    Ptr(*mut T),
}

/// Allocate and box a value on the heap.
fn box_new<T>(value: T) -> Box<T> {
    let ptr: *mut T = unsafe { alloc::<T>(1) };
    unsafe { *ptr = value };
    Box::<T>::Ptr(ptr)
}

/// Dereference a box.
fn box_deref<T>(Box::Ptr(ptr): &Box<T>) -> &T {
    unsafe { &**ptr }
}

// -------------------------- Vec<T> ------------------------------

/// Generic contiguous growable buffer.
enum Vec<T> {
    /// start, length, capacity
    Vec(*mut T, usize, usize),
}

/// Create an empty vector.
fn vec_new<T>() -> Vec<T> {
    Vec::<T>::Vec(0 as *mut T, 0, 0)
}

/// Create a vector with fixed starting capacity.
fn vec_with_capacity<T>(initial_capacity: usize) -> Vec<T> {
    if initial_capacity > 0 {
        let ptr: *mut T = unsafe { alloc::<T>(initial_capacity) };
        Vec::<T>::Vec(ptr, 0, initial_capacity)
    } else {
        Vec::<T>::Vec(0 as *mut T, 0, 0)
    }
}

/// Create a vector with a fixed initial length.
/// The caller must ensure to not read the vector's elements before initialising them.
unsafe fn vec_with_len<T>(len: usize) -> Vec<T> {
    let Vec::Vec(ptr, _, capacity): Vec<T> = vec_with_capacity::<T>(len);
    Vec::<T>::Vec(ptr, len, capacity)
}

/// Get the backing pointer.
/// The caller must ensure to not mutate the vector during the use of this pointer.
/// Otherwise, the vector may be reallocated, causing this pointer to become a dangling pointer.
unsafe fn vec_ptr<T>(Vec::Vec(ptr, _, _): &Vec<T>) -> *mut T {
    *ptr
}

/// Get the logical length.
fn vec_len<T>(Vec::Vec(_, len, _): &Vec<T>) -> usize {
    *len
}

/// Ensure capacity for extra elements.
fn vec_accomodate_extra_space<T>(Vec::Vec(ptr, len, cap): &mut Vec<T>, space: usize) {
    if *cap < *len + space {
        *cap = max(*cap, 1);
        while *cap < *len + space {
            *cap = *cap * 2;
        }
        unsafe {
            let new_ptr: *mut T = alloc::<T>(*cap);
            memcopy::<T>(new_ptr, *ptr, *len);
            free(*ptr as *mut u8);
            *ptr = new_ptr;
        };
    }
}

/// Append one element.
fn vec_push<T>(vec: &mut Vec<T>, value: T) {
    vec_accomodate_extra_space::<T>(vec, 1);
    let Vec::Vec(ptr, len, _): &mut Vec<T> = vec;
    unsafe { *ptr_add::<T>(*ptr, *len) = value };
    *len = *len + 1;
}

/// Set vector length after writing raw bytes/elements.
fn vec_set_len<T>(Vec::Vec(_, old_len, capacity): &mut Vec<T>, len: usize) {
    if len > *capacity {
        panic("Trying to set the length of a vector to more than its capacity")
    };
    *old_len = len;
}

/// Get an immutable reference to an element by index.
fn vec_get<T>(vec: &Vec<T>, index: usize) -> Option<&T> {
    if index >= vec_len::<T>(vec) {
        Option::<&T>::None
    } else {
        unsafe { Option::<&T>::Some(&*ptr_add::<T>(vec_ptr::<T>(vec), index)) }
    }
}

/// Get a mutable reference to an element by index.
fn vec_get_mut<T>(vec: &mut Vec<T>, index: usize) -> Option<&mut T> {
    if index >= vec_len::<T>(vec) {
        Option::<&mut T>::None
    } else {
        unsafe { Option::<&mut T>::Some(&mut *ptr_add::<T>(vec_ptr::<T>(vec), index)) }
    }
}

/// Get an immutable reference to an element by index.
/// Panics, if the index is out of bounds.
fn vec_at<T>(vec: &Vec<T>, index: usize) -> &T {
    if index >= vec_len::<T>(vec) {
        panic("Out-of-bounds vector access\n");
    }
    unsafe { &*ptr_add::<T>(vec_ptr::<T>(vec), index) }
}

/// Get a mutable reference to an element by index.
/// Panics, if the index is out of bounds.
fn vec_at_mut<T>(vec: &mut Vec<T>, index: usize) -> &mut T {
    if index >= vec_len::<T>(vec) {
        panic("Out-of-bounds vector access!");
    }
    unsafe { &mut *ptr_add::<T>(vec_ptr::<T>(vec), index) }
}

/// Set a value at the given index. Return false if the index is out of bounds.
fn vec_set<T>(vec: &mut Vec<T>, index: usize, value: T) -> bool {
    if index >= vec_len::<T>(vec) {
        false
    } else {
        *vec_at_mut::<T>(vec, index) = value;
        true
    }
}

/// Append all elements from one vector to another.
fn vec_extend<T>(vec: &mut Vec<T>, other: &Vec<T>) {
    let len: usize = vec_len::<T>(vec);
    let other_len: usize = vec_len::<T>(other);
    vec_accomodate_extra_space::<T>(vec, other_len);
    unsafe {
        let dest: *mut T = ptr_add::<T>(vec_ptr::<T>(vec), len);
        let src: *mut T = vec_ptr::<T>(other);
        memcopy::<T>(dest, src, other_len);
    };
    vec_set_len::<T>(vec, len + other_len);
}

/// Remove the element at index `index` and return true if it was removed.
fn vec_remove<T>(Vec::Vec(ptr, len, _): &mut Vec<T>, index: usize) -> bool {
    if or(index >= *len, *len == 0) {
        return false;
    }
    unsafe {
        let start: *mut T = ptr_add::<T>(*ptr, index);
        let after_index: *mut T = ptr_add::<T>(*ptr, index + 1);
        memcopy::<T>(start, after_index, *len - index - 1);

        *len = *len - 1;
    }
    true
}

// ----------------------- StringMap<T> ---------------------------

/// Bucket entry for StringMap.
enum StringMapEntry<T> {
    Entry(String, T),
}

/// Get the key stored in one StringMapEntry.
fn stringMapEntry_get_key<T>(StringMapEntry::Entry(key, _): &StringMapEntry<T>) -> &String {
    key
}

/// Get the value stored in one StringMapEntry via a shared reference.
fn stringMapEntry_get_value<T>(StringMapEntry::Entry(_, value): &StringMapEntry<T>) -> &T {
    value
}

/// Get the value stored in one StringMapEntry via a mutable reference.
fn stringMapEntry_get_value_mut<T>(StringMapEntry::Entry(_, value): &mut StringMapEntry<T>) -> &mut T {
    value
}

/// Hash map from String keys to generic values.
enum StringMap<T> {
    /// an array of vectors which store the entries in reverse order, i.e. they are traversed from
    /// the last index to the first index.
    Map(Vec<Vec<StringMapEntry<T>>>),
}

/// Create a map with default len.
fn stringMap_new<T>() -> StringMap<T> {
    stringMap_with_len::<T>(100)
}

/// Create a map with explicit len.
fn stringMap_with_len<T>(len: usize) -> StringMap<T> {
    let bucket_len: usize = max(1, len);
    let mut buckets: Vec<Vec<StringMapEntry<T>>> = vec_with_capacity::<Vec<StringMapEntry<T>>>(bucket_len);
    let mut i: usize = 0;
    while i < bucket_len {
        vec_push::<Vec<StringMapEntry<T>>>(&mut buckets, vec_new::<StringMapEntry<T>>());
        i = i + 1;
    }
    StringMap::<T>::Map(buckets)
}

/// Insert a key/value pair into the map by prepending it. This will ignore prexisting entries,
/// making it less efficient if there are duplicate keys, but more efficient if it is guaranteed
/// to be unique (since the collision list does not have to be traversed).
fn stringMap_insert<T>(map: &mut StringMap<T>, key: String, value: T) {
    let bucket: &mut Vec<StringMapEntry<T>> = stringMap_bucket_mut::<T>(map, &key);
    vec_push::<StringMapEntry<T>>(bucket, StringMapEntry::<T>::Entry(key, value));
}

/// Insert a key/value pair into the map or update the value if the key is already present.
/// Returns true if the key was not yet present and hence newly inserted.
fn stringMap_insert_or_update<T>(map: &mut StringMap<T>, key: &String, value: T) -> bool {
    let bucket: &mut Vec<StringMapEntry<T>> = stringMap_bucket_mut::<T>(map, key);
    let mut nth: usize = vec_len::<StringMapEntry<T>>(bucket);
    while nth > 0 {
        let StringMapEntry::Entry(other_key, entry_value): &mut StringMapEntry<T> =
            vec_at_mut::<StringMapEntry<T>>(bucket, nth - 1);
        if string_eq(key, other_key) {
            *entry_value = value;
            return false;
        }
        nth = nth - 1;
    }
    vec_push::<StringMapEntry<T>>(bucket, StringMapEntry::<T>::Entry(string_clone(key), value));
    true
}

/// Get a shared reference to the value for a key.
fn stringMap_get<'a, T>(map: &'a StringMap<T>, key: &String) -> Option<&'a T> {
    let StringMap::Map(buckets): &StringMap<T> = map;
    let bucket_index: usize = string_hash(key, vec_len::<Vec<StringMapEntry<T>>>(buckets));
    let bucket: &Vec<StringMapEntry<T>> = vec_at::<Vec<StringMapEntry<T>>>(buckets, bucket_index);

    let mut nth: usize = vec_len::<StringMapEntry<T>>(bucket); // traverse backwards due to construction of the collision list
    while nth > 0 {
        let entry: &StringMapEntry<T> = vec_at::<StringMapEntry<T>>(bucket, nth - 1);
        let other_key: &String = stringMapEntry_get_key::<T>(entry);
        if string_eq(other_key, key) {
            return Option::<&T>::Some(stringMapEntry_get_value::<T>(entry));
        }
        nth = nth - 1;
    }
    Option::<&T>::None
}

/// Get a mutable reference to the value for a key.
fn stringMap_get_mut<'a, T>(map: &'a mut StringMap<T>, key: &String) -> Option<&'a mut T> {
    let bucket: &mut Vec<StringMapEntry<T>> = stringMap_bucket_mut::<T>(map, key);

    let mut nth: usize = vec_len::<StringMapEntry<T>>(bucket); // traverse backwards due to construction of the collision list
    while nth > 0 {
        let other_key: &String =
            stringMapEntry_get_key::<T>(vec_at_mut::<StringMapEntry<T>>(bucket, nth - 1));
        if string_eq(other_key, key) {
            return Option::<&mut T>::Some(stringMapEntry_get_value_mut::<T>(
                vec_at_mut::<StringMapEntry<T>>(bucket, nth - 1),
            ));
        }
        nth = nth - 1;
    }
    Option::<&mut T>::None
}

/// Remove the first entry with key `key` and return true if it was removed.
fn stringMap_remove<T>(map: &mut StringMap<T>, key: &String) -> bool {
    let bucket: &mut Vec<StringMapEntry<T>> = stringMap_bucket_mut::<T>(map, key);

    let mut nth: usize = vec_len::<StringMapEntry<T>>(bucket); // traverse backwards due to construction of the collision list
    while nth > 0 {
        let entry: &StringMapEntry<T> = vec_at::<StringMapEntry<T>>(bucket, nth - 1);
        let other_key: &String = stringMapEntry_get_key::<T>(entry);
        if string_eq(other_key, key) {
            vec_remove::<StringMapEntry<T>>(bucket, nth - 1);
            return true;
        }
        nth = nth - 1;
    }
    false
}

/// Get a mutable reference to the bucket by hashing the given key `k` and indexing into the hashtable `b`.
fn stringMap_bucket_mut<'a, T>(
    StringMap::Map(b): &'a mut StringMap<T>,
    k: &String,
) -> &'a mut Vec<StringMapEntry<T>> {
    let bucket_index: usize = string_hash(k, vec_len::<Vec<StringMapEntry<T>>>(b));
    vec_at_mut::<Vec<StringMapEntry<T>>>(b, bucket_index)
}

// ---------------------- StringMapStack<T> -----------------------
//
// -> A stack of StringMap<T> which inserts/looks-up by stack order.

/// Stack of StringMap scopes.
enum StringMapStack<T> {
    Stack(Vec<StringMap<T>>, usize),
}

/// Create an empty StringMap stack.
fn stringMapStack_new<T>() -> StringMapStack<T> {
    StringMapStack::<T>::Stack(vec_new::<StringMap<T>>(), 0)
}

/// Push a new empty scope.
fn stringMapStack_push_empty<T>(StringMapStack::Stack(scopes, top): &mut StringMapStack<T>) {
    let new_scope: StringMap<T> = stringMap_new::<T>();
    if *top == vec_len::<StringMap<T>>(scopes) {
        vec_push::<StringMap<T>>(scopes, new_scope);
    } else {
        vec_set::<StringMap<T>>(scopes, *top, new_scope);
    };
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
fn stringMapStack_insert<T>(stack: &mut StringMapStack<T>, name: &String, value: T) -> bool {
    let StringMapStack::Stack(scopes, top): &mut StringMapStack<T> = stack;
    if *top == 0 {
        return true;
    }

    let idx: usize = *top - 1;
    let scope: &mut StringMap<T> = vec_at_mut::<StringMap<T>>(scopes, idx);
    not(stringMap_insert_or_update::<T>(scope, name, value))
}

/// Look up a value in any scope.
fn stringMapStack_get<'a, T>(stack: &'a StringMapStack<T>, name: &String) -> Option<&'a T> {
    let StringMapStack::Stack(scopes, top): &StringMapStack<T> = stack;
    let mut i: usize = *top;
    while i > 0 {
        i = i - 1;
        let scope: &StringMap<T> = vec_at::<StringMap<T>>(scopes, i);
        match stringMap_get::<T>(scope, name) {
            Option::Some(value) => return Option::<&T>::Some(value),
            Option::None => {},
        }
    }
    Option::<&T>::None
}

// --------------------------- Eq ---------------------------------

fn lType_eq(left: &LType, right: &LType) -> bool {
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
        LType::Void => match right {
            LType::Void => true,
            _ => false,
        },
    }
}

/// Check if two tokens are equal.
fn rToken_eq(a: &RToken, b: &RToken) -> bool {
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
        RToken::Eq => match b {
            RToken::Eq => true,
            _ => false,
        },
        RToken::Neq => match b {
            RToken::Neq => true,
            _ => false,
        },
        RToken::LAngle => match b {
            RToken::LAngle => true,
            _ => false,
        },
        RToken::RAngle => match b {
            RToken::RAngle => true,
            _ => false,
        },
        RToken::Leq => match b {
            RToken::Leq => true,
            _ => false,
        },
        RToken::Geq => match b {
            RToken::Geq => true,
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
        RToken::Arrow => match b {
            RToken::Arrow => true,
            _ => false,
        },
        RToken::Lifetime => match b {
            RToken::Lifetime => true,
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

/// Check if two literal tokens are equal.
fn rLiteral_eq(left: &RLiteral, right: &RLiteral) -> bool {
    match left {
        RLiteral::Int(left_value) => match right {
            RLiteral::Int(right_value) => *left_value == *right_value,
            _ => false,
        },
        RLiteral::String(left_value) => match right {
            RLiteral::String(right_value) => string_eq(left_value, right_value),
            _ => false,
        },
        RLiteral::Char(left_value) => match right {
            RLiteral::Char(right_value) => *left_value == *right_value,
            _ => false,
        },
        RLiteral::Bool(left_value) => match right {
            RLiteral::Bool(right_value) => *left_value == *right_value,
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
        RType::Enum(left, left_generic) => match b {
            RType::Enum(right, right_generic) => {
                if not(string_eq(left, right)) {
                    return false;
                }
                match left_generic {
                    Option::Some(ty) => match right_generic {
                        Option::Some(other_ty) => {
                            rType_eq(box_deref::<RType>(ty), box_deref::<RType>(other_ty))
                        },
                        _ => false,
                    },
                    _ => match right_generic {
                        Option::None => true,
                        _ => false,
                    },
                }
            },
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
            RType::RawPointerMut(right) => rType_eq(box_deref::<RType>(left), box_deref::<RType>(right)),
            _ => false,
        },
        RType::Generic => match b {
            RType::Generic => true,
            _ => false,
        },
    }
}

/// Check two LLVM tokens for equality.
fn lToken_eq(left: &LToken, right: &LToken) -> bool {
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
        LToken::X => match right {
            LToken::X => true,
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
        LToken::CString(left_value) => match right {
            LToken::CString(right_value) => string_eq(left_value, right_value),
            _ => false,
        },
        LToken::Local(left_name) => match right {
            LToken::Local(right_name) => string_eq(left_name, right_name),
            _ => false,
        },
        LToken::Global(left_name) => match right {
            LToken::Global(right_name) => string_eq(left_name, right_name),
            _ => false,
        },
        LToken::LabelIdent(left_name) => match right {
            LToken::LabelIdent(right_name) => string_eq(left_name, right_name),
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
        let c1: char = string_at(s1, i);
        let c2: char = string_at(s2, i);
        if c1 != c2 {
            return false;
        }
        i = i + 1;
    }
    true
}

/// Check if a `String` and a `&str` are equal.
fn str_eq(s1: &String, s2: &str) -> bool {
    let len: usize = string_len(s1);
    if len != str::len(s2) {
        return false;
    }
    let mut i: usize = 0;
    while i < len {
        let c1: char = string_at(s1, i);
        let c2: char = str_at(s2, i);
        if c1 != c2 {
            return false;
        }
        i = i + 1;
    }
    true
}

// ------------------------- Clone --------------------------------

/// Clone a Rust token.
fn rToken_clone(token: &RToken) -> RToken {
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
        RToken::Eq => RToken::Eq,
        RToken::Neq => RToken::Neq,
        RToken::LAngle => RToken::LAngle,
        RToken::RAngle => RToken::RAngle,
        RToken::Leq => RToken::Leq,
        RToken::Geq => RToken::Geq,
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
        RToken::Arrow => RToken::Arrow,
        RToken::Lifetime => RToken::Lifetime,
        RToken::Literal(literal) => RToken::Literal(rLiteral_clone(literal)),
        RToken::Identifier(value) => RToken::Identifier(string_clone(value)),
        RToken::Eof => RToken::Eof,
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

/// Clone a Rust AST type value.
fn rType_clone(t: &RType) -> RType {
    match t {
        RType::U8 => RType::U8,
        RType::Usize => RType::Usize,
        RType::Bool => RType::Bool,
        RType::Char => RType::Char,
        RType::Unit => RType::Unit,
        RType::Never => RType::Never,
        RType::Enum(name, generic) => {
            let generic: Option<Box<RType>> = match generic {
                Option::Some(inner) => {
                    let inner: &RType = box_deref::<RType>(inner);
                    Option::<Box<RType>>::Some(box_new::<RType>(rType_clone(inner)))
                },
                _ => Option::<Box<RType>>::None,
            };
            RType::Enum(string_clone(name), generic)
        },
        RType::Reference(inner, mutable) => {
            RType::Reference(box_new::<RType>(rType_clone(box_deref::<RType>(inner))), *mutable)
        },
        RType::RawPointerMut(inner) => {
            RType::RawPointerMut(box_new::<RType>(rType_clone(box_deref::<RType>(inner))))
        },
        RType::Generic => RType::Generic,
    }
}

/// Clone a vector of Rust types.
fn types_clone(types: &Vec<RType>) -> Vec<RType> {
    let mut clone: Vec<RType> = vec_with_capacity::<RType>(vec_len::<RType>(types));
    let mut i: usize = 0;
    while i < vec_len::<RType>(types) {
        let ty: RType = rType_clone(vec_at::<RType>(types, i));
        vec_push::<RType>(&mut clone, ty);
        i = i + 1;
    }
    clone
}

/// Clone a STPair
fn stPair_clone(STPair::ST(string, ty): &STPair) -> STPair {
    STPair::ST(string_clone(string), rType_clone(ty))
}

/// Serves as dummy. A correct program should never be able to cause execution of this.
fn stPair_unreachable() -> STPair {
    STPair::ST(string_new(), RType::Unit)
}

/// Clone an LLVM token.
fn lToken_clone(token: &LToken) -> LToken {
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
        LToken::X => LToken::X,
        LToken::LParen => LToken::LParen,
        LToken::RParen => LToken::RParen,
        LToken::LBrace => LToken::LBrace,
        LToken::RBrace => LToken::RBrace,
        LToken::LBracket => LToken::LBracket,
        LToken::RBracket => LToken::RBracket,
        LToken::Comma => LToken::Comma,
        LToken::Assign => LToken::Assign,
        LToken::CString(value) => LToken::CString(string_clone(value)),
        LToken::Local(name) => LToken::Local(string_clone(name)),
        LToken::Global(name) => LToken::Global(string_clone(name)),
        LToken::LabelIdent(name) => LToken::LabelIdent(string_clone(name)),
        LToken::Integer(value) => LToken::Integer(*value),
        LToken::Eof => LToken::Eof,
    }
}

/// Clone an LLVM type.
fn lType_clone(ty: &LType) -> LType {
    match ty {
        LType::I1 => LType::I1,
        LType::I8 => LType::I8,
        LType::I64 => LType::I64,
        LType::Ptr => LType::Ptr,
        LType::Void => LType::Void,
    }
}

/// Clone a string.
fn string_clone(String::Inner(Vec::Vec(ptr, len, _)): &String) -> String {
    let mut clone: Vec<u8> = vec_with_capacity::<u8>(*len);
    unsafe {
        memcopy::<u8>(vec_ptr::<u8>(&clone), *ptr, *len);
        vec_set_len::<u8>(&mut clone, *len);
    };
    String::Inner(clone)
}

// --------------------------------- Drop ---------------------------------

/// Drop a Vec, i.e. deallocate the underlying buffer. This only frees the memory used for the buffer. If
/// the elements own memory, it is disregarded and leaked (e.g. Vec<String>). If that is the case, then
/// a custom Drop implementation is more suitable than this function.
fn drop_vec<T>(Vec::Vec(ptr, _, _): Vec<T>) {
    unsafe { free(ptr as *mut u8) }
}

/// Drop a StringMap<Value> map, i.e. deallocate all associated values.
fn drop_stringValueMap(StringMap::Map(buckets): StringMap<usize>) {
    let mut i: usize = 0;
    while i < vec_len::<Vec<StringMapEntry<usize>>>(&buckets) {
        unsafe {
            let bucket: &Vec<StringMapEntry<usize>> = vec_at::<Vec<StringMapEntry<usize>>>(&buckets, i);
            let mut j: usize = 0;
            while j < vec_len::<StringMapEntry<usize>>(bucket) {
                let StringMapEntry::Entry(name, _): &StringMapEntry<usize> =
                    vec_at::<StringMapEntry<usize>>(bucket, j);
                let String::Inner(Vec::Vec(str_ptr, _, _)): &String = name;
                free(*str_ptr);
                j = j + 1;
            }
            free(vec_ptr::<StringMapEntry<usize>>(bucket) as *mut u8);
        }
        i = i + 1;
    }
    drop_vec::<Vec<StringMapEntry<usize>>>(buckets);
}

// ------------------------- String -------------------------------

/// A growable ASCII string.
enum String {
    Inner(Vec<u8>),
}

/// Create a new empty string.
fn string_new() -> String {
    String::Inner(vec_new::<u8>())
}

/// Create a new string with the specified capacity
fn string_with_capacity(initial_capacity: usize) -> String {
    String::Inner(vec_with_capacity::<u8>(initial_capacity))
}

/// Create an owned string from a string slice.
fn string(str: &str) -> String {
    let mut s: String = string_with_capacity(str::len(str));
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
        Option::Some(value) => Option::<char>::Some(*value as char),
        Option::None => Option::<char>::None,
    }
}

/// Get the character at the given index and panic if the index is out of bounds.
fn string_at(String::Inner(bytes): &String, index: usize) -> char {
    *vec_at::<u8>(bytes, index) as char
}

/// Get the character at the given index and panic if the index is out of bounds.
fn str_at(str: &str, index: usize) -> char {
    if index >= str::len(str) {
        panic("out-of-bounds &str index");
    }
    unsafe { *ptr_add::<u8>(str::as_ptr(str) as *mut u8, index) as char }
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
    unsafe {
        let dest: *mut u8 = ptr_add::<u8>(vec_ptr::<u8>(bytes), len);
        memcopy::<u8>(dest, str_ptr, str_len);
        vec_set_len::<u8>(bytes, len + str_len);
    };
}

/// Push a string onto another string.
fn string_push_string(String::Inner(bytes): &mut String, String::Inner(other_bytes): &String) {
    vec_extend::<u8>(bytes, other_bytes);
}

/// Replace all characters in `string` that are contained in `old_chars` and replace them with their
/// counterpart in `new_chars` based on the corresponding index.
fn string_replace_all(string: &mut String, old_chars: &str, new_chars: &str) {
    if str::len(old_chars) != str::len(new_chars) {
        return;
    }
    let mut i: usize = 0;
    while i < string_len(string) {
        let c: char = string_at(string, i);
        let mut j: usize = 0;
        while j < str::len(old_chars) {
            if c == str_at(old_chars, j) {
                string_set(string, i, str_at(new_chars, j));
            }
            j = j + 1;
        }
        i = i + 1;
    }
}

/// Converts a string into an integer given the base.
/// Returns None if the integer contained in the string is invalid for 64-bit integers.
fn string_to_integer(string: &String, base: usize) -> Option<usize> {
    let mut value: usize = 0;

    let mut i: usize = 0;
    while i < string_len(string) {
        let digit: char = string_at(string, i);

        let digit_value: usize = if is_digit(digit) {
            digit as usize - '0' as usize
        } else {
            digit as usize - 'A' as usize + 10
        };

        let max: usize = 18446744073709551615; // 2^64 - 1

        if or(digit_value > base - 1, value > max / base) {
            return Option::<usize>::None;
        }

        value = value * base + digit_value;

        i = i + 1;
    }
    Option::<usize>::Some(value)
}

/// Add as many leading zeros as are needed to reach `digits` digits.
fn string_integer_extend(integer: &String, digits: usize) -> String {
    if string_len(integer) >= digits {
        return string_clone(integer);
    }
    let mut s: String = string("0");
    while string_len(&s) + string_len(integer) < digits {
        string_push(&mut s, '0');
    }
    let mut i: usize = 0;
    while i < string_len(integer) {
        string_push(&mut s, string_at(integer, i));
        i = i + 1;
    }
    s
}

/// Hash a String.
fn string_hash(string: &String, bucket_count: usize) -> usize {
    if bucket_count == 0 {
        return 0;
    }
    let mut hash: usize = 0;
    let mut i: usize = 0;
    while i < string_len(string) {
        let character: usize = string_at(string, i) as usize;
        hash = hash * 67 + character;
        i = i + 1;
    }
    hash % bucket_count
}

// -------------------- Display (to_string()) ----------------------

/// Convert a decimal integer into a string.
fn integer_to_string(integer: usize) -> String {
    integer_to_string_base(integer, 10)
}

/// Convert an integer into a string.
fn integer_to_string_base(integer: usize, base: usize) -> String {
    if integer == 0 {
        string("0")
    } else {
        let mut number: String = string_new();
        int2string(&mut number, integer, base);
        number
    }
}

// Converts integer > 0 to strings by appending it to `number`.
fn int2string(number: &mut String, integer: usize, base: usize) {
    if integer == 0 {
        return;
    }
    int2string(number, integer / base, base);
    let digit: u8 = (integer % base) as u8;
    string_push(number, digit_to_ascii(digit));
}

/// Convert a token into a string.
fn rToken_to_string(token: &RToken) -> String {
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
        RToken::Eq => string("=="),
        RToken::Neq => string("!="),
        RToken::LAngle => string("<"),
        RToken::RAngle => string(">"),
        RToken::Leq => string("<="),
        RToken::Geq => string(">="),
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
        RToken::Arrow => string("->"),
        RToken::Lifetime => string("'.."),
        RToken::Literal(literal) => rLiteral_to_string(literal),
        RToken::Identifier(name) => string_clone(name),
        RToken::Eof => string("<eof>"),
    }
}

/// Convert a Rust type into a string.
fn rType_to_string(ty: &RType) -> String {
    match ty {
        RType::U8 => string("u8"),
        RType::Usize => string("usize"),
        RType::Bool => string("bool"),
        RType::Char => string("char"),
        RType::Unit => string("()"),
        RType::Never => string("!"),
        RType::Enum(name, generic) => {
            let mut type_name: String = string_clone(name);
            match generic {
                Option::Some(inner) => {
                    let inner: &RType = box_deref::<RType>(inner);
                    string_push(&mut type_name, '<');
                    string_push_string(&mut type_name, &rType_to_string(inner));
                    string_push(&mut type_name, '>');
                },
                _ => {},
            }
            type_name
        },
        RType::Reference(inner, mutable) => {
            let mut str: String = if *mutable { string("&mut ") } else { string("&") };
            string_push_string(&mut str, &rType_to_string(box_deref::<RType>(inner)));
            str
        },
        RType::RawPointerMut(inner) => {
            let mut str: String = string("*mut ");
            string_push_string(&mut str, &rType_to_string(box_deref::<RType>(inner)));
            str
        },
        RType::Generic => string("T"), // generics can only use parameter "T"
    }
}

/// Convert an LLVM token into a string.
fn lToken_to_string(token: &LToken) -> String {
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
        LToken::X => string("x"),
        LToken::LParen => string("("),
        LToken::RParen => string(")"),
        LToken::LBrace => string("{"),
        LToken::RBrace => string("}"),
        LToken::LBracket => string("["),
        LToken::RBracket => string("]"),
        LToken::Comma => string(","),
        LToken::Assign => string("="),
        LToken::CString(value) => {
            let mut string: String = string_new();
            string_push_str(&mut string, "c\"");
            string_push_string(&mut string, value);
            string_push(&mut string, '"');
            string
        },
        LToken::Local(name) => {
            let mut string: String = string("%");
            string_push_string(&mut string, name);
            string
        },
        LToken::Global(name) => {
            let mut string: String = string("@");
            string_push_string(&mut string, name);
            string
        },
        LToken::LabelIdent(name) => {
            let mut string: String = string_clone(name);
            string_push(&mut string, ':');
            string
        },
        LToken::Integer(value) => integer_to_string(*value),
        LToken::Eof => string("<eof>"),
    }
}

fn lType_to_str(ty: &LType) -> &'static str {
    match ty {
        LType::I1 => "i1",
        LType::I8 => "i8",
        LType::I64 => "i64",
        LType::Ptr => "ptr",
        LType::Void => "void",
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

/// Given a path separated by `/`, return the filename.
fn path_to_file_with_ending(path: &String, ending: &str) -> String {
    let mut start: usize = 0;
    let mut dot_index: usize = string_len(path);
    let mut i: usize = 0;
    while i < string_len(path) {
        let c: char = string_at(path, i);
        if c == '/' {
            start = i + 1;
        } else if c == '.' {
            dot_index = i;
        }
        i = i + 1;
    }
    let mut name: String = string_new();
    i = start;
    while i < dot_index {
        string_push(&mut name, string_at(path, i));
        i = i + 1;
    }
    string_push(&mut name, '.');
    string_push_str(&mut name, ending);
    name
}

enum IOResult {
    OpenFailure,
    WriteFailure,
    ReadFailure,
    Success,
}

/// Check for errors and report if there is one.
fn ioResult_check_error(result: &IOResult, filename: &String) {
    match result {
        IOResult::OpenFailure => print_str("Could not open "),
        IOResult::WriteFailure => print_str("Could not write to "),
        IOResult::ReadFailure => print_str("Could not read "),
        _ => return,
    }
    print_string(filename);
    println();
    exit_process(1);
}

/// Print a string to stderr.
fn print_string(String::Inner(vec): &String) {
    match unsafe { io_write_stdout(vec_ptr::<u8>(vec), vec_len::<u8>(vec)) } {
        IOResult::WriteFailure => exit_process(12),
        _ => {},
    }
}

/// Print a string slice to stderr.
fn print_str(message: &str) {
    match unsafe { io_write_stdout(str::as_ptr(message) as *mut u8, str::len(message)) } {
        IOResult::WriteFailure => exit_process(12),
        _ => {},
    }
}

fn println() {
    print_str("\n");
}

/// Write the entire contents of a string into a file. Creates missing and truncates existing files.
fn write_file(mut filename: String, String::Inner(Vec::Vec(buf_ptr, len, _)): &String) {
    string_push(&mut filename, 0 as u8 as char); // NULL-terminate the string
    let String::Inner(Vec::Vec(path_ptr, _, _)): &String = &filename;
    let result: IOResult = unsafe { io_write(*path_ptr, *buf_ptr, *len) };
    ioResult_check_error(&result, &filename);
}

/// Read the entire contents of a file and return it as a String.
fn read_file(mut filename: String) -> String {
    string_push(&mut filename, 0 as u8 as char); // NULL-terminate the string
    let String::Inner(Vec::Vec(path_ptr, _, _)): &String = &filename;
    let O_RDONLY: usize = 0;

    unsafe {
        let fd: usize = open(*path_ptr, O_RDONLY, 0);
        if is_negative(fd) {
            ioResult_check_error(&IOResult::OpenFailure, &filename);
        }

        let mut string: Vec<u8> = vec_new::<u8>();
        let buffer_len: usize = 16384;
        let mut buffer: Vec<u8> = vec_with_len::<u8>(buffer_len);
        let buffer_ptr: *mut u8 = vec_ptr::<u8>(&buffer);
        while true {
            let bytes_read: usize = read(fd, buffer_ptr, buffer_len);
            if is_negative(bytes_read) {
                ioResult_check_error(&IOResult::ReadFailure, &filename);
            }
            if bytes_read == 0 {
                free(buffer_ptr);
                return String::Inner(string);
            }
            vec_set_len::<u8>(&mut buffer, bytes_read);
            vec_extend::<u8>(&mut string, &buffer);
            vec_set_len::<u8>(&mut buffer, buffer_len);
        }
    }
    unreachable()
}

/// Write the given `buffer` to `path` and return an IOResult. Creates the file if it does not
/// exist, otherwise truncates the existing file.
/// The caller must ensure that `path` is a NULL-terminated string and memory from `buffer[0]` to
/// `buffer[len - 1]` can be read safely.
unsafe fn io_write(path: *mut u8, buffer: *mut u8, len: usize) -> IOResult {
    let O_WRONLY_CREAT_TRUNC: usize = 321; // O_WRONLY = 1, O_CREAT = 64, O_TRUNC = 256
    let mode: usize = 420; // = 0o0644
    unsafe {
        let fd: usize = open(path, O_WRONLY_CREAT_TRUNC, mode);
        if is_negative(fd) {
            return IOResult::OpenFailure;
        }
        let mut offset: usize = 0;
        while offset < len {
            let remaining: usize = len - offset;
            let written: usize = write(fd, ptr_add::<u8>(buffer, offset), remaining);
            if or(is_negative(written), written == 0) {
                return IOResult::WriteFailure;
            }
            offset = offset + written;
        }
        IOResult::Success
    }
}

/// Write the given `buffer` to stdout and return an IOResult.
/// The caller must ensure that memory from `buffer[0]` to `buffer[len - 1]` can be read safely.
unsafe fn io_write_stdout(buffer: *mut u8, len: usize) -> IOResult {
    let stdout_fd: usize = 1;
    let mut offset: usize = 0;
    while offset < len {
        let remaining: usize = len - offset;
        let written: usize = unsafe { write(stdout_fd, ptr_add::<u8>(buffer, offset), remaining) };
        if or(is_negative(written), written == 0) {
            return IOResult::WriteFailure;
        }
        offset = offset + written;
    }
    IOResult::Success
}

// ------------------------- Memory -------------------------------

/// Copy `n` bytes from `src` to `dest`.
/// The caller must ensure that memory ranging from `dest[0]` to `dest[n - 1]`
/// can be written safely and from `src[0]` to `src[n - 1]` can be read safely.
unsafe fn memcopy<T>(dest: *mut T, src: *mut T, n: usize) {
    let byte_count: usize = n * size_of::<T>();
    let mut i: usize = 0;
    while i < byte_count {
        unsafe { *ptr_add::<u8>(dest as *mut u8, i) = *ptr_add::<u8>(src as *mut u8, i) };
        i = i + 1;
    }
}

/// Increment a pointer by n elements.
fn ptr_add<T>(ptr: *mut T, n: usize) -> *mut T {
    (ptr as usize + n * size_of::<T>()) as *mut T
}

/// Heap-allocate memory for `count` T and return a pointer to the beginning of the memory block.
/// The returned pointer is never null, but the memory is not zeroed, i.e. the caller must ensure
/// that uninitialised memory is never read.
/// The caller should cast the returned pointer to the desired type.
unsafe fn alloc<T>(count: usize) -> *mut T {
    unsafe {
        let p: *mut u8 = malloc(size_of::<T>() * count);
        if p as usize == 0 {
            print_str("Memory Allocation Error!\n");
            exit(1);
        }
        p as *mut T
    }
}

/// Exit the current process.
fn exit_process(code: usize) -> ! {
    unsafe { exit(code) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
    fn open(path: *mut u8, flags: usize, mode: usize) -> usize;
    fn write(fd: usize, buf: *mut u8, count: usize) -> usize;
    fn read(fd: usize, buf: *mut u8, count: usize) -> usize;
}
