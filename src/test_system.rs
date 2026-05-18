use std::{
    fs::{read_dir, remove_file, write},
    path::{Path, PathBuf},
    process::{Command, Stdio, id},
    time::{SystemTime, UNIX_EPOCH},
    vec::Vec,
};
use walkdir::WalkDir;

#[test]
fn test_system() {
    assert!(tool_available("rustc"), "rustc is required");
    assert!(tool_available("clang"), "clang is required");
    assert!(tool_available("lli"), "lli is required");

    for source_path in rust_sources() {
        let label = source_label(&source_path);

        // NOTE: Do not use rustc for now, because in Rust you actually can't return any value (while you
        // can in LLVM-IR), which makes testing early on easier, at least until I bootstrap an exit
        // function.
        //
        // let rust_source_path = write_file(&format!("{}-source", label), "rs", &source);
        // let rustc_exe_path = unique_path(&format!("{}-rustc", label), "bin");
        // run_rustc(&rust_source_path, &rustc_exe_path);
        // let rustc_exit = run_binary(&rustc_exe_path);

        let (emu_exit, llvm_path) = compile_emulate(&source_path);

        let clang_exe_path = unique_path(&format!("{}-clang", label), "bin");
        run_clang(&llvm_path, &clang_exe_path);
        let clang_exit = run_binary(&clang_exe_path);

        let lli_exit = run_lli(&llvm_path);

        assert_eq!(
            emu_exit, clang_exit,
            "emulator exit code does not match clang-compiled binary exit code"
        );
        assert_eq!(
            emu_exit, lli_exit,
            "emulator exit code does not match lli emulated exit code"
        );
        remove_file(&llvm_path).expect("can remove generated LLVM-IR file");
    }
}

fn unique_path(label: &str, extension: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before UNIX_EPOCH")
        .as_nanos();
    path.push(format!(
        "thesis-system-{}-{}-{}.{}",
        label,
        id(),
        nanos,
        extension
    ));
    path
}

fn write_file(label: &str, extension: &str, content: &str) -> PathBuf {
    let path = unique_path(label, extension);
    write(&path, content).expect("failed to write system test file");
    path
}

fn tool_available(tool: &str) -> bool {
    match Command::new(tool)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

fn run_binary(path: &Path) -> i32 {
    let status = Command::new(path).status().expect("able to execute binary");
    status.code().expect("binary terminates with exit code")
}

fn run_lli(path: &Path) -> i32 {
    let status = Command::new("lli")
        .arg(path)
        .status()
        .expect("able to execute lli");
    status.code().expect("lli terminates with exit code")
}

fn run_clang(path: &Path, output_path: &Path) {
    let status = Command::new("clang")
        .arg(path)
        .arg("-o")
        .arg(output_path)
        .status()
        .expect("able to execute clang");
    assert!(status.success(), "clang accepts generated LLVM-IR output");
}

fn run_rustc(path: &Path, output_path: &Path) {
    let status = Command::new("rustc")
        .arg("--edition")
        .arg("2024")
        .arg(path)
        .arg("-o")
        .arg(output_path)
        .status()
        .expect("able to execute rustc");
    assert!(status.success(), "rustc accepts Rust source file");
}

fn rust_sources() -> Vec<PathBuf> {
    let mut sources: Vec<_> = WalkDir::new("tests/rust")
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_str().expect("is string").to_string())
        .map(|e| PathBuf::from(e))
        .collect();
    sources.sort_unstable();
    sources
}

fn compile_emulate(source: &Path) -> (i32, PathBuf) {
    let status = Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("-c")
        .arg(source)
        .arg("-e")
        // .stdout(std::process::Stdio::null())
        // .stderr(std::process::Stdio::null())
        .status()
        .expect("able to run bootstrapped compiler/emulator");

    let stem = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("code");
    let output = PathBuf::from(format!("{}.ll", stem));
    (status.code().expect("returns an exit code"), output)
}

fn source_label(path: &Path) -> std::string::String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map_or_else(|| "source".to_owned(), ToOwned::to_owned)
}
