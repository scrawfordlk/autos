enum Structure {
    VariantA(usize, char, char, u8),
    VariantB(u8, u8, Inner),
    VariantC(*mut Structure, u8),
}

enum Inner {
    Inner(u8, Deep, usize),
}

enum Deep {
    Inner(usize),
}

fn test() -> usize {
    let s1: Structure = Structure::VariantA(134, 'A', '0', 4 as u8);
    let s2: Structure = Structure::VariantB(233 as u8, 255 as u8, Inner::Inner(3 as u8, Deep::Inner(3), 4));
    let mut inner: Structure = Structure::VariantA(107, '5', '0', 7 as u8);
    let s3: Structure = Structure::VariantC(&mut inner as *mut Structure, 1 as u8);

    let deep: Deep = Deep::Inner(13);
    let Inner::Inner(u1, Deep::Inner(d), n): Inner = Inner::Inner(4 as u8, deep, 12);
    let sum: usize = u1 as usize + d + n + param(Inner::Inner(6 as u8, Deep::Inner(100), 4));

    (match_test(s1) == 17) as usize + (match_test(s2) == 12) as usize + (match_test(s3) == 13) as usize + sum
}

fn match_test(s: Structure) -> usize {
    match s {
        Structure::VariantC(ptr, u1) => unsafe {
            let s: &Structure = &*ptr;
            match s {
                Structure::VariantB(_, u2, _) => 7,
                Structure::VariantA(m, c1, c2, u1) => *m - *c1 as usize - *c2 as usize + *u1 as usize,
                Structure::VariantC(_, a) => 8,
            }
        },
        Structure::VariantA(n, c1, c2, u1) => n - c1 as usize - c2 as usize - u1 as usize,
        Structure::VariantB(u1, u2, Inner::Inner(u3, Deep::Inner(d), n)) => {
            (u2 - u1) as usize - u3 as usize - d - n
        },
    }
}

fn param(Inner::Inner(u1, _, n): Inner) -> usize {
    u1 as usize + n
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
