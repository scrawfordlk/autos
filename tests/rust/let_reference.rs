fn test() -> usize {
    let x: usize = 21;
    let x1_ref: &usize = &x;
    let x1_ref_ref: &&usize = &x1_ref;
    let x2_ref_ref: &&usize = &&x;
    **x2_ref_ref + **x1_ref_ref
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
