fn test() -> usize {
    // flags for Linux:
    let O_RDONLY: usize = 0;
    let O_WRONLY_CREAT_TRUNC: usize = 321; // O_WRONLY = 1, O_CREAT = 64, O_TRUNC = 256
    let mode: usize = 420; // = 0o0644
    let usize_MAX: usize = 18446744073709551615; // = -1

    unsafe {
        let fd: usize = open(
            str::as_ptr("/tmp/test.txt\0") as *mut u8,
            O_WRONLY_CREAT_TRUNC,
            mode,
        );
        if fd == usize_MAX {
            return 1;
        }
        let mut c: char = '*';
        let x: usize = write(fd, &mut c as *mut char as *mut u8, 1);
        if x != 1 {
            return 2;
        }

        let fd: usize = open(str::as_ptr("/tmp/test.txt\0") as *mut u8, O_RDONLY, 0);
        if fd == usize_MAX {
            return 1;
        }
        let mut buf: char = 10 as u8 as char;
        let x: usize = read(fd, &mut buf as *mut char as *mut u8, 1);
        if x != 1 {
            return 3;
        }
        buf as usize
    }
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
