fn test() -> usize {
    let a: usize = 2;
    let b: usize = {
        let a: u8 = 5 as u8;
        let b: u8 = {
            let a: usize = 6;
            let b: usize = 10;
            a + b
        } as u8;
        (a + b) as usize
    };
    let c: usize = a * b;
    let a: usize = c + a;
    let result: usize = a;
    result
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
