fn test() -> usize {
    ('\\' as u8
    - ((48 as u8 as char) < '1') as u8 // since integer literals are of usize type, cast into u8 before char 
    - ('\n' as usize + 'A' as usize) as u8
        + ('z' == 'z') as u8
        + 25 as u8) as usize
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
}
