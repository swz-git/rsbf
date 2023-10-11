# Fast (JIT & non-JIT) brainfuck interpreter and "compiler" written in rust

Rsbf includes 2 binaries, rsbfi and rsbfc. Rsbfi is a fast (both JIT and non-JIT) optimizing brainfuck interpreter. Rsbfc is a brainfuck to C transpiler that compiles the transpiled C code using clang to a binary executable.

## Runtime dependencies (rsbfc)

- [clang](https://clang.llvm.org/) (make sure it is in [PATH](https://en.wikipedia.org/wiki/PATH_(variable)))

## Install
`cargo install --git https://github.com/swz-git/rsbf`

## Usage
`rsbfc --help` or `rsbfi --help`

## Future plans

- [ ] Custom [cranelift](https://cranelift.dev/)-powered compiler