#[test]
fn test_unwrap_char_some() {
    assert_eq!(unwrap::<char>(Option::Some('a')), 'a');
}

#[test]
fn test_string_new() {
    let s = string_new();
    assert_eq!(string_len(&s), 0);
}

#[test]
fn test_string_push() {
    let mut s = string_new();
    string_push(&mut s, 'H');
    assert_eq!(string_len(&s), 1);
    assert_eq!(to_std_string(&s), "H");
}

#[test]
fn test_string_push_multiple() {
    let mut s = string_new();
    for c in ['a', 'b', 'c'] {
        string_push(&mut s, c);
    }
    assert_eq!(string_len(&s), 3);
    assert_eq!(to_std_string(&s), "abc");
}

#[test]
fn test_string_push_str() {
    let mut s = string_new();
    string_push_str(&mut s, "Hello");
    assert_eq!(string_len(&s), 5);
    assert_eq!(to_std_string(&s), "Hello");
}

#[test]
fn test_string_push_str_empty() {
    let mut s = string_new();
    string_push_str(&mut s, "");
    assert_eq!(string_len(&s), 0);
    assert_eq!(to_std_string(&s), "");
}

#[test]
fn test_string_push_and_push_str_combined() {
    let mut s = string_new();
    string_push_str(&mut s, "Hi");
    string_push(&mut s, '!');
    assert_eq!(to_std_string(&s), "Hi!");
}

#[test]
fn test_string_get_out_of_bounds() {
    let s = string_new();
    assert!(matches!(string_get(&s, 0), Option::None));
}

#[test]
fn test_string_get_out_of_bounds_nonempty() {
    let mut s = string_new();
    string_push(&mut s, 'x');
    assert!(matches!(string_get(&s, 1), Option::None));
}

#[test]
fn test_string_grows_for_many_pushes() {
    let mut s = string_new();
    for _ in 0..128 {
        string_push(&mut s, 'x');
    }
    assert_eq!(string_len(&s), 128);
    assert_eq!(to_std_string(&s), "x".repeat(128));
}

#[test]
fn test_ptr_add() {
    let data: [u8; 4] = [10, 20, 30, 40];
    let ptr = data.as_ptr() as *mut u8;
    unsafe {
        for (i, &expected) in data.iter().enumerate() {
            assert_eq!(*ptr_add::<u8>(ptr, i), expected);
        }
    }
}

#[test]
fn test_memcopy() {
    let src = [1u8, 2, 3, 4];
    let mut dest = [0u8; 4];
    unsafe { memcopy::<u8>(dest.as_mut_ptr(), src.as_ptr() as *mut u8, 4) };
    assert_eq!(dest, src);
}

#[test]
fn test_memcopy_partial() {
    let src = [5u8, 6, 7, 8];
    let mut dest = [0u8; 4];
    println!("HEY");
    unsafe { memcopy::<u8>(dest.as_mut_ptr(), src.as_ptr() as *mut u8, 2) };
    assert_eq!(dest, [5, 6, 0, 0]);
}

#[test]
fn test_memcopy_zero() {
    let src = [1u8, 2, 3, 4];
    let mut dest = [0u8; 4];
    unsafe { memcopy::<u8>(dest.as_mut_ptr(), src.as_ptr() as *mut u8, 0) };
    assert_eq!(dest, [0; 4]);
}

fn rType_match(a: &RType, b: &RType) -> bool {
    match (a, b) {
        (RType::U8, RType::U8) => true,
        (RType::Usize, RType::Usize) => true,
        (RType::Bool, RType::Bool) => true,
        (RType::Char, RType::Char) => true,
        (RType::Unit, RType::Unit) => true,
        (RType::Never, RType::Never) => true,
        (RType::Enum(a_name), RType::Enum(b_name)) => string_eq(a_name, b_name),
        _ => false,
    }
}

#[test]
fn test_and() {
    assert_eq!(and(true, true), true);
    assert_eq!(and(true, false), false);
    assert_eq!(and(false, true), false);
    assert_eq!(and(false, false), false);
}

#[test]
fn test_or() {
    assert_eq!(or(true, true), true);
    assert_eq!(or(true, false), true);
    assert_eq!(or(false, true), true);
    assert_eq!(or(false, false), false);
}

#[test]
fn test_string_with_capacity() {
    let mut s = string_with_capacity(32);
    assert_eq!(string_len(&s), 0);
    for _ in 0..32 {
        string_push(&mut s, 'a');
    }
    assert_eq!(string_len(&s), 32);
    assert_eq!(to_std_string(&s), "a".repeat(32));
}

#[test]
fn test_string_clone() {
    let mut s = string("clone me");
    let clone = string_clone(&s);
    string_push(&mut s, '!');
    assert_eq!(to_std_string(&clone), "clone me");
    assert_eq!(to_std_string(&s), "clone me!");
}

#[test]
fn test_type_clone() {
    let custom = RType::Enum(string("MyType"));
    let cloned = rType_clone(&custom);
    assert!(rType_match(&custom, &cloned));
}
