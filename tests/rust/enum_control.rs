enum Primitive {
    Int(usize),
    Char(char),
}

fn test() -> usize {
    let prim: Primitive = if true {
        Primitive::Int(6)
    } else {
        Primitive::Char('0')
    };

    let prim2: Primitive = match &prim {
        Primitive::Int(x) => Primitive::Char(*x as u8 as char),
        Primitive::Char(c) => Primitive::Int(*c as usize),
    };

    match prim {
        Primitive::Int(x) => {
            x + match prim2 {
                Primitive::Int(x) => x,
                Primitive::Char(c) => c as usize,
            }
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
