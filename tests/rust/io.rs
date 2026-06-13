fn test() -> usize {
    // flags for Linux:
    let O_RDONLY: usize = 0;
    let O_WRONLY_CREAT_TRUNC: usize = 321; // O_WRONLY = 1, O_CREAT = 64, O_TRUNC = 256
    let mode: usize = 420; // = 0o0644

    unsafe {
        let fd: usize = open(
            str::as_ptr("/tmp/test.txt\0") as *mut u8,
            O_WRONLY_CREAT_TRUNC,
            mode,
        );
        let mut c: char = '*';
        let x: usize = write(fd, &mut c as *mut char as *mut u8, 1);
        if x != 1 {
            return x;
        }
    }

    let x: usize = unsafe {
        let fd: usize = open(str::as_ptr("/tmp/test.txt\0") as *mut u8, O_RDONLY, 0);
        let mut buf: char = 69 as u8 as char;
        let x: usize = read(fd, &mut buf as *mut char as *mut u8, 1);
        if x != 1 {
            return x;
        }
        buf as usize
    };

    x
}

fn main() {
    unsafe { exit(test()) }
}

unsafe extern "C" {
    fn exit(code: usize) -> !;
    fn open(path: *mut u8, flags: usize, mode: usize) -> usize;
    fn write(fd: usize, buf: *mut u8, count: usize) -> usize;
    fn read(fd: usize, buf: *mut u8, count: usize) -> usize;
}
