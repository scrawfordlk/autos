// ------------------------- Lexer Helpers ----------------------------

fn make_lexer(input: &str) -> Lexer {
    let mut content = string_new();
    string_push_str(&mut content, input);
    let source = SourceFile::SourceFile(content, 0, 1, 0);
    Lexer::Lexer(source, Token::Eof)
}

fn collect_tokens(lexer: &mut Lexer) -> std::vec::Vec<Token> {
    let mut tokens: std::vec::Vec<Token> = std::vec::Vec::<Token>::new();
    loop {
        let tok = lexer_next_token(lexer);
        let is_eof = matches!(tok, Token::Eof);
        tokens.push(tok);
        if is_eof {
            break;
        }
    }
    tokens
}

fn ident(s: &str) -> Token {
    let mut string = string_new();
    string_push_str(&mut string, s);
    Token::Identifier(string)
}

fn str_lit(s: &str) -> Token {
    let mut string = string_new();
    string_push_str(&mut string, s);
    Token::Literal(Literal::String(string))
}

fn int_lit(value: usize) -> Token {
    Token::Literal(Literal::Int(value))
}

fn bool_lit(value: bool) -> Token {
    Token::Literal(Literal::Bool(value))
}

fn char_lit(value: char) -> Token {
    Token::Literal(Literal::Char(value))
}

fn cmp_token(comparison: Comparison) -> Token {
    Token::Cmp(comparison)
}

fn tokens_match(a: &Token, b: &Token) -> bool {
    token_eq(a, b)
}

fn assert_tokens(actual: std::vec::Vec<Token>, expected: std::vec::Vec<Token>) {
    assert_eq!(
        actual.len(),
        expected.len(),
        "token count mismatch: got {}, expected {}",
        actual.len(),
        expected.len()
    );
    for (i, (a, e)) in actual.iter().zip(expected.iter()).enumerate() {
        assert!(tokens_match(a, e), "token {} mismatch", i);
    }
}

// ------------------------- Char Classification ----------------------------

#[test]
fn test_is_whitespace() {
    assert!(is_whitespace(' '));
    assert!(is_whitespace('\t'));
    assert!(is_whitespace('\n'));
    assert!(is_whitespace('\r'));
    assert!(!is_whitespace('a'));
    assert!(!is_whitespace('0'));
}

#[test]
fn test_is_digit() {
    for c in '0'..='9' {
        assert!(is_digit(c));
    }
    assert!(!is_digit('a'));
    assert!(!is_digit(' '));
}

#[test]
fn test_is_alpha() {
    for c in 'a'..='z' {
        assert!(is_alpha(c));
    }
    for c in 'A'..='Z' {
        assert!(is_alpha(c));
    }
    assert!(is_alpha('_'));
    assert!(!is_alpha('0'));
    assert!(!is_alpha(' '));
}

#[test]
fn test_is_alphanumeric() {
    assert!(is_alphanumeric('a'));
    assert!(is_alphanumeric('Z'));
    assert!(is_alphanumeric('_'));
    assert!(is_alphanumeric('0'));
    assert!(is_alphanumeric('9'));
    assert!(!is_alphanumeric(' '));
    assert!(!is_alphanumeric('+'));
}

// ------------------------- Lexer Internals ----------------------------

#[test]
fn test_lexer_peek() {
    let lexer = make_lexer("a");
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('a')));
}

#[test]
fn test_lexer_peek_empty() {
    let lexer = make_lexer("");
    assert!(matches!(lexer_peek_char(&lexer), Option::None));
}

#[test]
fn test_lexer_consume() {
    let mut lexer = make_lexer("ab");
    assert!(matches!(lexer_consume_char(&mut lexer), Option::Some('a')));
    assert!(matches!(lexer_consume_char(&mut lexer), Option::Some('b')));
    assert!(matches!(lexer_consume_char(&mut lexer), Option::None));
}

#[test]
fn test_lexer_eof_detection() {
    let mut lexer = make_lexer("a");
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('a')));
    lexer_consume_char(&mut lexer);
    assert!(matches!(lexer_peek_char(&lexer), Option::None));
}

