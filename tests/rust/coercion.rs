fn test() -> usize {
    let mut n: usize = 3;
    let r: &usize = &mut n;
    g();
    if *r == 0 {
        unsafe { exit(2) }
    } else {
        *r + f(&mut n)
    }
}

fn f(x: &usize) -> usize {
    *x
}

enum Option {
    Some(usize),
    None,
}

fn g() {
    return;
    let opt: Option = Option::Some(4);
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
