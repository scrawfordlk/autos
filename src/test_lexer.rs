fn make_lexer(input: &str) -> RLexer {
    let mut content = string_new();
    string_push_str(&mut content, input);
    let source = SourceFile::SourceFile(content, 0, 1, 0);
    RLexer::Lexer(source, RToken::Eof)
}

fn collect_tokens(lexer: &mut RLexer) -> std::vec::Vec<RToken> {
    let mut tokens: std::vec::Vec<RToken> = std::vec::Vec::<RToken>::new();
    loop {
        let tok = rLexer_next_token(lexer);
        let is_eof = matches!(tok, RToken::Eof);
        tokens.push(tok);
        if is_eof {
            break;
        }
    }
    tokens
}

fn ident(s: &str) -> RToken {
    let mut string = string_new();
    string_push_str(&mut string, s);
    RToken::Identifier(string)
}

fn str_lit(s: &str) -> RToken {
    let mut string = string_new();
    string_push_str(&mut string, s);
    RToken::Literal(RLiteral::String(string))
}

fn int_lit(value: usize) -> RToken {
    RToken::Literal(RLiteral::Int(value))
}

fn bool_lit(value: bool) -> RToken {
    RToken::Literal(RLiteral::Bool(value))
}

fn char_lit(value: char) -> RToken {
    RToken::Literal(RLiteral::Char(value))
}

fn cmp_token(comparison: RComparisonOp) -> RToken {
    RToken::Cmp(comparison)
}

fn tokens_match(a: &RToken, b: &RToken) -> bool {
    token_eq(a, b)
}

fn assert_tokens(actual: std::vec::Vec<RToken>, expected: std::vec::Vec<RToken>) {
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

#[test]
fn test_rLexer_peek() {
    let lexer = make_lexer("a");
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('a')));
}

#[test]
fn test_rLexer_peek_empty() {
    let lexer = make_lexer("");
    assert!(matches!(rLexer_peek_char(&lexer), Option::None));
}

#[test]
fn test_rLexer_consume() {
    let mut lexer = make_lexer("ab");
    assert!(matches!(rLexer_consume_char(&mut lexer), Option::Some('a')));
    assert!(matches!(rLexer_consume_char(&mut lexer), Option::Some('b')));
    assert!(matches!(rLexer_consume_char(&mut lexer), Option::None));
}

#[test]
fn test_rLexer_eof_detection() {
    let mut lexer = make_lexer("a");
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('a')));
    rLexer_consume_char(&mut lexer);
    assert!(matches!(rLexer_peek_char(&lexer), Option::None));
}

#[test]
fn test_rLexer_sourcefile() {
    let lexer = make_lexer("abc");
    let SourceFile::SourceFile(_, index, _, _) = rLexer_sourcefile(&lexer);
    assert_eq!(*index, 0);
}

#[test]
fn test_rLexer_expect_char_success() {
    let mut lexer = make_lexer("xyz");
    rLexer_expect_char(&mut lexer, 'x');
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('y')));
}

#[test]
fn test_scan_identifier_direct() {
    let mut lexer = make_lexer("hello_42!");
    let ident = rLexer_scan_identifier(&mut lexer);
    assert!(string_eq(&ident, &string("hello_42")));
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('!')));
}

#[test]
fn test_identifier_to_token_direct_keyword() {
    let tok = rust_identifier_to_token(string("usize"));
    assert!(matches!(tok, RToken::Usize));
}

#[test]
fn test_identifier_to_token_direct_identifier() {
    let tok = rust_identifier_to_token(string("my_var"));
    match tok {
        RToken::Identifier(s) => assert!(string_eq(&s, &string("my_var"))),
        _ => assert!(false, "expected identifier token"),
    }
}

#[test]
fn test_scan_integer_direct() {
    let mut lexer = make_lexer("123abc");
    let value = rLexer_scan_integer(&mut lexer);
    assert_eq!(value, 123);
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('a')));
}

#[test]
fn test_scan_char_literal_direct() {
    let mut lexer = make_lexer("'x'");
    assert_eq!(rLexer_scan_char_literal(&mut lexer), 'x');
    assert!(matches!(rLexer_peek_char(&lexer), Option::None));
}

