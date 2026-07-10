#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    use std::{
        fs::{remove_file, write},
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
        assert!(tool_available("diff"), "lli is required"); // used to check fixpoint

        for source_path in rust_sources() {
            let label = source_label(&source_path);

            let (emu_exit, llvm_path) = compile_emulate(&source_path);

            let source = std::fs::read_to_string(&source_path).expect("can read rust test source");
            let rust_source = rustc_source(&source);
            let rust_source_path = write_file(&format!("{}-source", label), "rs", rust_source.as_str());
            let rustc_exe_path = unique_path(&format!("{}-rustc", label), "bin");
            run_rustc(&rust_source_path, &rustc_exe_path);
            let rustc_exit = run_binary(&rustc_exe_path);

            let clang_exe_path = unique_path(&format!("{}-clang", label), "bin");
            run_clang(&llvm_path, &clang_exe_path);
            let clang_exit = run_binary(&clang_exe_path);

            let lli_exit = run_lli(&llvm_path);

            assert_eq!(
                emu_exit,
                clang_exit,
                "emulator exit code does not match clang-compiled binary exit code for {}",
                source_path.display()
            );
            assert_eq!(
                emu_exit,
                rustc_exit,
                "emulator exit code does not match rustc-compiled binary exit code for {}",
                source_path.display()
            );
            assert_eq!(
                emu_exit,
                lli_exit,
                "emulator exit code does not match lli emulated exit code for {}",
                source_path.display()
            );
            remove_file(&llvm_path).expect("can remove generated LLVM-IR file");
            remove_file(&rust_source_path).expect("can remove rust source file");
            remove_file(&rustc_exe_path).expect("can remove generated rustc binary");
        }
    }

    #[test]
    fn test_llvm() {
        assert!(tool_available("clang"), "clang is required");
        assert!(tool_available("lli"), "lli is required");

        for llvm_path in llvm_sources() {
            let label = source_label(&llvm_path);

            let emu_exit = emulate_llvm(&llvm_path);

            let clang_exe_path = unique_path(&format!("{}-clang", label), "bin");
            run_clang(&llvm_path, &clang_exe_path);
            let clang_exit = run_binary(&clang_exe_path);

            let lli_exit = run_lli(&llvm_path);

            assert_eq!(
                emu_exit,
                clang_exit,
                "emulator exit code does not match clang-compiled binary exit code for {}",
                llvm_path.display()
            );
            assert_eq!(
                emu_exit,
                lli_exit,
                "emulator exit code does not match lli emulated exit code for {}",
                llvm_path.display()
            );
        }
    }

    #[test]
    fn test_self_compilation() {
        let source = "src/main.rs";
        let l1 = "level1";
        let l2 = "level2";
        let level1 = unique_path(&format!("{}-autos", l1), "ll");
        let level2 = unique_path(&format!("{}-autos", l2), "ll");

        // boostrapping & self-compiling autos
        let status = Command::new("cargo")
            .env("RUSTFLAGS", "-Awarnings") // hide warnings
            .arg("run")
            .arg("--")
            .arg("-c")
            .arg(source)
            .arg("-o")
            .arg(&level1)
            .stdout(Stdio::null())
            .status()
            .expect("can bootstrap and self-compile using the bootstrapped binary");
        assert!(
            status.success(),
            "autos can self-compile using the bootstrapped compiler"
        );

        // lower LLVM-IR into machine code using clang
        let status = Command::new("clang")
            .arg(&level1)
            .arg("-o")
            .arg(&l1)
            .stdout(Stdio::null())
            .status()
            .expect("able to lower self-compiled compiler code to machine code");
        assert!(
            status.success(),
            "clang can lower the generated self-compiled code to machine code"
        );

        // self-compile using the self-compiled binary
        let mut compiler = PathBuf::from(".");
        compiler.push(&l1);
        let status = Command::new(&compiler)
            .arg("-c")
            .arg(source)
            .arg("-o")
            .arg(&level2)
            .stdout(Stdio::null())
            .status()
            .expect("able to self-compile autos");
        assert!(
            status.success(),
            "autos can self-compile using the bootstrapped compiler"
        );

        // Check if the fixpoint was reached
        let status = Command::new("diff")
            .arg(&level1)
            .arg(&level2)
            .status()
            .expect("able diff the LLVM-IR generated outputs of the compiler");
        assert!(
            status.code().expect("diff exits with an exit code") == 0,
            "self-compilation reaches a fixpoint"
        );

        remove_file(&level1).expect("can remove level 1 generated LLVM-IR code");
        remove_file(&l1).expect("can remove clang-compiled level 1 self-compiled autos binary");
        remove_file(&level2).expect("can remove level 2 generated LLVM-IR code");
    }

    fn unique_path(label: &str, extension: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before UNIX_EPOCH")
            .as_nanos();
        path.push(format!("system-{}-{}-{}.{}", label, id(), nanos, extension));
        path
    }

    fn write_file(label: &str, extension: &str, content: &str) -> PathBuf {
        let path = unique_path(label, extension);
        write(&path, content).expect("failed to write system test file");
        path
    }

    fn rustc_source(source: &str) -> std::string::String {
        format!(
            "#![allow(overflowing_literals, unused_parens, unused_assignments, unreachable_code, unused_variables, dead_code, unused_must_use, non_snake_case)]\n{}",
            source
        )
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
        status
            .code()
            .expect(&format!("binary {} terminates with exit code", path.display()))
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
            .arg("-Wno-override-module") // ignores the missing triple warning
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
            .map(PathBuf::from)
            .collect();
        sources.sort_unstable();
        sources
    }

    fn llvm_sources() -> Vec<PathBuf> {
        let mut sources: Vec<_> = WalkDir::new("tests/llvm")
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_str().expect("is string").to_string())
            .map(PathBuf::from)
            .collect();
        sources.sort_unstable();
        sources
    }

    fn compile_emulate(source: &Path) -> (i32, PathBuf) {
        let status = Command::new("cargo")
            .env("RUSTFLAGS", "-Awarnings") // hide warnings
            .arg("run")
            .arg("--")
            .arg("-c")
            .arg(source)
            .arg("-e")
            .stdout(Stdio::null())
            .status()
            .expect("able to run bootstrapped autos");

        let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("code");
        let output = PathBuf::from(format!("{}.ll", stem));
        let error_msg = format!(
            "returns an exit code for source {}",
            source.as_os_str().to_string_lossy()
        );
        (status.code().expect(&error_msg), output)
    }

    fn emulate_llvm(path: &Path) -> i32 {
        let status = Command::new("cargo")
            .env("RUSTFLAGS", "-Awarnings") // hide warnings
            .arg("run")
            .arg("--")
            .arg("-e")
            .arg(path)
            .status()
            .expect("able to run LLVM emulator");
        status.code().expect("returns an exit code")
    }

    fn source_label(path: &Path) -> std::string::String {
        path.file_stem()
            .and_then(|stem| stem.to_str())
            .map_or_else(|| "source".to_owned(), ToOwned::to_owned)
    }
}
