// TODO Copyright Header

#![crate_name="main"]
#![crate_type="rlib"]
#![no_std]

#![feature(globs, phase, macro_rules, asm, if_let, unsafe_destructor)]


#[phase(plugin, link)] extern crate core;
#[phase(plugin, link)] extern crate base;
#[phase(plugin, link)] extern crate procs;
#[phase(plugin, link)] extern crate mm;
extern crate alloc;
extern crate startup;
extern crate libc;
extern crate collections;
extern crate drivers;

use alloc::boxed::*;

use core::fmt::FormatWriter;
use drivers::bytedev;
use drivers::blockdev;
use mm::page;
use drivers::DeviceId;
use core::str::from_utf8;
use procs::cleanup_bootstrap_function;
use base::kernel;
use base::errno;
use procs::kproc;
use procs::kproc::{KProc, Pid, ProcId};
use core::iter::*;
use libc::c_void;
use mm::pagetable;
use core::prelude::*;
use collections::String;
use procs::interrupt;

mod proctest;
mod kshell;

#[no_stack_check]
fn clear_screen(background: u16) {
    for i in range(0u, 80 * 25) {
        unsafe {
            *((0xb8000 + i * 2) as *mut u16) = background << 12;
        }
    }
}

pub fn init_stage1() { }
pub fn init_stage2() { }

#[no_mangle]
#[no_stack_check]
pub extern "C" fn bootstrap(_: i32, _: *mut c_void) -> *mut c_void {
    dbg!(debug::CORE, "Kernel binary:");
    dbg!(debug::CORE, "  text: {:p}-{:p}", &kernel::start_text, &kernel::end_text);
    dbg!(debug::CORE, "  data: {:p}-{:p}", &kernel::start_data, &kernel::end_data);
    dbg!(debug::CORE, "  bss:  {:p}-{:p}", &kernel::start_bss, &kernel::end_bss);

    pagetable::template_init();
    kproc::start_idle_proc(idle_proc_run, 0, 0 as *mut c_void);
}

fn shutdown() -> ! {
    drivers::bytedev::shutdown();
    kernel::halt();
}

// TODO
fn finish_init() {
    // TODO VFS Setup.
    procs::init_stage3();
    drivers::init_stage3();
    interrupt::enable();
    interrupt::set_ipl(interrupt::LOW);
}
extern "C" fn idle_proc_run(_: i32, _: *mut c_void) -> *mut c_void {
    cleanup_bootstrap_function();
    dbg!(debug::CORE, "got into process {} and thread {}", current_proc!(), current_thread!());
    finish_init();
    assert!(KProc::new(String::from_str("Init Proc"), init_proc_run, 0, 0 as *mut c_void).is_ok(),
            "Unable to create init proc");
    let x = KProc::waitpid(Pid(ProcId(1)), 0);
    dbg!(debug::CORE, "done with waitpid");
    match x {
        Ok((pid, pst)) => { dbg!(debug::CORE, "Returned {}, 0x{:x}", pid, pst); },
        Err(errno) => {dbg!(debug::CORE, "returned errno {}", errno);}
    }
    shutdown();
}

extern "C" fn init_proc_run(_: i32, _: *mut c_void) -> *mut c_void {
    interrupt::enable();
    dbg!(debug::CORE, "got into process {} and thread {}", current_proc!(), current_thread!());
    kshell::start(0);
    loop {
        let x = KProc::waitpid(kproc::Any, 0);
        match x {
            Ok((pid, pst)) => { dbg!(debug::CORE, "{} Returned {} (0x{:x})", pid, pst, pst); },
            Err(errno) => {
                dbg!(debug::CORE, "returned errno {}", errno);
                if errno == errno::ECHILD {
                    break;
                }
            }
        }
    }
    return 0 as *mut c_void;
}

mod std {
    pub use core::fmt;
    pub use core::clone;
}
