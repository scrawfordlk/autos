# Autos

Autos is a self-contained 64-bit software system that consists of:

- A self-compiling compiler that compiles RawRust,
  a tiny subset of the Rust programming language
- A self-emulating emulator that emulates LLLVM,
  a tiny subset of LLVM Intermediate Representation

This project serves as my bachelor thesis, which is inspired by
the [Selfie project](https://github.com/cksystemsteaching/selfie/).

## Demo

To run the system, you need `cargo` (build tool) and `rustc` (bootstrapping compiler).
You can then manually invoke:

```bash
cargo build --release && cp target/release/autos .
```

Or use `make`:

```bash
make
```

Once the system is bootstrapped, you can start compiling RawRust programs, such
as Autos, i.e. we can attempt self-compilation:

```bash
./autos -c src/main.rs -o autos.ll
```

You can then execute the generated LLLVM
using the standard LLVM-tools (`clang`, `lli`, $\dots$)
or you can emulate it using Autos:

```bash
./autos -e autos.ll -c src/main.rs
```

Alternatively, instead of the previous two commands, you can also just do:

```bash
./autos -c src/main.rs -e -c src/main.rs
```

Which will self-compile autos, then emulate the resulting LLLVM
and self-compile itself again.

While questionable, Autos also allows you to skip the
semantic analysis phase of the compiler, which leads to
faster compile times, but can silently generate incorrect
or invalid LLLVM, if the given source program is not a correct RawRust program:

```bash
./autos -c src/main.rs -o autos.ll --unsafe
```

## Components

### Compiler

Autos is a compiler that is written in and compiles
a tiny subset of Rust, called RawRust.

To summarise, RawRust features:

- Rust's type-safety, memory-safety and thread-safety
- Generic programming using generic functions/enums
- Rust-like features such as enums and pattern matching

## LLLVM

Autos is also an emulator that can emulate a tiny
subset of [LLVM-IR](https://llvm.org/), called Little LLVM (LLLVM).

To summarise, LLLVM features:

- Organisation of code using functions and basic blocks
- A minimal subset of RISC-like instructions
- Static string literals
- Type-safety
