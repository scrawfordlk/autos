fn main() {
    if f() != 42 {
        unsafe { exit(1) }
    }
    never()
}

fn never() -> ! {
    unsafe {
        let x: usize = { (exit(42)) };
    }
}

fn f() -> usize {
    let unit: () = {}; // there is not unit literal
    let never: usize = return 42; // ! ~> usize
    33 // unreachable
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
