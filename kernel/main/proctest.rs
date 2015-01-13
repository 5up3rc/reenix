// TODO Copyright Header

use procs::kproc;
use alloc::boxed::*;
use libc::c_void;
use core::prelude::*;
use procs::kthread;
use core::mem::transmute_copy;
use core::intrinsics::transmute;
use procs::kproc::{ProcStatus, ProcId, KProc};
use procs::interrupt;
use procs::sync::*;
use procs::args::ProcArgs;
use alloc::rc::*;
use collections::string::ToString;

const GOOD : *mut c_void = 1 as *mut c_void;
const BAD  : *mut c_void = 0 as *mut c_void;

#[allow(dead_code, unused_must_use)]
pub fn start() {
    use base::debug;
    let (pass, total) = do_run(true);
    dbg!(debug::TEST, "passed {:?} of {:?} tests", pass, total);

    if cfg!(all(not(TEST_LOW_MEMORY), TEST_KILL_ALL)) {
        debug::remove_mode(debug::TEST);
        for i in range::<i32>(0, 10) {
            kproc::KProc::new("fork fn".to_string(), fork_some, i, 0 as *mut c_void);
            kthread::kyield();
        }
        for _ in range::<usize>(0, 10) {
            kthread::kyield();
        }
        dbg!(debug::TEST | debug::CORE, "killing everything");
        kproc::KProc::kill_all();
    }
}

pub fn run() -> (usize, usize) {
    do_run(false)
}

fn do_run(single: bool) -> (usize, usize) {
    // TODO Embarrassing. This is not thread safe...
    let mut total : usize = 0;
    let mut pass : usize = 0;
    macro_rules! basic_test{
        ($name:expr, $v:expr) => ({
            total += 1;
            match kproc::KProc::new(stringify!($name).to_string(), $name, $v, 0 as *mut c_void) {
                Ok(cnt1) => {
                    match kproc::KProc::waitpid(kproc::Pid(cnt1), 0) {
                        Ok((_, status)) => {
                            if status == GOOD as kproc::ProcStatus {
                                dbg!(debug::TESTPASS, "Test {:?} {:?} passes", total, stringify!($name));
                                pass += 1;
                            } else {
                                dbg!(debug::TESTFAIL, "Test {:?} {:?} failed with {:?}", total, stringify!($name), status);
                            }
                        },
                        Err(errno) => {
                            dbg!(debug::TESTFAIL, "test {:?} {:?} failed with errno {:?}", total, stringify!($name), errno);
                        }
                    }
                },
                _ => { dbg!(debug::TEST, "Failed to allocate new process"); },
            }
        });
        ($name:expr) => (basic_test!($name, 0))
    }
    basic_test!(normal_fork);
    basic_test!(kill_self);
    basic_test!(kill_other, 0);
    basic_test!(kill_other, 1);
    basic_test!(kill_other, 4);
    basic_test!(kill_other, 8);
    basic_test!(uncontested_mutex);
    if single {
        basic_test!(contested_mutex, 1);
        basic_test!(contested_mutex, 2);
        basic_test!(contested_mutex, 5);
    }
    basic_test!(better_mutex, 1);
    basic_test!(better_mutex, 2);
    basic_test!(better_mutex, 5);
    basic_test!(send_ignored_intr);
    basic_test!(test_handle_intr);
    basic_test!(test_modify_intr_regs);
    basic_test!(orphan_procs, 1);
    basic_test!(orphan_procs, 3);
    basic_test!(orphan_procs, 5);
    (pass, total)
}

extern "Rust" fn regular_intr_handler(r: &mut interrupt::Registers) {
    dbg!(debug::TEST, "entered intr handler! {:?}", r);
}

extern "C" fn test_handle_intr(_: i32, _: *mut c_void) -> *mut c_void {
    let x = interrupt::register(135, regular_intr_handler);
    assert!(x.is_none());
    unsafe { asm!("int $$135"); }
    interrupt::register(135, interrupt::unhandled_intr);
    //assert!(x == Some(regular_intr_handler));
    GOOD
}

