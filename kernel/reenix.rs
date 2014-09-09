#![crate_name = "main"]
#![crate_type = "lib"]

#![no_std]
#![no_split_stack]

#![feature(asm, macro_rules, default_type_params, phase, globs, lang_items, intrinsics)]

// The plugin phase imports compiler plugins, including regular macros.

extern crate core;

pub mod main;

#[no_mangle]
#[no_split_stack]
pub extern "C" fn __morestack() {
    ()
}