#[test]
fn test_lexer_sourcefile() {
    let lexer = make_lexer("abc");
    let SourceFile::SourceFile(_, index, _, _) = lexer_sourcefile(&lexer);
    assert_eq!(*index, 0);
}

#[test]
fn test_lexer_expect_char_success() {
    let mut lexer = make_lexer("xyz");
    lexer_expect_char(&mut lexer, 'x');
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('y')));
}

#[test]
fn test_scan_identifier_direct() {
    let mut lexer = make_lexer("hello_42!");
    let ident = lexer_scan_identifier(&mut lexer);
    assert!(string_eq(&ident, &string_from_str("hello_42")));
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('!')));
}

#[test]
fn test_identifier_to_token_direct_keyword() {
    let tok = identifier_to_token(string_from_str("usize"));
    assert!(matches!(tok, Token::Usize));
}

#[test]
fn test_identifier_to_token_direct_identifier() {
    let tok = identifier_to_token(string_from_str("my_var"));
    match tok {
        Token::Identifier(s) => assert!(string_eq(&s, &string_from_str("my_var"))),
        _ => assert!(false, "expected identifier token"),
    }
}

#[test]
fn test_scan_integer_direct() {
    let mut lexer = make_lexer("123abc");
    let value = lexer_scan_integer(&mut lexer);
    assert_eq!(value, 123);
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('a')));
}

#[test]
fn test_scan_char_literal_direct() {
    let mut lexer = make_lexer("'x'");
    assert_eq!(lexer_scan_char_literal(&mut lexer), 'x');
    assert!(matches!(lexer_peek_char(&lexer), Option::None));
}

#[test]
fn test_scan_string_literal_direct() {
    let mut lexer = make_lexer("\"ab\\n\"");
    let s = lexer_scan_string_literal(&mut lexer);
    assert!(string_eq(&s, &string_from_str("ab\n")));
    assert!(matches!(lexer_peek_char(&lexer), Option::None));
}

#[test]
fn test_scan_escape_char_direct() {
    let mut lexer = make_lexer("n");
    assert_eq!(lexer_scan_escape_char(&mut lexer), '\n');
}

#[test]
fn test_scan_symbol_direct() {
    let mut lexer = make_lexer("+");
    let tok = lexer_scan_symbol(&mut lexer);
    assert!(matches!(tok, Token::Plus));
}

#[test]
fn test_scan_slash_direct() {
    let mut lexer = make_lexer("x");
    assert!(matches!(lexer_scan_slash(&mut lexer), Token::Slash));
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('x')));
}

#[test]
fn test_scan_colon_direct() {
    let mut lexer = make_lexer("x");
    assert!(matches!(lexer_scan_colon(&mut lexer), Token::Colon));
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('x')));
}

#[test]
fn test_scan_equals_direct() {
    let mut lexer = make_lexer(">");
    assert!(matches!(lexer_scan_equals(&mut lexer), Token::ArmArrow));
}

#[test]
fn test_scan_minus_direct() {
    let mut lexer = make_lexer(">");
    assert!(matches!(lexer_scan_minus(&mut lexer), Token::TypeArrow));
}

#[test]
fn test_scan_bang_direct() {
    let mut lexer = make_lexer("=");
    assert!(matches!(
        lexer_scan_bang(&mut lexer),
        Token::Cmp(Comparison::Ne)
    ));
}

#[test]
fn test_scan_less_direct() {
    let mut lexer = make_lexer("=");
    assert!(matches!(
        lexer_scan_less(&mut lexer),
        Token::Cmp(Comparison::Leq)
    ));
}

#[test]
fn test_scan_greater_direct() {
    let mut lexer = make_lexer("=");
    assert!(matches!(
        lexer_scan_greater(&mut lexer),
        Token::Cmp(Comparison::Geq)
    ));
}