extern "C" fn test_modify_intr_regs(_: i32, _: *mut c_void) -> *mut c_void {
    let res : u32;
    let x = interrupt::register(135, return_intr);
    assert!(x.is_none());
    unsafe { asm!("int $$135" : "={eax}"(res):::"volatile"); }
    interrupt::register(135, interrupt::unhandled_intr);
    res as *mut c_void
}

extern "Rust" fn return_intr(r: &mut interrupt::Registers) {
    dbg!(debug::TEST, "entered intr handler! Initial registers {:?}", r);
    dbg!(debug::TEST, "returning value {:?}", GOOD);
    r.eax = GOOD as u32;
}

#[allow(unused_must_use)]
extern "C" fn orphan_procs(n: i32, _:*mut c_void) -> *mut c_void {
    for i in range(0, n) {
        kproc::KProc::new("ignored".to_string(), orphan_procs, i, 0 as *mut c_void);
    }
    kthread::kyield();
    GOOD
}

// TODO Test for writing interrupt handler.
extern "C" fn send_ignored_intr(_: i32, _: *mut c_void) -> *mut c_void {
    unsafe { asm!("int $$0xEF"); }
    GOOD
}

extern "C" fn kill_self(_: i32, _: *mut c_void) -> *mut c_void {
    (current_proc_mut!()).kill(GOOD as isize);
    BAD
}

extern "C" fn normal_fork(_: i32, _:*mut c_void) -> *mut c_void { GOOD }

#[allow(unused_must_use, dead_code)]
extern "C" fn fork_some(n: i32, _: *mut c_void) -> *mut c_void {
    if n > 0 {
        for i in range::<i32>(1, n) {
            if (current_thread!()).cancelled {
                (current_thread!()).exit((current_thread!()).retval);
            } else {
                kproc::KProc::new("target fn".to_string(), fork_some, i, 0 as *mut c_void);
                kthread::kyield();
            }
        }
    }
    dbg!(debug::TEST, "thread {:?} going to sleep.", n);
    loop {
        kthread::kyield();
        if (current_thread!()).cancelled {
            (current_thread!()).exit((current_thread!()).retval);
        }
        dbg!(debug::TEST, "{:?} {:?} not yet dead", current_proc!(), current_thread!());
    }
}


extern "C" fn to_die(_: i32, _: *mut c_void) -> *mut c_void {
    loop {
        kthread::kyield();
        if (current_thread!()).cancelled {
            (current_thread!()).exit((current_thread!()).retval);
        }
        dbg!(debug::TEST, "to_die thread not yet dead");
    }
}

extern "C" fn to_kill(n: i32, p: *mut c_void) -> *mut c_void {
    for _ in range(0, n) {
        kthread::kyield();
    }
    let pid : Box<ProcId> = unsafe { transmute(p) };
    (*KProc::get_proc(&*pid).expect("there is no process of that pid")).borrow_mut().kill(GOOD as ProcStatus);
    dbg!(debug::TEST, "to_die thread killed");
    GOOD
}

extern "C" fn kill_other(n: i32, _: *mut c_void) -> *mut c_void {
    let target = match kproc::KProc::new("target fn".to_string(), to_die, 0, 0 as *mut c_void) {
        Ok(p) => p,
        _ => { return BAD; },
    };
    let rtarget = box target.clone();
    let sniper = match kproc::KProc::new("sniper fn".to_string(), to_kill, n, unsafe { transmute(rtarget) }) {
        Ok(p) => p,
        _ => { return BAD; },
    };
    let (_, sv) = match KProc::waitpid(kproc::Pid(sniper), 0) {
        Ok(e) => e,
        Err(e) => { dbg!(debug::TESTFAIL, "Waitpid returned {:?}", e); return BAD; }
    };
    let (_, tv) = match KProc::waitpid(kproc::Pid(target), 0) {
        Ok(e) => e,
        Err(e) => { dbg!(debug::TESTFAIL, "Waitpid returned {:?}", e); return BAD; }
    };
    if sv == (GOOD as ProcStatus) && tv == (GOOD as ProcStatus) {
        return GOOD;
    } else {
        return BAD;
    }
}

extern "C" fn uncontested_mutex(_: i32, _: *mut c_void) -> *mut c_void {
    dbg!(debug::TEST, "Attempting to create a mutex and lock it.");
    let x = KMutex::new("test a mutex");
    if x.lock() {
        dbg!(debug::TEST, "locking of mutex succeeded");
        x.unlock();
        dbg!(debug::TEST, "unlocking of mutex succeeded");
        return GOOD;
    } else {
        dbg!(debug::TEST, "Locking of mutex failed.");
        return BAD;
    }
}

