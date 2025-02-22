// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Copyright (c) 2022 Andre Richter <andre.o.richter@gmail.com>

//! Debug symbol support.

use crate::memory::{Address, Virtual};
use core::{cell::UnsafeCell, slice};
use debug_symbol_types::Symbol;

//--------------------------------------------------------------------------------------------------
// Private Definitions
//--------------------------------------------------------------------------------------------------

// Symbol from the linker script.
extern "Rust" {
    static __kernel_symbols_start: UnsafeCell<()>;
}

//--------------------------------------------------------------------------------------------------
// Global instances
//--------------------------------------------------------------------------------------------------

/// This will be patched to the correct value by the "kernel symbols tool" after linking. This given
/// value here is just a (safe) dummy.
#[no_mangle]
static NUM_KERNEL_SYMBOLS: u64 = 0;

//--------------------------------------------------------------------------------------------------
// Private Code
//--------------------------------------------------------------------------------------------------

fn kernel_symbol_section_virt_start_addr() -> Address<Virtual> {
    Address::new(unsafe { __kernel_symbols_start.get() as usize })
}

fn kernel_symbols_slice() -> &'static [Symbol] {
    let ptr = kernel_symbol_section_virt_start_addr().as_usize() as *const Symbol;

    unsafe {
        let num = core::ptr::read_volatile(&NUM_KERNEL_SYMBOLS as *const u64) as usize;
        slice::from_raw_parts(ptr, num)
    }
}

//--------------------------------------------------------------------------------------------------
// Public Code
//--------------------------------------------------------------------------------------------------

/// Retrieve the symbol name corresponding to a virtual address, if any.
pub fn lookup_symbol(addr: Address<Virtual>) -> Option<&'static str> {
    for i in kernel_symbols_slice() {
        if i.contains(addr.as_usize()) {
            return Some(i.name());
        }
    }

    None
}

//--------------------------------------------------------------------------------------------------
// Testing
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use test_macros::kernel_test;

    /// Sanity of symbols module.
    #[kernel_test]
    fn symbols_sanity() {
        let first_sym = lookup_symbol(Address::new(
            crate::common::is_aligned as *const usize as usize,
        ))
        .unwrap();

        assert_eq!(first_sym, "libkernel::common::is_aligned");

        let second_sym =
            lookup_symbol(Address::new(crate::version as *const usize as usize)).unwrap();

        assert_eq!(second_sym, "libkernel::version");
    }
}