#[test]
fn test_skip_whitespace_direct() {
    let mut lexer = make_lexer("  \n\tabc");
    lexer_skip_whitespace(&mut lexer);
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('a')));
}

#[test]
fn test_skip_line_comment_direct() {
    let mut lexer = make_lexer("comment text\nz");
    lexer_skip_line_comment(&mut lexer);
    assert!(matches!(lexer_peek_char(&lexer), Option::Some('z')));
}

// ------------------------- Keywords ----------------------------

#[test]
fn test_keyword_fn() {
    let mut lexer = make_lexer("fn");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Fn, Token::Eof]);
}

#[test]
fn test_keyword_enum() {
    let mut lexer = make_lexer("enum");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Enum, Token::Eof]);
}

#[test]
fn test_keyword_let() {
    let mut lexer = make_lexer("let");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Let, Token::Eof]);
}

#[test]
fn test_keyword_if() {
    let mut lexer = make_lexer("if");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::If, Token::Eof]);
}

#[test]
fn test_keyword_else() {
    let mut lexer = make_lexer("else");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Else, Token::Eof]);
}

#[test]
fn test_keyword_while() {
    let mut lexer = make_lexer("while");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::While, Token::Eof]);
}

#[test]
fn test_keyword_return() {
    let mut lexer = make_lexer("return");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Return, Token::Eof]);
}

#[test]
fn test_keyword_match() {
    let mut lexer = make_lexer("match");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Match, Token::Eof]);
}

#[test]
fn test_keyword_as() {
    let mut lexer = make_lexer("as");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::As, Token::Eof]);
}

#[test]
fn test_keyword_mut() {
    let mut lexer = make_lexer("mut");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Mut, Token::Eof]);
}

#[test]
fn test_keyword_unsafe() {
    let mut lexer = make_lexer("unsafe");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Unsafe, Token::Eof]);
}

// ------------------------- Types ----------------------------

#[test]
fn test_type_usize() {
    let mut lexer = make_lexer("usize");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Usize, Token::Eof]);
}

#[test]
fn test_type_u8() {
    let mut lexer = make_lexer("u8");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::U8, Token::Eof]);
}

#[test]
fn test_type_char() {
    let mut lexer = make_lexer("char");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Char, Token::Eof]);
}

#[test]
fn test_type_str() {
    let mut lexer = make_lexer("str");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Str, Token::Eof]);
}

#[test]
fn test_type_bool() {
    let mut lexer = make_lexer("bool");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Bool, Token::Eof]);
}

// ------------------------- Identifiers ----------------------------

#[test]
fn test_identifier_simple() {
    let mut lexer = make_lexer("foo");
    assert_tokens(collect_tokens(&mut lexer), vec![ident("foo"), Token::Eof]);
}

#[test]
fn test_identifier_with_underscore() {
    let mut lexer = make_lexer("foo_bar");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![ident("foo_bar"), Token::Eof],
    );
}

#[test]
fn test_identifier_with_numbers() {
    let mut lexer = make_lexer("foo123");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![ident("foo123"), Token::Eof],
    );
}

#[test]
fn test_identifier_starting_with_underscore() {
    let mut lexer = make_lexer("_private");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![ident("_private"), Token::Eof],
    );
}

// ------------------------- Integer Literals ----------------------------

#[test]
fn test_integer_zero() {
    let mut lexer = make_lexer("0");
    assert_tokens(collect_tokens(&mut lexer), vec![int_lit(0), Token::Eof]);
}

#[test]
fn test_integer_single_digit() {
    let mut lexer = make_lexer("7");
    assert_tokens(collect_tokens(&mut lexer), vec![int_lit(7), Token::Eof]);
}

#[test]
fn test_integer_multi_digit() {
    let mut lexer = make_lexer("12345");
    assert_tokens(collect_tokens(&mut lexer), vec![int_lit(12345), Token::Eof]);
}

// ------------------------- Boolean Literals ----------------------------

#[test]
fn test_boolean_true() {
    let mut lexer = make_lexer("true");
    assert_tokens(collect_tokens(&mut lexer), vec![bool_lit(true), Token::Eof]);
}

