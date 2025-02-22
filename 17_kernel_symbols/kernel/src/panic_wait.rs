// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2018-2022 Andre Richter <andre.o.richter@gmail.com>

//! A panic handler that infinitely waits.

use crate::{bsp, cpu, exception};
use core::{fmt, panic::PanicInfo};

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

fn _panic_print(args: fmt::Arguments) {
    use fmt::Write;

    unsafe { bsp::console::panic_console_out().write_fmt(args).unwrap() };
}

/// The point of exit for `libkernel`.
///
/// It is linked weakly, so that the integration tests can overload its standard behavior.
#[linkage = "weak"]
#[no_mangle]
fn _panic_exit() -> ! {
    #[cfg(not(feature = "test_build"))]
    {
        cpu::wait_forever()
    }

    #[cfg(feature = "test_build")]
    {
        cpu::qemu_exit_failure()
    }
}

/// Prints with a newline - only use from the panic handler.
///
/// Carbon copy from <https://doc.rust-lang.org/src/std/macros.rs.html>
#[macro_export]
macro_rules! panic_println {
    ($($arg:tt)*) => ({
        _panic_print(format_args_nl!($($arg)*));
    })
}

/// Stop immediately if called a second time.
///
/// # Note
///
/// Using atomics here relieves us from needing to use `unsafe` for the static variable.
///
/// On `AArch64`, which is the only implemented architecture at the time of writing this,
/// [`AtomicBool::load`] and [`AtomicBool::store`] are lowered to ordinary load and store
/// instructions. They are therefore safe to use even with MMU + caching deactivated.
///
/// [`AtomicBool::load`]: core::sync::atomic::AtomicBool::load
/// [`AtomicBool::store`]: core::sync::atomic::AtomicBool::store
fn panic_prevent_reenter() {
    use core::sync::atomic::{AtomicBool, Ordering};

    #[cfg(not(target_arch = "aarch64"))]
    compile_error!("Add the target_arch to above's check if the following code is safe to use");

    static PANIC_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

    if !PANIC_IN_PROGRESS.load(Ordering::Relaxed) {
        PANIC_IN_PROGRESS.store(true, Ordering::Relaxed);

        return;
    }

    _panic_exit()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    use crate::time::interface::TimeManager;

    unsafe { exception::asynchronous::local_irq_mask() };

    // Protect against panic infinite loops if any of the following code panics itself.
    panic_prevent_reenter();

    let timestamp = crate::time::time_manager().uptime();
    let (location, line, column) = match info.location() {
        Some(loc) => (loc.file(), loc.line(), loc.column()),
        _ => ("???", 0, 0),
    };

    panic_println!(
        "[  {:>3}.{:06}] Kernel panic!\n\n\
        Panic location:\n      File '{}', line {}, column {}\n\n\
        {}",
        timestamp.as_secs(),
        timestamp.subsec_micros(),
        location,
        line,
        column,
        info.message().unwrap_or(&format_args!("")),
    );

    _panic_exit()
}
