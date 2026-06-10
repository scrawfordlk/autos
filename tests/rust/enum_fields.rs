// TODO: should contain another enum
enum Structure {
    VariantA(usize, char, char, u8),
    VariantB(u8, u8, usize),
    VariantC(*mut Structure, u8),
}

fn test() -> usize {
    let s1: Structure = Structure::VariantA(134, 'A', '0', 4 as u8);
    let s2: Structure = Structure::VariantB(233 as u8, 255 as u8, 10);
    let mut inner: Structure = Structure::VariantA(120, '5', '0', 7 as u8);
    let s3: Structure = Structure::VariantC(&mut inner as *mut Structure, 1 as u8);

    (match_test(s1) == 17) as usize + (match_test(s2) == 12) as usize + (match_test(s3) == 13) as usize + 39
}

fn match_test(s: Structure) -> usize {
    match s {
        Structure::VariantC(ptr, u1) => unsafe {
            // TODO: handle matching on references
            // let s: &Structure = &*ptr;
            // match &s {
            //     Structure::VariantB(u1, u2, _) => (*u1 + *u2) as usize,
            //     Structure::VariantA(m, c1, c2, u1) => *m - *c1 as usize - *c2 as usize + *u1 as usize,
            //     Structure::VariantC(_, u) => *u as usize,
            // };
            13
        },
        Structure::VariantA(n, c1, c2, u1) => n - c1 as usize - c2 as usize - u1 as usize,
        Structure::VariantB(u1, u2, n) => (u2 - u1) as usize - n,
    }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
