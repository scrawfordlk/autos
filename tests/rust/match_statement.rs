fn test() -> usize {
    let mut x: usize = 19;
    let y: usize = 0;

    match x {
        100 => return 2,
        4 => x = 3,
        mut z => {
            z = 1;
            x = x + z
        },
    }

    let z: usize = 0;
    match x {
        mut z => {
            z = 1;
            x = x + z;
        },
    }

    match x {
        20 => return 5,
        y => x = x + y,
    }

    x + z
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
