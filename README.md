<!-- Add info about the interpreter as well -->

# Brainfuck "compiler" written in rust

Simple rust program that translates brainfuck into C and then compiles it using the native C compiler (currently, it only works with gcc).

## Runtime dependencies

- [GCC](https://gcc.gnu.org/)

## Install
`cargo install --git https://github.com/swz-git/rsbf`

## Usage
run `rsbfc --help`