static mut c_mutex : *mut KMutex  = 0 as *mut KMutex;
static mut cnt : i32 = 0;

fn get_c_mutex() -> &'static KMutex {
    unsafe { c_mutex.as_ref().expect("CMutex is not set") }
}

#[allow(unused_must_use)]
extern "C" fn contested_mutex(n : i32, _: *mut c_void) -> *mut c_void {
    let y = unsafe {
        let x = box KMutex::new("contested mutex test");
        c_mutex = transmute_copy(&x);
        x
    };

    let high : i32 = 200;

    for _ in range(0, n) {
        // TODO How to make this say which number they are?
        kproc::KProc::new("counter n".to_string(), counter, high, 0 as *mut c_void);
    }

    let mut tot : i32 = 0;
    for _ in range(0, n) {
        let (p, v) = match kproc::KProc::waitpid(kproc::Any, 0) {
            Ok(e) => e,
            Err(_) => { return BAD; },
        };
        dbg!(debug::TEST, "pid {:?} returned {:?}", p, v);
        tot += v as i32;
    }
    let ret = if tot == unsafe { cnt } {
        dbg!(debug::TESTPASS, "successfully counted to {:?} with {:?} counters", tot, n);
        GOOD
    } else {
        dbg!(debug::TESTFAIL, "failed counted to {:?} with {:?} counters, got {:?}", high, n, tot);
        BAD
    };
    unsafe { cnt = 0; c_mutex = 0 as *mut KMutex; }
    drop(y);
    return ret;
}

#[allow(unused_must_use)]
extern "C" fn better_mutex(n : i32, _: *mut c_void) -> *mut c_void {
    let x = Rc::new(Mutex::<i32>::new("contested mutex test", 0));

    let high : i32 = 200;

    for _ in range(0, n) {
        // TODO How to make this say which number they are?
        kproc::KProc::new("better counter n".to_string(), better_counter, high, unsafe { ProcArgs::new(x.clone()).unwrap().to_arg() });
    }

    let mut tot : i32 = 0;
    for _ in range(0, n) {
        let (p, v) = match kproc::KProc::waitpid(kproc::Any, 0) {
            Ok(e) => e,
            Err(_) => { return BAD; },
        };
        dbg!(debug::TEST, "pid {:?} returned {:?}", p, v);
        tot += v as i32;
    }
    let ret = if tot == (*x).lock().and_then(|g| { Ok(*g) }).unwrap_or(0) {
        dbg!(debug::TESTPASS, "successfully counted to {:?} with {:?} counters, using Mutex", tot, n);
        GOOD
    } else {
        dbg!(debug::TESTFAIL, "failed counted to {:?} with {:?} counters, got {:?}, using Mutex", high, n, tot);
        BAD
    };
    assert!(is_unique(&x));
    drop(x);
    return ret;
}

extern "C" fn better_counter(h: i32, v : *mut c_void) -> *mut c_void {
    let mut c : usize = 0;
    let x : Rc<Mutex<i32>> = unsafe { ProcArgs::from_arg(v).unwrap() };
    loop {
        kthread::kyield();
        let mut v = (*x).force_lock();
        if c % 2 == 0 {
            kthread::kyield();
        }
        if *v == h {
            return c as *mut c_void;
        } else {
            *(&mut *v) += 1;
            c += 1;
            if c % 5 == 0 {
                kthread::kyield();
            }
        }
    }
}

extern "C" fn counter(h: i32, _ : *mut c_void) -> *mut c_void {
    let mut c : usize = 0;
    loop {
        if !get_c_mutex().lock() {
            return c as *mut c_void;
        }
        if c % 2 == 0 {
            kthread::kyield();
        }
        if unsafe { cnt == h } {
            get_c_mutex().unlock();
            return c as *mut c_void;
        } else {
            unsafe {cnt += 1; }
            c += 1;
            if c % 5 == 0 {
                kthread::kyield();
            }
            get_c_mutex().unlock();
            kthread::kyield();
        }
    }
}
