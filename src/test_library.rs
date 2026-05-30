// ----------------------- Option<char> --------------------------

#[test]
fn test_unwrap_char_some() {
    assert_eq!(unwrap::<char>(Option::Some('a')), 'a');
}

#[test]
#[should_panic(expected = "tried to unwrap None variant of Option<T>")]
fn test_unwrap_char_none() {
    unwrap::<char>(Option::None);
}

// ------------------------- String ----------------------------

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

// ------------------------- Memory ----------------------------

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

#[test]
fn test_alloc() {
    let ptr = alloc::<u8>(16);
    assert!(!ptr.is_null());
    // Verify zeroed allocation
    unsafe {
        for i in 0..16 {
            assert_eq!(*ptr_add::<u8>(ptr, i), 0);
        }
    }
}

#[test]
fn test_string_push_string() {
    let mut left = string("left");
    let right = string("_right");
    string_push_string(&mut left, &right);
    assert_eq!(to_std_string(&left), "left_right");
}

#[test]
fn test_string_direct() {
    let s = string("hello");
    assert!(string_eq(&s, &string("hello")));
}

fn rAstType_match(a: &RAstType, b: &RAstType) -> bool {
    match (a, b) {
        (RAstType::U8, RAstType::U8) => true,
        (RAstType::Usize, RAstType::Usize) => true,
        (RAstType::Bool, RAstType::Bool) => true,
        (RAstType::Char, RAstType::Char) => true,
        (RAstType::Unit, RAstType::Unit) => true,
        (RAstType::Never, RAstType::Never) => true,
        (RAstType::Custom(a_name), RAstType::Custom(b_name)) => string_eq(a_name, b_name),
        _ => false,
    }
}

fn rAstTypeList_match(a: &List<RAstType>, b: &List<RAstType>) -> bool {
    match (a, b) {
        (List::Nil, List::Nil) => true,
        (List::Cons(a_head, a_tail), List::Cons(b_head, b_tail)) => and(
            rAstType_match(a_head, b_head),
            rAstTypeList_match(
                box_deref::<List<RAstType>>(a_tail),
                box_deref::<List<RAstType>>(b_tail),
            ),
        ),
        _ => false,
    }
}

fn rAstTypeList_single(t: RAstType) -> List<RAstType> {
    List::Cons(t, box_new::<List<RAstType>>(List::Nil))
}

// ------------------------- Bool ----------------------------

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
    let custom = RAstType::Custom(string("MyType"));
    let cloned = rAstType_clone(&custom);
    assert!(rAstType_match(&custom, &cloned));
}