#[test]
fn test_scan_string_literal_direct() {
    let mut lexer = make_lexer("\"ab\\n\"");
    let s = rLexer_scan_string_literal(&mut lexer);
    assert!(string_eq(&s, &string("ab\n")));
    assert!(matches!(rLexer_peek_char(&lexer), Option::None));
}

#[test]
fn test_scan_escape_char_direct() {
    let mut lexer = make_lexer("n");
    assert_eq!(rLexer_scan_escape_char(&mut lexer), '\n');
}

#[test]
fn test_scan_symbol_direct() {
    let mut lexer = make_lexer("+");
    let tok = rLexer_scan_symbol(&mut lexer);
    assert!(matches!(tok, RToken::Plus));
}

#[test]
fn test_scan_slash_direct() {
    let mut lexer = make_lexer("x");
    assert!(matches!(rLexer_scan_slash(&mut lexer), RToken::Slash));
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('x')));
}

#[test]
fn test_scan_colon_direct() {
    let mut lexer = make_lexer("x");
    assert!(matches!(rLexer_scan_colon(&mut lexer), RToken::Colon));
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('x')));
}

#[test]
fn test_scan_equals_direct() {
    let mut lexer = make_lexer(">");
    assert!(matches!(rLexer_scan_equals(&mut lexer), RToken::FatArrow));
}

#[test]
fn test_scan_minus_direct() {
    let mut lexer = make_lexer(">");
    assert!(matches!(rLexer_scan_minus(&mut lexer), RToken::Arrow));
}

#[test]
fn test_scan_bang_direct() {
    let mut lexer = make_lexer("=");
    assert!(matches!(
        rLexer_scan_bang(&mut lexer),
        RToken::Cmp(RComparisonOp::Ne)
    ));
}

#[test]
fn test_scan_less_direct() {
    let mut lexer = make_lexer("=");
    assert!(matches!(
        rLexer_scan_less(&mut lexer),
        RToken::Cmp(RComparisonOp::Leq)
    ));
}

#[test]
fn test_scan_greater_direct() {
    let mut lexer = make_lexer("=");
    assert!(matches!(
        rLexer_scan_greater(&mut lexer),
        RToken::Cmp(RComparisonOp::Geq)
    ));
}

#[test]
fn test_skip_whitespace_direct() {
    let mut lexer = make_lexer("  \n\tabc");
    rLexer_skip_whitespace(&mut lexer);
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('a')));
}

#[test]
fn test_skip_line_comment_direct() {
    let mut lexer = make_lexer("comment text\nz");
    rLexer_skip_line_comment(&mut lexer);
    assert!(matches!(rLexer_peek_char(&lexer), Option::Some('z')));
}

#[test]
fn test_keyword_fn() {
    let mut lexer = make_lexer("fn");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Fn, RToken::Eof]);
}

#[test]
fn test_keyword_enum() {
    let mut lexer = make_lexer("enum");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Enum, RToken::Eof]);
}

#[test]
fn test_keyword_let() {
    let mut lexer = make_lexer("let");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Let, RToken::Eof]);
}

#[test]
fn test_keyword_if() {
    let mut lexer = make_lexer("if");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::If, RToken::Eof]);
}

#[test]
fn test_keyword_else() {
    let mut lexer = make_lexer("else");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Else, RToken::Eof]);
}

#[test]
fn test_keyword_while() {
    let mut lexer = make_lexer("while");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::While, RToken::Eof]);
}

#[test]
fn test_keyword_return() {
    let mut lexer = make_lexer("return");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::Return, RToken::Eof],
    );
}

#[test]
fn test_keyword_match() {
    let mut lexer = make_lexer("match");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Match, RToken::Eof]);
}

#[test]
fn test_keyword_as() {
    let mut lexer = make_lexer("as");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::As, RToken::Eof]);
}

#[test]
fn test_keyword_mut() {
    let mut lexer = make_lexer("mut");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Mut, RToken::Eof]);
}

