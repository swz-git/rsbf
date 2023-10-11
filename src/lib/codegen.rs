use cranelift::{
    codegen::{
        entity::EntityRef,
        ir::{
            condcodes::IntCC, types::I8, AbiParam, Function, InstBuilder,
            MemFlags, Signature, UserFuncName,
        },
        isa::{self, CallConv},
        settings::{self, Configurable},
        verify_function, Context,
    },
    frontend::{FunctionBuilder, FunctionBuilderContext, Variable},
};
use std::{
    error::Error,
    io::{Read, Write},
};
use target_lexicon::Triple;

use crate::{BracketState, Token, TokenKind};

/*
Thanks a LOT! to https://github.com/Rodrigodd

Almost all of this code is from
https://github.com/Rodrigodd/bf-compiler/blob/master/cranelift-jit/src/main.rs
*/

pub fn compile(instructions: Vec<Token>) -> Result<Vec<u8>, Box<dyn Error>> {
    // possible settings: https://docs.rs/cranelift-codegen/latest/src/cranelift_codegen/opt/rustwide/target/x86_64-unknown-linux-gnu/debug/build/cranelift-codegen-b5deaeb0cd154533/out/settings.rs.html#490-664
    let mut builder = settings::builder();
    builder.set("opt_level", "speed").unwrap();
    // issue: https://github.com/bytecodealliance/wasmtime/issues/1148
    // builder.set("preserve_frame_pointers", "false").unwrap();
    // builder.set("use_egraphs", "true").unwrap();

    let flags = settings::Flags::new(builder);

    let isa = match isa::lookup(Triple::host()) {
        Err(_) => panic!("x86_64 ISA is not avaliable"),
        Ok(isa_builder) => isa_builder.finish(flags).unwrap(),
    };

    let pointer_type = isa.pointer_type();

    let call_conv = CallConv::triple_default(isa.triple());

    // get memory address parameter, and return pointer to io::Error
    let mut sig = Signature::new(call_conv);
    sig.params.push(AbiParam::new(pointer_type));
    sig.returns.push(AbiParam::new(pointer_type));

    let mut func = Function::with_name_signature(UserFuncName::user(0, 0), sig);

    let mut func_ctx = FunctionBuilderContext::new();
    let mut builder = FunctionBuilder::new(&mut func, &mut func_ctx);

    let pointer = Variable::new(0);
    builder.declare_var(pointer, pointer_type);

    let exit_block = builder.create_block();
    builder.append_block_param(exit_block, pointer_type);

    let block = builder.create_block();
    builder.seal_block(block);

    builder.append_block_params_for_function_params(block);
    builder.switch_to_block(block);

    let memory_address = builder.block_params(block)[0];

    let zero_byte = builder.ins().iconst(I8, 0);
    let zero = builder.ins().iconst(pointer_type, 0);
    builder.def_var(pointer, zero);

    let mem_flags = MemFlags::new(); //.with_notrap().with_heap();

    let (write_sig, write_address) = {
        let mut write_sig = Signature::new(call_conv);
        write_sig.params.push(AbiParam::new(I8));
        write_sig.returns.push(AbiParam::new(pointer_type));
        let write_sig = builder.import_signature(write_sig);

        let write_address = write as *const () as i64;
        let write_address = builder.ins().iconst(pointer_type, write_address);
        (write_sig, write_address)
    };

    let (read_sig, read_address) = {
        let mut read_sig = Signature::new(call_conv);
        read_sig.params.push(AbiParam::new(pointer_type));
        read_sig.returns.push(AbiParam::new(pointer_type));
        let read_sig = builder.import_signature(read_sig);

        let read_address = read as *const () as i64;
        let read_address = builder.ins().iconst(pointer_type, read_address);
        (read_sig, read_address)
    };

    let mut stack = Vec::new();

    for (_, instr) in instructions.into_iter().enumerate() {
        match instr.kind {
            TokenKind::ValMod(n) => {
                let n = n as i64;
                let pointer_value = builder.use_var(pointer);
                let cell_address =
                    builder.ins().iadd(memory_address, pointer_value);
                let cell_value =
                    builder.ins().load(I8, mem_flags, cell_address, 0);
                let cell_value = builder.ins().iadd_imm(cell_value, n as i64);
                builder.ins().store(mem_flags, cell_value, cell_address, 0);
            }
            TokenKind::PosMod(n) => {
                let n = n as i64;
                let pointer_value = builder.use_var(pointer);
                let pointer_plus = builder.ins().iadd_imm(pointer_value, n);

                let pointer_value = if n > 0 {
                    let wrapped =
                        builder.ins().iadd_imm(pointer_value, n - 30_000);
                    let cmp = builder.ins().icmp_imm(
                        IntCC::SignedLessThan,
                        pointer_plus,
                        30_000,
                    );
                    builder.ins().select(cmp, pointer_plus, wrapped)
                } else {
                    let wrapped =
                        builder.ins().iadd_imm(pointer_value, n + 30_000);
                    let cmp = builder.ins().icmp_imm(
                        IntCC::SignedLessThan,
                        pointer_plus,
                        0,
                    );
                    builder.ins().select(cmp, wrapped, pointer_plus)
                };

                builder.def_var(pointer, pointer_value);
            }
            TokenKind::Output => {
                let pointer_value = builder.use_var(pointer);
                let cell_address =
                    builder.ins().iadd(memory_address, pointer_value);
                let cell_value =
                    builder.ins().load(I8, mem_flags, cell_address, 0);

                let inst = builder.ins().call_indirect(
                    write_sig,
                    write_address,
                    &[cell_value],
                );
                let result = builder.inst_results(inst)[0];

                let after_block = builder.create_block();

                // builder
                //     .ins()
                //     .brif(result, exit_block, &[], after_block, &[]);
                builder.ins().brnz(result, exit_block, &[result]);
                builder.ins().jump(after_block, &[]);

                builder.seal_block(after_block);
                builder.switch_to_block(after_block);
            }
            TokenKind::Input => {
                let pointer_value = builder.use_var(pointer);
                let cell_address =
                    builder.ins().iadd(memory_address, pointer_value);

                let inst = builder.ins().call_indirect(
                    read_sig,
                    read_address,
                    &[cell_address],
                );
                let result = builder.inst_results(inst)[0];

                let after_block = builder.create_block();

                // builder
                //     .ins()
                //     .brif(result, exit_block, &[], after_block, &[]);
                builder.ins().brnz(result, exit_block, &[result]);
                builder.ins().jump(after_block, &[]);

                builder.seal_block(after_block);
                builder.switch_to_block(after_block);
            }
            TokenKind::Bracket(BracketState::Open) => {
                let inner_block = builder.create_block();
                let after_block = builder.create_block();

                let pointer_value = builder.use_var(pointer);
                let cell_address =
                    builder.ins().iadd(memory_address, pointer_value);
                let cell_value =
                    builder.ins().load(I8, mem_flags, cell_address, 0);

                // builder.ins().brif(
                //     cell_value,
                //     inner_block,
                //     &[],
                //     after_block,
                //     &[],
                // );
                builder.ins().brz(cell_value, after_block, &[]);
                builder.ins().jump(inner_block, &[]);

                builder.switch_to_block(inner_block);

                stack.push((inner_block, after_block));
            }
            TokenKind::Bracket(BracketState::Closed) => {
                let (inner_block, after_block) = match stack.pop() {
                    Some(x) => x,
                    None => Err("UnbalancedBrackets")?,
                };

                let pointer_value = builder.use_var(pointer);
                let cell_address =
                    builder.ins().iadd(memory_address, pointer_value);
                let cell_value =
                    builder.ins().load(I8, mem_flags, cell_address, 0);

                // builder.ins().brif(
                //     cell_value,
                //     inner_block,
                //     &[],
                //     after_block,
                //     &[],
                // );
                builder.ins().brnz(cell_value, inner_block, &[]);
                builder.ins().jump(after_block, &[]);

                builder.seal_block(inner_block);
                builder.seal_block(after_block);

                builder.switch_to_block(after_block);
            }
            TokenKind::Clear => {
                let pointer_value = builder.use_var(pointer);
                let cell_address =
                    builder.ins().iadd(memory_address, pointer_value);
                builder.ins().store(mem_flags, zero_byte, cell_address, 0);
            }
            TokenKind::Copy(n) => {
                let n = n as i64;
                let pointer_value = builder.use_var(pointer);
                let to_add = builder.ins().iadd_imm(pointer_value, n);

                let to_add = if n > 0 {
                    let wrapped =
                        builder.ins().iadd_imm(pointer_value, n - 30_000);
                    let cmp = builder.ins().icmp_imm(
                        IntCC::SignedLessThan,
                        to_add,
                        30_000,
                    );
                    builder.ins().select(cmp, to_add, wrapped)
                } else {
                    let wrapped =
                        builder.ins().iadd_imm(pointer_value, n + 30_000);
                    let cmp = builder.ins().icmp_imm(
                        IntCC::SignedLessThan,
                        to_add,
                        0,
                    );
                    builder.ins().select(cmp, wrapped, to_add)
                };

                let from_address =
                    builder.ins().iadd(memory_address, pointer_value);
                let to_address = builder.ins().iadd(memory_address, to_add);

                let from_value =
                    builder.ins().load(I8, mem_flags, from_address, 0);
                let to_value = builder.ins().load(I8, mem_flags, to_address, 0);

                let sum = builder.ins().iadd(to_value, from_value);

                builder.ins().store(mem_flags, sum, to_address, 0);
            }
            TokenKind::Comment => {}
        }
    }

    if !stack.is_empty() {
        Err("UnbalancedBrackets")?
    }

    builder.ins().return_(&[zero]);

    builder.switch_to_block(exit_block);
    builder.seal_block(exit_block);

    let result = builder.block_params(exit_block)[0];
    builder.ins().return_(&[result]);

    builder.finalize();

    let res = verify_function(&func, &*isa);

    if let Err(errors) = res {
        panic!("{}", errors);
    }

    let mut ctx = Context::for_function(func);
    // let code = match ctx.compile(&*isa, &mut ControlPlane::default()) {
    let code = match ctx.compile(&*isa) {
        Ok(x) => x,
        Err(err) => {
            eprintln!("error compiling: {:?}", err);
            std::process::exit(4);
        }
    };

    let code = code.code_buffer().to_vec();

    Ok(code)
}

extern "C" fn write(value: u8) -> *mut std::io::Error {
    // Writing a non-UTF-8 byte sequence on Windows error out.
    if cfg!(target_os = "windows") && value >= 128 {
        return std::ptr::null_mut();
    }

    let mut stdout = std::io::stdout().lock();

    let result = stdout.write_all(&[value]).and_then(|_| stdout.flush());

    match result {
        Err(err) => Box::into_raw(Box::new(err)),
        _ => std::ptr::null_mut(),
    }
}

unsafe extern "C" fn read(buf: *mut u8) -> *mut std::io::Error {
    let mut stdin = std::io::stdin().lock();
    loop {
        let mut value = 0;
        let err = stdin.read_exact(std::slice::from_mut(&mut value));

        if let Err(err) = err {
            if err.kind() != std::io::ErrorKind::UnexpectedEof {
                return Box::into_raw(Box::new(err));
            }
            value = 0;
        }

        // ignore CR from Window's CRLF
        if cfg!(target_os = "windows") && value == b'\r' {
            continue;
        }

        *buf = value;

        return std::ptr::null_mut();
    }
}