#[test]
fn test_boolean_false() {
    let mut lexer = make_lexer("false");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![bool_lit(false), Token::Eof],
    );
}

// ------------------------- Character Literals ----------------------------

#[test]
fn test_char_literal_simple() {
    let mut lexer = make_lexer("'a'");
    assert_tokens(collect_tokens(&mut lexer), vec![char_lit('a'), Token::Eof]);
}

#[test]
fn test_char_literal_escape_n() {
    let mut lexer = make_lexer("'\\n'");
    assert_tokens(collect_tokens(&mut lexer), vec![char_lit('\n'), Token::Eof]);
}

#[test]
fn test_char_literal_escape_t() {
    let mut lexer = make_lexer("'\\t'");
    assert_tokens(collect_tokens(&mut lexer), vec![char_lit('\t'), Token::Eof]);
}

#[test]
fn test_char_literal_escape_r() {
    let mut lexer = make_lexer("'\\r'");
    assert_tokens(collect_tokens(&mut lexer), vec![char_lit('\r'), Token::Eof]);
}

#[test]
fn test_char_literal_escape_backslash() {
    let mut lexer = make_lexer("'\\\\'");
    assert_tokens(collect_tokens(&mut lexer), vec![char_lit('\\'), Token::Eof]);
}

#[test]
fn test_char_literal_escape_quote() {
    let mut lexer = make_lexer("'\\''");
    assert_tokens(collect_tokens(&mut lexer), vec![char_lit('\''), Token::Eof]);
}

#[test]
fn test_char_literal_escape_null() {
    let mut lexer = make_lexer("'\\0'");
    assert_tokens(collect_tokens(&mut lexer), vec![char_lit('\0'), Token::Eof]);
}

// ------------------------- String Literals ----------------------------

#[test]
fn test_string_literal_empty() {
    let mut lexer = make_lexer("\"\"");
    assert_tokens(collect_tokens(&mut lexer), vec![str_lit(""), Token::Eof]);
}

#[test]
fn test_string_literal_simple() {
    let mut lexer = make_lexer("\"hello\"");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![str_lit("hello"), Token::Eof],
    );
}

#[test]
fn test_string_literal_with_escapes() {
    let mut lexer = make_lexer("\"a\\nb\\tc\"");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![str_lit("a\nb\tc"), Token::Eof],
    );
}

#[test]
fn test_string_literal_escaped_quote() {
    let mut lexer = make_lexer("\"say \\\"hi\\\"\"");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![str_lit("say \"hi\""), Token::Eof],
    );
}

// ------------------------- Symbol ----------------------------

#[test]
fn test_symbol_braces() {
    let mut lexer = make_lexer("{}");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::LBrace, Token::RBrace, Token::Eof],
    );
}

#[test]
fn test_symbol_parens() {
    let mut lexer = make_lexer("()");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::LParen, Token::RParen, Token::Eof],
    );
}

#[test]
fn test_symbol_colon() {
    let mut lexer = make_lexer(":");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Colon, Token::Eof]);
}

#[test]
fn test_symbol_double_colon() {
    let mut lexer = make_lexer("::");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::DoubleColon, Token::Eof],
    );
}

#[test]
fn test_symbol_semicolon() {
    let mut lexer = make_lexer(";");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::SemiColon, Token::Eof],
    );
}

#[test]
fn test_symbol_comma() {
    let mut lexer = make_lexer(",");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Comma, Token::Eof]);
}

#[test]
fn test_symbol_assign() {
    let mut lexer = make_lexer("=");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Assign, Token::Eof]);
}

#[test]
fn test_symbol_arm_arrow() {
    let mut lexer = make_lexer("=>");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::ArmArrow, Token::Eof],
    );
}

#[test]
fn test_symbol_type_arrow() {
    let mut lexer = make_lexer("->");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::TypeArrow, Token::Eof],
    );
}

#[test]
fn test_symbol_ampersand() {
    let mut lexer = make_lexer("&");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::Ampersand, Token::Eof],
    );
}