#[test]
fn test_keyword_unsafe() {
    let mut lexer = make_lexer("unsafe");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::Unsafe, RToken::Eof],
    );
}

#[test]
fn test_type_usize() {
    let mut lexer = make_lexer("usize");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Usize, RToken::Eof]);
}

#[test]
fn test_type_u8() {
    let mut lexer = make_lexer("u8");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::U8, RToken::Eof]);
}

#[test]
fn test_type_char() {
    let mut lexer = make_lexer("char");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Char, RToken::Eof]);
}

#[test]
fn test_type_str() {
    let mut lexer = make_lexer("str");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Str, RToken::Eof]);
}

#[test]
fn test_type_bool() {
    let mut lexer = make_lexer("bool");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Bool, RToken::Eof]);
}

#[test]
fn test_identifier_simple() {
    let mut lexer = make_lexer("foo");
    assert_tokens(collect_tokens(&mut lexer), vec![ident("foo"), RToken::Eof]);
}

#[test]
fn test_identifier_with_underscore() {
    let mut lexer = make_lexer("foo_bar");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![ident("foo_bar"), RToken::Eof],
    );
}

#[test]
fn test_identifier_with_numbers() {
    let mut lexer = make_lexer("foo123");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![ident("foo123"), RToken::Eof],
    );
}

#[test]
fn test_identifier_starting_with_underscore() {
    let mut lexer = make_lexer("_private");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![ident("_private"), RToken::Eof],
    );
}

#[test]
fn test_integer_zero() {
    let mut lexer = make_lexer("0");
    assert_tokens(collect_tokens(&mut lexer), vec![int_lit(0), RToken::Eof]);
}

#[test]
fn test_integer_single_digit() {
    let mut lexer = make_lexer("7");
    assert_tokens(collect_tokens(&mut lexer), vec![int_lit(7), RToken::Eof]);
}

#[test]
fn test_integer_multi_digit() {
    let mut lexer = make_lexer("12345");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![int_lit(12345), RToken::Eof],
    );
}

#[test]
fn test_boolean_true() {
    let mut lexer = make_lexer("true");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![bool_lit(true), RToken::Eof],
    );
}

#[test]
fn test_boolean_false() {
    let mut lexer = make_lexer("false");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![bool_lit(false), RToken::Eof],
    );
}

#[test]
fn test_char_literal_simple() {
    let mut lexer = make_lexer("'a'");
    assert_tokens(collect_tokens(&mut lexer), vec![char_lit('a'), RToken::Eof]);
}

#[test]
fn test_char_literal_escape_n() {
    let mut lexer = make_lexer("'\\n'");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![char_lit('\n'), RToken::Eof],
    );
}

#[test]
fn test_char_literal_escape_t() {
    let mut lexer = make_lexer("'\\t'");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![char_lit('\t'), RToken::Eof],
    );
}

#[test]
fn test_char_literal_escape_r() {
    let mut lexer = make_lexer("'\\r'");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![char_lit('\r'), RToken::Eof],
    );
}

#[test]
fn test_char_literal_escape_backslash() {
    let mut lexer = make_lexer("'\\\\'");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![char_lit('\\'), RToken::Eof],
    );
}

#[test]
fn test_char_literal_escape_quote() {
    let mut lexer = make_lexer("'\\''");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![char_lit('\''), RToken::Eof],
    );
}

#[test]
fn test_char_literal_escape_null() {
    let mut lexer = make_lexer("'\\0'");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![char_lit('\0'), RToken::Eof],
    );
}

#[test]
fn test_string_literal_empty() {
    let mut lexer = make_lexer("\"\"");
    assert_tokens(collect_tokens(&mut lexer), vec![str_lit(""), RToken::Eof]);
}

#[test]
fn test_string_literal_simple() {
    let mut lexer = make_lexer("\"hello\"");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![str_lit("hello"), RToken::Eof],
    );
}

#[test]
fn test_string_literal_with_escapes() {
    let mut lexer = make_lexer("\"a\\nb\\tc\"");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![str_lit("a\nb\tc"), RToken::Eof],
    );
}

