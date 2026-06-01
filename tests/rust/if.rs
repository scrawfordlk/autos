fn test() -> usize {
    if 4 < 6 {
        if 4 == 5 {
            1
        } else if 0 != 0 {
            2
        } else if 7 == 7 {
            if false { 3 } else { 42 }
        } else {
            4
        }
    } else {
        5
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
