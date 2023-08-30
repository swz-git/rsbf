# Fast (non-JIT) brainfuck interpreter and "compiler" written in rust

Rsbf includes 2 binaries, rsbfi and rsbfc. Rsbfi is a fast (for being non-JIT) optimizing brainfuck interpreter. Rsbfc is a brainfuck to C transpiler that using clang then compiles the C code to a binary executable.

## Runtime dependencies (rsbfc)

- [clang](https://clang.llvm.org/) (make sure it is in [PATH](https://en.wikipedia.org/wiki/PATH_(variable)))

## Install
`cargo install --git https://github.com/swz-git/rsbf`

## Usage
`rsbfc --help` or `rsbfi --help`

## Future plans

- [ ] Custom [cranelift](https://cranelift.dev/)-powered compiler