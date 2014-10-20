// TODO Copyright Header

//! Macro's for failable allocation

#![macro_escape]

/// Allocates a value. If it seems that we are likely to run out of memory we will return
/// Err(AllocError), otherwise Ok(val). It is not nessecarially gaurenteed to work.
///
/// NOTE This is super dangerous and very bad. It is only being done because we cannot use the
/// normal method of detecting alloc failures (namely new_task that returns value or fails if it
/// can't).
#[macro_export]
macro_rules! alloc(
    // TODO This is rather wordy and arbitrary.
    (try $f:expr) => ({
        use mm::alloc;
        use core::mem;
        if alloc::is_memory_low() {
            Err(concat!("Allocation of '", stringify!($f), "' failed due to insufficent memory"))
        } else {
            let x = $f;
            if alloc::is_memory_low() {
                mem::drop(x);
                Err(concat!("Allocation of '", stringify!($f), "' failed due to insufficent memory"))
            } else {
                Ok(x)
            }
        }
    });
    (try_box $e:expr) => ({
        use mm::alloc;
        use core::mem;
        if alloc::is_memory_low() {
            Err(concat!("Allocation of '", stringify!($e), "' failed due to insufficent memory"))
        } else {
            let x = box $e;

            if alloc::is_memory_low() {
                mem::drop(x);
                Err(concat!("Allocation of '", stringify!($e), "' failed due to insufficent memory"))
            } else {
                Ok(x)
            }
        }
    });
)