// ------------------------- Operators ----------------------------

#[test]
fn test_operator_plus() {
    let mut lexer = make_lexer("+");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Plus, Token::Eof]);
}

#[test]
fn test_operator_minus() {
    let mut lexer = make_lexer("-");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Minus, Token::Eof]);
}

#[test]
fn test_operator_star() {
    let mut lexer = make_lexer("*");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Star, Token::Eof]);
}

#[test]
fn test_operator_slash() {
    let mut lexer = make_lexer("/");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Slash, Token::Eof]);
}

#[test]
fn test_operator_remainder() {
    let mut lexer = make_lexer("%");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::Remainder, Token::Eof],
    );
}

// ------------------------- Comparisons ----------------------------

#[test]
fn test_comparison_eq() {
    let mut lexer = make_lexer("==");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(Comparison::Eq), Token::Eof],
    );
}

#[test]
fn test_comparison_neq() {
    let mut lexer = make_lexer("!=");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(Comparison::Ne), Token::Eof],
    );
}

#[test]
fn test_comparison_gt() {
    let mut lexer = make_lexer(">");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(Comparison::Gt), Token::Eof],
    );
}

#[test]
fn test_comparison_lt() {
    let mut lexer = make_lexer("<");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(Comparison::Lt), Token::Eof],
    );
}

#[test]
fn test_comparison_geq() {
    let mut lexer = make_lexer(">=");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(Comparison::Geq), Token::Eof],
    );
}

#[test]
fn test_comparison_leq() {
    let mut lexer = make_lexer("<=");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(Comparison::Leq), Token::Eof],
    );
}

// ------------------------- Whitespace and Comments ----------------------------

#[test]
fn test_skip_whitespace() {
    let mut lexer = make_lexer("   fn");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Fn, Token::Eof]);
}

#[test]
fn test_skip_tabs_and_newlines() {
    let mut lexer = make_lexer("\t\n\r  fn");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Fn, Token::Eof]);
}

#[test]
fn test_skip_line_comment() {
    let mut lexer = make_lexer("// comment\nfn");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Fn, Token::Eof]);
}

#[test]
fn test_skip_multiple_comments() {
    let mut lexer = make_lexer("// first\n// second\nfn");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Fn, Token::Eof]);
}

#[test]
fn test_comment_at_eof() {
    let mut lexer = make_lexer("// comment");
    assert_tokens(collect_tokens(&mut lexer), vec![Token::Eof]);
}

// ------------------------- Complex Sequences ----------------------------

#[test]
fn test_function_signature() {
    let mut lexer = make_lexer("fn foo(x: usize) -> u8");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            Token::Fn,
            ident("foo"),
            Token::LParen,
            ident("x"),
            Token::Colon,
            Token::Usize,
            Token::RParen,
            Token::TypeArrow,
            Token::U8,
            Token::Eof,
        ],
    );
}

#[test]
fn test_let_statement() {
    let mut lexer = make_lexer("let x: usize = 42;");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            Token::Let,
            ident("x"),
            Token::Colon,
            Token::Usize,
            Token::Assign,
            int_lit(42),
            Token::SemiColon,
            Token::Eof,
        ],
    );
}

#[test]
fn test_match_arm() {
    let mut lexer = make_lexer("match x { 1 => 2, }");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            Token::Match,
            ident("x"),
            Token::LBrace,
            int_lit(1),
            Token::ArmArrow,
            int_lit(2),
            Token::Comma,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_comparison_expression() {
    let mut lexer = make_lexer("a == b != c < d > e <= f >= g");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            ident("a"),
            cmp_token(Comparison::Eq),
            ident("b"),
            cmp_token(Comparison::Ne),
            ident("c"),
            cmp_token(Comparison::Lt),
            ident("d"),
            cmp_token(Comparison::Gt),
            ident("e"),
            cmp_token(Comparison::Leq),
            ident("f"),
            cmp_token(Comparison::Geq),
            ident("g"),
            Token::Eof,
        ],
    );
}