#[test]
fn test_string_literal_escaped_quote() {
    let mut lexer = make_lexer("\"say \\\"hi\\\"\"");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![str_lit("say \"hi\""), RToken::Eof],
    );
}

#[test]
fn test_symbol_braces() {
    let mut lexer = make_lexer("{}");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::LBrace, RToken::RBrace, RToken::Eof],
    );
}

#[test]
fn test_symbol_parens() {
    let mut lexer = make_lexer("()");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::LParen, RToken::RParen, RToken::Eof],
    );
}

#[test]
fn test_symbol_colon() {
    let mut lexer = make_lexer(":");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Colon, RToken::Eof]);
}

#[test]
fn test_symbol_double_colon() {
    let mut lexer = make_lexer("::");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::DoubleColon, RToken::Eof],
    );
}

#[test]
fn test_symbol_semicolon() {
    let mut lexer = make_lexer(";");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::SemiColon, RToken::Eof],
    );
}

#[test]
fn test_symbol_comma() {
    let mut lexer = make_lexer(",");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Comma, RToken::Eof]);
}

#[test]
fn test_symbol_assign() {
    let mut lexer = make_lexer("=");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::Assign, RToken::Eof],
    );
}

#[test]
fn test_symbol_arm_arrow() {
    let mut lexer = make_lexer("=>");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::FatArrow, RToken::Eof],
    );
}

#[test]
fn test_symbol_type_arrow() {
    let mut lexer = make_lexer("->");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Arrow, RToken::Eof]);
}

#[test]
fn test_symbol_ampersand() {
    let mut lexer = make_lexer("&");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::Ampersand, RToken::Eof],
    );
}

#[test]
fn test_operator_plus() {
    let mut lexer = make_lexer("+");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Plus, RToken::Eof]);
}

#[test]
fn test_operator_minus() {
    let mut lexer = make_lexer("-");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Minus, RToken::Eof]);
}

#[test]
fn test_operator_star() {
    let mut lexer = make_lexer("*");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Star, RToken::Eof]);
}

#[test]
fn test_operator_slash() {
    let mut lexer = make_lexer("/");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Slash, RToken::Eof]);
}

#[test]
fn test_operator_remainder() {
    let mut lexer = make_lexer("%");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::Remainder, RToken::Eof],
    );
}

#[test]
fn test_comparison_eq() {
    let mut lexer = make_lexer("==");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(RComparisonOp::Eq), RToken::Eof],
    );
}

#[test]
fn test_comparison_neq() {
    let mut lexer = make_lexer("!=");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(RComparisonOp::Ne), RToken::Eof],
    );
}

#[test]
fn test_comparison_gt() {
    let mut lexer = make_lexer(">");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(RComparisonOp::Gt), RToken::Eof],
    );
}

#[test]
fn test_comparison_lt() {
    let mut lexer = make_lexer("<");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(RComparisonOp::Lt), RToken::Eof],
    );
}

#[test]
fn test_comparison_geq() {
    let mut lexer = make_lexer(">=");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(RComparisonOp::Geq), RToken::Eof],
    );
}

#[test]
fn test_comparison_leq() {
    let mut lexer = make_lexer("<=");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![cmp_token(RComparisonOp::Leq), RToken::Eof],
    );
}

#[test]
fn test_skip_whitespace() {
    let mut lexer = make_lexer("   fn");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Fn, RToken::Eof]);
}

#[test]
fn test_skip_tabs_and_newlines() {
    let mut lexer = make_lexer("\t\n\r  fn");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Fn, RToken::Eof]);
}

#[test]
fn test_skip_line_comment() {
    let mut lexer = make_lexer("// comment\nfn");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Fn, RToken::Eof]);
}

#[test]
fn test_skip_multiple_comments() {
    let mut lexer = make_lexer("// first\n// second\nfn");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Fn, RToken::Eof]);
}

#[test]
fn test_comment_at_eof() {
    let mut lexer = make_lexer("// comment");
    assert_tokens(collect_tokens(&mut lexer), vec![RToken::Eof]);
}

