fn test() -> usize {
    (**&&(({ 1241 as u8 } == (1241 % 256) as u8) as usize
        + (13 - (27 > 128) as usize) as u8 as usize)
        - ((10 / 2 * 2) as u8 / ((*&0 < 2) as usize + (4 != 3) as usize) as u8) as usize % 3)
        * ({ 4 + { (3 <= 3) } as usize } - (3 <= 4) as usize)
        - { 4 as u8 - ({ 5 >= { 5 } }) as u8 + ({ 9 } >= 0) as u8 } as usize
        - { 2 }
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
