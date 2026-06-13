fn test() -> usize {
    let s: &str = "Hello";
    let mut s: String = string(s);
    string_push_str(&mut s, " World!\n");

    let mut sum: usize = 0;
    let mut i: usize = 0;
    while i < string_len(&s) {
        sum = sum + string_get(&s, i) as usize;
        i = i + 1;
    }

    let length: usize = string_len(&s); // = 12
    let String::Inner(_, _, capacity): String = s; // = 20

    (sum == 1085) as usize + length + capacity + 9
}

enum String {
    Inner(*mut u8, usize, usize),
}

fn string_new() -> String {
    let p: *mut u8 = unsafe { malloc(10) };
    String::Inner(p, 0, 10)
}

fn string(str: &str) -> String {
    let mut s: String = string_new();
    string_push_str(&mut s, str);
    s
}

fn string_len(String::Inner(_, len, _): &String) -> usize {
    *len
}

fn string_push_str(String::Inner(ptr, len, cap): &mut String, str: &str) {
    if *len + str::len(str) > *cap {
        while *len + str::len(str) > *cap {
            *cap = *cap * 2;
        }
        let new_ptr: *mut u8 = unsafe { malloc(*cap) };
        unsafe { memcopy(new_ptr, *ptr, *len) };
        *ptr = new_ptr;
    }
    unsafe { memcopy(ptr_add(*ptr, *len), str::as_ptr(str) as *mut u8, str::len(str)) };
    *len = *len + str::len(str);
}

fn string_get(String::Inner(ptr, len, _): &String, index: usize) -> char {
    if index > *len {
        return 0 as u8 as char;
    } else {
        unsafe { *ptr_add(*ptr, index) as char }
    }
}

unsafe fn memcopy(dest_u8: *mut u8, src_u8: *mut u8, byte_count: usize) {
    let mut i: usize = 0;
    while i < byte_count {
        unsafe { *ptr_add(dest_u8, i) = *ptr_add(src_u8, i) };
        i = i + 1;
    }
}

fn ptr_add(ptr: *mut u8, n: usize) -> *mut u8 {
    (ptr as usize + n) as *mut u8
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn malloc(size: usize) -> *mut u8;
}
