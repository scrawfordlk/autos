fn test() -> usize {
    let n: u8 = 21 as u8;
    let reference: &u8 = &n;

    let a: u8 = match reference {
        10 => return 1,
        x => match *x {
            y => *x + y,
        },
    };

    let mut structure: Top = Top::Top(Mid::Mid(Bot::Bot(5 as u8, 4)));

    let b: usize = match &structure {
        Top::Top(Mid::Mid(Bot::Bot(u1, n))) => *u1 as usize - *n,
    };

    match &mut structure {
        Top::Top(Mid::Mid(Bot::Bot(u1, n))) => {
            *u1 = 42 as u8;
            *n = 42;
        },
    }

    let Top::Top(Mid::Mid(Bot::Bot(u1, n))): Top = structure;

    (a as usize + b + u1 as usize + n) / 3
}

enum Top {
    Top(Mid),
}

enum Mid {
    Mid(Bot),
}

enum Bot {
    Bot(u8, usize),
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