#[test]
fn test_function_signature() {
    let mut lexer = make_lexer("fn foo(x: usize) -> u8");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            RToken::Fn,
            ident("foo"),
            RToken::LParen,
            ident("x"),
            RToken::Colon,
            RToken::Usize,
            RToken::RParen,
            RToken::Arrow,
            RToken::U8,
            RToken::Eof,
        ],
    );
}

#[test]
fn test_let_statement() {
    let mut lexer = make_lexer("let x: usize = 42;");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            RToken::Let,
            ident("x"),
            RToken::Colon,
            RToken::Usize,
            RToken::Assign,
            int_lit(42),
            RToken::SemiColon,
            RToken::Eof,
        ],
    );
}

#[test]
fn test_match_arm() {
    let mut lexer = make_lexer("match x { 1 => 2, }");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            RToken::Match,
            ident("x"),
            RToken::LBrace,
            int_lit(1),
            RToken::FatArrow,
            int_lit(2),
            RToken::Comma,
            RToken::RBrace,
            RToken::Eof,
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
            cmp_token(RComparisonOp::Eq),
            ident("b"),
            cmp_token(RComparisonOp::Ne),
            ident("c"),
            cmp_token(RComparisonOp::Lt),
            ident("d"),
            cmp_token(RComparisonOp::Gt),
            ident("e"),
            cmp_token(RComparisonOp::Leq),
            ident("f"),
            cmp_token(RComparisonOp::Geq),
            ident("g"),
            RToken::Eof,
        ],
    );
}

#[test]
fn test_enum_definition() {
    let mut lexer = make_lexer("enum Foo { A, B(usize) }");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![
            RToken::Enum,
            ident("Foo"),
            RToken::LBrace,
            ident("A"),
            RToken::Comma,
            ident("B"),
            RToken::LParen,
            RToken::Usize,
            RToken::RParen,
            RToken::RBrace,
            RToken::Eof,
        ],
    );
}

#[test]
fn test_path_with_double_colon() {
    let mut lexer = make_lexer("Foo::Bar");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![ident("Foo"), RToken::DoubleColon, ident("Bar"), RToken::Eof],
    );
}

