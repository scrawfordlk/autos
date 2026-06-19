enum Primitive {
    Int(usize),
    Char(char),
}

fn test() -> usize {
    let prim: Primitive = if true {
        Primitive::Int(6)
    } else {
        Primitive::Char('a')
    };

    let prim2: Primitive = match &prim {
        Primitive::Int(_) => Primitive::Char('0'),
        Primitive::Char(c) => Primitive::Int(*c as usize),
    };

    match prim {
        Primitive::Int(x) => {
            (match prim2 {
                Primitive::Int(x) => x,
                Primitive::Char(c) => c as usize,
            }) - x
        },
        Primitive::Char(c) => {
            c as usize
                + match prim2 {
                    Primitive::Int(x) => x,
                    Primitive::Char(c) => c as usize,
                }
        },
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
