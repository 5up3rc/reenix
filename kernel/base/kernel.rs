// TODO Copyright Header

//! The Reenix base kernel-CPU Interface stuff


use libc::c_void;

/// The linker script will initialize these symbols. Note
/// that the linker does not actually allocate any space
/// for these variables (thus the void type) it only sets
/// the address that the symbol points to. So for example
/// the address where the kernel ends is &kernel_end,
/// NOT kernel_end.
#[allow(dead_code)]
extern "C" {
    #[link_name="kernel_start"]
    static start : *const c_void;
    #[link_name="kernel_start_text"]
    static start_text : *const c_void;
    #[link_name="kernel_start_data"]
    static start_data : *const c_void;
    #[link_name="kernel_start_bss"]
    static start_bss : *const c_void;
    #[link_name="kernel_start_init"]
    static start_init: *const c_void;

    #[link_name="kernel_end"]
    static end : *const c_void;
    #[link_name="kernel_end_text"]
    static end_text : *const c_void;
    #[link_name="kernel_end_data"]
    static end_data : *const c_void;
    #[link_name="kernel_end_bss"]
    static end_bss : *const c_void;
    #[link_name="kernel_end_init"]
    static end_init: *const c_void;
}

// TODO I maybe should move this to a different module.
/// This stops everything.
#[no_split_stack]
#[inline]
pub fn halt() -> ! {
    unsafe {
        asm!("cli; hlt");
    }
    loop {}
}
