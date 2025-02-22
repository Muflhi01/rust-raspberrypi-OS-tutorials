// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2022 Andre Richter <andre.o.richter@gmail.com>

//! A simple sanity test to see if exception restore code works.

#![feature(format_args_nl)]
#![no_main]
#![no_std]

/// Console tests should time out on the I/O harness in case of panic.
mod panic_wait_forever;

use core::arch::asm;
use libkernel::{bsp, cpu, exception, info, memory, println};

#[inline(never)]
fn nested_system_call() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        asm!("svc #0x1337", options(nomem, nostack, preserves_flags));
    }

    #[cfg(not(target_arch = "aarch64"))]
    {
        info!("Not supported yet");
        cpu::wait_forever();
    }
}

#[no_mangle]
unsafe fn kernel_init() -> ! {
    use libkernel::driver::interface::DriverManager;

    exception::handling_init();

    // This line will be printed as the test header.
    println!("Testing exception restore");

    let phys_kernel_tables_base_addr = match memory::mmu::kernel_map_binary() {
        Err(string) => {
            info!("Error mapping kernel binary: {}", string);
            cpu::qemu_exit_failure()
        }
        Ok(addr) => addr,
    };

    if let Err(e) = memory::mmu::enable_mmu_and_caching(phys_kernel_tables_base_addr) {
        info!("Enabling MMU failed: {}", e);
        cpu::qemu_exit_failure()
    }
    // Printing will silently fail from here on, because the driver's MMIO is not remapped yet.

    memory::mmu::post_enable_init();
    bsp::console::qemu_bring_up_console();

    // Bring up the drivers needed for printing first.
    for i in bsp::driver::driver_manager()
        .early_print_device_drivers()
        .iter()
    {
        // Any encountered errors cannot be printed yet, obviously, so just safely park the CPU.
        i.init().unwrap_or_else(|_| cpu::qemu_exit_failure());
    }
    bsp::driver::driver_manager().post_early_print_device_driver_init();
    // Printing available again from here on.

    info!("Making a dummy system call");

    // Calling this inside a function indirectly tests if the link register is restored properly.
    nested_system_call();

    info!("Back from system call!");

    // The QEMU process running this test will be closed by the I/O test harness.
    cpu::wait_forever();
}
