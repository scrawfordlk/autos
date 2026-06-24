# Autos

Autos (from Ancient Greek αὐτός, meaning "self") is a self-contained 64-bit software system that consists of:
- a self-compiling compiler that compiles RawRust, a tiny subset of the Rust programming language
- a self-emulating emulator that emulates LLLVM-IR, a tiny subset of LLVM Intermediate Representation

This project serves as my bachelor thesis, which is inspired by
the [Selfie project](https://github.com/cksystemsteaching/selfie/).

## Components
### Compiler
Autos is a compiler that is written in and compiles a tiny subset of Rust, called RawRust.

To summarise, RawRust features:
- Strict type-safety
- Guaranteed memory-safety (in the safe subset)
- Generic programming using generic functions/enums
- Rust-like features such as enums and pattern matching

## LLLVM-IR
Autos is also an emulator that can emulate a tiny subset of [LLVM-IR](https://llvm.org/), called LLLVM-IR.

To summarise, LLLVM-IR features:
- A minimal subset of RISC-like instructions
- Organisation of code using functions
- Static string literals
- strict type-safety
- Single Static Assignment (SSA) compliance
