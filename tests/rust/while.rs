fn test() -> usize {
    let exp: usize = 4;
    let mut i: usize = 0;
    let mut product: usize = 1;
    while i < exp {
        product = product * 2;
        i = i + 1;
    }

    let mut done: bool = false;
    let mut i: usize = 0;
    while done as usize == 0 {
        i = i + 1;
        while i == 25 {
            done = true;
            i = i + 1;
        }
    }

    product + i
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
