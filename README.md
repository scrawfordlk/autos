# Autos

Autos is a self-contained 64-bit software system that consists of:

- A self-compiling compiler that compiles Rawrust,
  a tiny subset of the Rust programming language
- A self-emulating emulator that emulates LLLVM,
  a tiny subset of LLVM Intermediate Representation

This project serves as my bachelor project, which is inspired by
the [Selfie project](https://github.com/cksystemsteaching/selfie/).

## Demo

To run the system, you need a rust compiler (typically `rustc`).
You can then bootstrap the system using `rustc` by invoking:

```bash
rustc src/main.rs -o autos -O
```

Alternatively, you can use `make`:

```bash
make
```

(In general `make` will have a few useful "shortcuts" for common invocations).

Once the system is bootstrapped, you can start compiling Rawrust programs, such
as Autos itself, i.e. we can attempt self-compilation:

```bash
./autos -c src/main.rs -o autos.ll
```

You can then process/execute the generated LLLVM
using any LLVM-tool (`clang`, `lli`, `opt`, $\dots$)
or you can emulate it using Autos:

```bash
./autos -e autos.ll 100 -c src/main.rs -o autos2.ll
```

The 100 here specifies how much memory (in MB)
the emulator may use, as it manages its own memory.

You can also compose these command line arguments into one invocation:

```bash
./autos -c src/main.rs -o autos.ll -e 100 -c src/main.rs -o autos2.ll
```

This self-compiles autos, then emulates the resulting LLLVM
and self-compiles itself again.

This self-referential cycle can be repeated arbitrarily many times,
but layering multiple emulations on top of each other naturally slows
down execution by a lot.
For instance, emulating self-compilation under emulation of
the self-compiled system takes many hours to complete:

```bash
./autos -c src/main.rs \
 -e 200 -c src/main.rs \
 -e 100 -c src/main.rs -o autos3.ll
```

While questionable, Autos also allows you to skip the
semantic analysis phase of the compiler, which leads to
faster compile times, but can silently generate semantically incorrect LLLVM,
if the given source program is not a semantically correct Rawrust program:

```bash
./autos -c src/main.rs --unsafe
```

## Running tests

Testing requires installation of the following tools:

- `rustc`, `cargo` to run the tests
- `clang`, `lli` (often part of an `llvm` package)
- `diff` to check fix-points

Invoking the tests can be done by entering the `tests/` directory and invoking `cargo`:

```bash
cd tests && cargo test
```

Or by using `make:`

```bash
make test
```

This runs the tests for Rawrust, LLLVM and a self-compilation test, which
checks for the fix-point when self-compiling under emulation.

There is an additional test, which is by default disabled,
due to it taking close to a minute to complete.
It tests whether the fix-point is retained
when emulating self-compilation under emulation:

```bash
cd tests && cargo test -- --ignored
```

Or simply:

```bash
make test-self-self
```