#[test]
fn test_enum_definition() {
    let mut lexer = make_lexer("enum Foo { A, B(usize) }");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            Token::Enum,
            ident("Foo"),
            Token::LBrace,
            ident("A"),
            Token::Comma,
            ident("B"),
            Token::LParen,
            Token::Usize,
            Token::RParen,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_path_with_double_colon() {
    let mut lexer = make_lexer("Foo::Bar");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![ident("Foo"), Token::DoubleColon, ident("Bar"), Token::Eof],
    );
}

#[test]
fn test_reference_and_mut() {
    let mut lexer = make_lexer("&mut x");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![Token::Ampersand, Token::Mut, ident("x"), Token::Eof],
    );
}

// ------------------------- Full Programs ----------------------------

#[test]
fn test_full_program_hello_world() {
    let program = r#"
fn main() {
    let msg: &str = "Hello, World!";
}
"#;
    let mut lexer = make_lexer(program);
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            Token::Fn,
            ident("main"),
            Token::LParen,
            Token::RParen,
            Token::LBrace,
            Token::Let,
            ident("msg"),
            Token::Colon,
            Token::Ampersand,
            Token::Str,
            Token::Assign,
            str_lit("Hello, World!"),
            Token::SemiColon,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_full_program_enum_and_match() {
    let program = r#"
enum Option {
    Some(usize),
    None,
}

fn unwrap(opt: Option) -> usize {
    match opt {
        Option::Some(x) => x,
        Option::None => 0,
    }
}
"#;
    let mut lexer = make_lexer(program);
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            // enum Option { Some(usize), None, }
            Token::Enum,
            ident("Option"),
            Token::LBrace,
            ident("Some"),
            Token::LParen,
            Token::Usize,
            Token::RParen,
            Token::Comma,
            ident("None"),
            Token::Comma,
            Token::RBrace,
            // fn unwrap(opt: Option) -> usize {
            Token::Fn,
            ident("unwrap"),
            Token::LParen,
            ident("opt"),
            Token::Colon,
            ident("Option"),
            Token::RParen,
            Token::TypeArrow,
            Token::Usize,
            Token::LBrace,
            // match opt {
            Token::Match,
            ident("opt"),
            Token::LBrace,
            // Option::Some(x) => x,
            ident("Option"),
            Token::DoubleColon,
            ident("Some"),
            Token::LParen,
            ident("x"),
            Token::RParen,
            Token::ArmArrow,
            ident("x"),
            Token::Comma,
            // Option::None => 0,
            ident("Option"),
            Token::DoubleColon,
            ident("None"),
            Token::ArmArrow,
            int_lit(0),
            Token::Comma,
            // closing braces
            Token::RBrace,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_full_program_while_loop() {
    let program = r#"
fn factorial(n: usize) -> usize {
    let mut result: usize = 1;
    let mut i: usize = 1;
    while i <= n {
        result = result * i;
        i = i + 1;
    }
    return result;
}
"#;
    let mut lexer = make_lexer(program);
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            // fn factorial(n: usize) -> usize {
            Token::Fn,
            ident("factorial"),
            Token::LParen,
            ident("n"),
            Token::Colon,
            Token::Usize,
            Token::RParen,
            Token::TypeArrow,
            Token::Usize,
            Token::LBrace,
            // let mut result: usize = 1;
            Token::Let,
            Token::Mut,
            ident("result"),
            Token::Colon,
            Token::Usize,
            Token::Assign,
            int_lit(1),
            Token::SemiColon,
            // let mut i: usize = 1;
            Token::Let,
            Token::Mut,
            ident("i"),
            Token::Colon,
            Token::Usize,
            Token::Assign,
            int_lit(1),
            Token::SemiColon,
            // while i <= n {
            Token::While,
            ident("i"),
            cmp_token(Comparison::Leq),
            ident("n"),
            Token::LBrace,
            // result = result * i;
            ident("result"),
            Token::Assign,
            ident("result"),
            Token::Star,
            ident("i"),
            Token::SemiColon,
            // i = i + 1;
            ident("i"),
            Token::Assign,
            ident("i"),
            Token::Plus,
            int_lit(1),
            Token::SemiColon,
            // }
            Token::RBrace,
            // return result;
            Token::Return,
            ident("result"),
            Token::SemiColon,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_full_program_if_else() {
    let program = r#"
fn max(a: usize, b: usize) -> usize {
    if a > b {
        return a;
    } else {
        return b;
    }
}
"#;
    let mut lexer = make_lexer(program);
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            // fn max(a: usize, b: usize) -> usize {
            Token::Fn,
            ident("max"),
            Token::LParen,
            ident("a"),
            Token::Colon,
            Token::Usize,
            Token::Comma,
            ident("b"),
            Token::Colon,
            Token::Usize,
            Token::RParen,
            Token::TypeArrow,
            Token::Usize,
            Token::LBrace,
            // if a > b {
            Token::If,
            ident("a"),
            cmp_token(Comparison::Gt),
            ident("b"),
            Token::LBrace,
            // return a;
            Token::Return,
            ident("a"),
            Token::SemiColon,
            Token::RBrace,
            // else {
            Token::Else,
            Token::LBrace,
            // return b;
            Token::Return,
            ident("b"),
            Token::SemiColon,
            Token::RBrace,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_full_program_pointer_arithmetic() {
    let program = r#"
fn write_byte(ptr: *mut u8, offset: usize, value: u8) {
    let target: *mut u8 = ptr as usize + offset as *mut u8;
    *target = value;
}
"#;
    let mut lexer = make_lexer(program);
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            // fn write_byte(ptr: *mut u8, offset: usize, value: u8) {
            Token::Fn,
            ident("write_byte"),
            Token::LParen,
            ident("ptr"),
            Token::Colon,
            Token::Star,
            Token::Mut,
            Token::U8,
            Token::Comma,
            ident("offset"),
            Token::Colon,
            Token::Usize,
            Token::Comma,
            ident("value"),
            Token::Colon,
            Token::U8,
            Token::RParen,
            Token::LBrace,
            // let target: *mut u8 = ptr as usize + offset as *mut u8;
            Token::Let,
            ident("target"),
            Token::Colon,
            Token::Star,
            Token::Mut,
            Token::U8,
            Token::Assign,
            ident("ptr"),
            Token::As,
            Token::Usize,
            Token::Plus,
            ident("offset"),
            Token::As,
            Token::Star,
            Token::Mut,
            Token::U8,
            Token::SemiColon,
            // *target = value;
            Token::Star,
            ident("target"),
            Token::Assign,
            ident("value"),
            Token::SemiColon,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_full_program_with_comments() {
    let program = r#"
// This is a comment
fn add(a: usize, b: usize) -> usize {
    // Add two numbers
    return a + b; // return sum
}
"#;
    let mut lexer = make_lexer(program);
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            Token::Fn,
            ident("add"),
            Token::LParen,
            ident("a"),
            Token::Colon,
            Token::Usize,
            Token::Comma,
            ident("b"),
            Token::Colon,
            Token::Usize,
            Token::RParen,
            Token::TypeArrow,
            Token::Usize,
            Token::LBrace,
            Token::Return,
            ident("a"),
            Token::Plus,
            ident("b"),
            Token::SemiColon,
            Token::RBrace,
            Token::Eof,
        ],
    );
}

#[test]
fn test_full_program_unsafe_and_bool() {
    let program = r#"
unsafe fn flag(x: bool) -> bool {
    return true;
}
"#;
    let mut lexer = make_lexer(program);
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            Token::Unsafe,
            Token::Fn,
            ident("flag"),
            Token::LParen,
            ident("x"),
            Token::Colon,
            Token::Bool,
            Token::RParen,
            Token::TypeArrow,
            Token::Bool,
            Token::LBrace,
            Token::Return,
            bool_lit(true),
            Token::SemiColon,
            Token::RBrace,
            Token::Eof,
        ],
    );
}