#[test]
fn test_reference_and_mut() {
    let mut lexer = make_lexer("&mut x");
    assert_tokens(
        collect_tokens(&mut lexer),
        vec![RToken::Ampersand, RToken::Mut, ident("x"), RToken::Eof],
    );
}

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
            RToken::Fn,
            ident("main"),
            RToken::LParen,
            RToken::RParen,
            RToken::LBrace,
            RToken::Let,
            ident("msg"),
            RToken::Colon,
            RToken::Ampersand,
            RToken::Str,
            RToken::Assign,
            str_lit("Hello, World!"),
            RToken::SemiColon,
            RToken::RBrace,
            RToken::Eof,
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
            RToken::Enum,
            ident("Option"),
            RToken::LBrace,
            ident("Some"),
            RToken::LParen,
            RToken::Usize,
            RToken::RParen,
            RToken::Comma,
            ident("None"),
            RToken::Comma,
            RToken::RBrace,
            // fn unwrap(opt: Option) -> usize {
            RToken::Fn,
            ident("unwrap"),
            RToken::LParen,
            ident("opt"),
            RToken::Colon,
            ident("Option"),
            RToken::RParen,
            RToken::Arrow,
            RToken::Usize,
            RToken::LBrace,
            // match opt {
            RToken::Match,
            ident("opt"),
            RToken::LBrace,
            // Option::Some(x) => x,
            ident("Option"),
            RToken::DoubleColon,
            ident("Some"),
            RToken::LParen,
            ident("x"),
            RToken::RParen,
            RToken::FatArrow,
            ident("x"),
            RToken::Comma,
            // Option::None => 0,
            ident("Option"),
            RToken::DoubleColon,
            ident("None"),
            RToken::FatArrow,
            int_lit(0),
            RToken::Comma,
            // closing braces
            RToken::RBrace,
            RToken::RBrace,
            RToken::Eof,
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
            RToken::Fn,
            ident("factorial"),
            RToken::LParen,
            ident("n"),
            RToken::Colon,
            RToken::Usize,
            RToken::RParen,
            RToken::Arrow,
            RToken::Usize,
            RToken::LBrace,
            // let mut result: usize = 1;
            RToken::Let,
            RToken::Mut,
            ident("result"),
            RToken::Colon,
            RToken::Usize,
            RToken::Assign,
            int_lit(1),
            RToken::SemiColon,
            // let mut i: usize = 1;
            RToken::Let,
            RToken::Mut,
            ident("i"),
            RToken::Colon,
            RToken::Usize,
            RToken::Assign,
            int_lit(1),
            RToken::SemiColon,
            // while i <= n {
            RToken::While,
            ident("i"),
            cmp_token(RComparisonOp::Leq),
            ident("n"),
            RToken::LBrace,
            // result = result * i;
            ident("result"),
            RToken::Assign,
            ident("result"),
            RToken::Star,
            ident("i"),
            RToken::SemiColon,
            // i = i + 1;
            ident("i"),
            RToken::Assign,
            ident("i"),
            RToken::Plus,
            int_lit(1),
            RToken::SemiColon,
            // }
            RToken::RBrace,
            // return result;
            RToken::Return,
            ident("result"),
            RToken::SemiColon,
            RToken::RBrace,
            RToken::Eof,
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
            RToken::Fn,
            ident("max"),
            RToken::LParen,
            ident("a"),
            RToken::Colon,
            RToken::Usize,
            RToken::Comma,
            ident("b"),
            RToken::Colon,
            RToken::Usize,
            RToken::RParen,
            RToken::Arrow,
            RToken::Usize,
            RToken::LBrace,
            // if a > b {
            RToken::If,
            ident("a"),
            cmp_token(RComparisonOp::Gt),
            ident("b"),
            RToken::LBrace,
            // return a;
            RToken::Return,
            ident("a"),
            RToken::SemiColon,
            RToken::RBrace,
            // else {
            RToken::Else,
            RToken::LBrace,
            // return b;
            RToken::Return,
            ident("b"),
            RToken::SemiColon,
            RToken::RBrace,
            RToken::RBrace,
            RToken::Eof,
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
            RToken::Fn,
            ident("write_byte"),
            RToken::LParen,
            ident("ptr"),
            RToken::Colon,
            RToken::Star,
            RToken::Mut,
            RToken::U8,
            RToken::Comma,
            ident("offset"),
            RToken::Colon,
            RToken::Usize,
            RToken::Comma,
            ident("value"),
            RToken::Colon,
            RToken::U8,
            RToken::RParen,
            RToken::LBrace,
            // let target: *mut u8 = ptr as usize + offset as *mut u8;
            RToken::Let,
            ident("target"),
            RToken::Colon,
            RToken::Star,
            RToken::Mut,
            RToken::U8,
            RToken::Assign,
            ident("ptr"),
            RToken::As,
            RToken::Usize,
            RToken::Plus,
            ident("offset"),
            RToken::As,
            RToken::Star,
            RToken::Mut,
            RToken::U8,
            RToken::SemiColon,
            // *target = value;
            RToken::Star,
            ident("target"),
            RToken::Assign,
            ident("value"),
            RToken::SemiColon,
            RToken::RBrace,
            RToken::Eof,
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
            RToken::Fn,
            ident("add"),
            RToken::LParen,
            ident("a"),
            RToken::Colon,
            RToken::Usize,
            RToken::Comma,
            ident("b"),
            RToken::Colon,
            RToken::Usize,
            RToken::RParen,
            RToken::Arrow,
            RToken::Usize,
            RToken::LBrace,
            RToken::Return,
            ident("a"),
            RToken::Plus,
            ident("b"),
            RToken::SemiColon,
            RToken::RBrace,
            RToken::Eof,
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
            RToken::Unsafe,
            RToken::Fn,
            ident("flag"),
            RToken::LParen,
            ident("x"),
            RToken::Colon,
            RToken::Bool,
            RToken::RParen,
            RToken::Arrow,
            RToken::Bool,
            RToken::LBrace,
            RToken::Return,
            bool_lit(true),
            RToken::SemiColon,
            RToken::RBrace,
            RToken::Eof,
        ],
    );
}
