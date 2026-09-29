//! The slice of the Mach VM interface mechanism (a) needs, declared directly against
//! `libSystem`. No crate dependency, no build script.
//!
//! Types follow `<mach/mach_vm.h>` and `<mach/vm_types.h>` on arm64 macOS:
//! `mach_vm_address_t`/`mach_vm_size_t` are `uint64_t`, `vm_address_t`/`vm_size_t` are
//! `uintptr_t`, `boolean_t` and `integer_t` are `int`, `vm_prot_t` is `int`.

#![allow(non_camel_case_types)]
// An FFI declaration module: it declares the whole slice of the interface it describes,
// including the constants a reader needs to check the calls against the headers.
#![allow(dead_code)]

use std::io;

pub type kern_return_t = i32;
pub type mach_port_t = u32;
pub type vm_map_t = mach_port_t;
pub type vm_prot_t = i32;
pub type vm_inherit_t = u32;
pub type boolean_t = i32;
pub type mach_vm_address_t = u64;
pub type mach_vm_size_t = u64;
pub type vm_address_t = usize;
pub type vm_size_t = usize;
pub type natural_t = u32;
pub type integer_t = i32;

pub const KERN_SUCCESS: kern_return_t = 0;

pub const VM_FLAGS_FIXED: i32 = 0x0000;
pub const VM_FLAGS_ANYWHERE: i32 = 0x0001;
pub const VM_FLAGS_OVERWRITE: i32 = 0x4000;

pub const VM_PROT_NONE: vm_prot_t = 0x0;
pub const VM_PROT_READ: vm_prot_t = 0x1;
pub const VM_PROT_WRITE: vm_prot_t = 0x2;

pub const VM_INHERIT_SHARE: vm_inherit_t = 0;
pub const VM_INHERIT_COPY: vm_inherit_t = 1;
pub const VM_INHERIT_NONE: vm_inherit_t = 2;

/// `MACH_TASK_BASIC_INFO`, and its count in `natural_t` units.
pub const MACH_TASK_BASIC_INFO: i32 = 20;
pub const MACH_TASK_BASIC_INFO_COUNT: u32 = 12;

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct time_value_t {
    pub seconds: integer_t,
    pub microseconds: integer_t,
}

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct mach_task_basic_info {
    pub virtual_size: u64,
    pub resident_size: u64,
    pub resident_size_max: u64,
    pub user_time: time_value_t,
    pub system_time: time_value_t,
    pub policy: integer_t,
    pub suspend_count: integer_t,
}

extern "C" {
    /// The *name* of the current task's port. A global in libSystem; `mach_task_self()`
    /// is a macro for reading it, so read it directly.
    static mach_task_self_: mach_port_t;

    pub fn mach_vm_allocate(
        target: vm_map_t,
        address: *mut mach_vm_address_t,
        size: mach_vm_size_t,
        flags: i32,
    ) -> kern_return_t;

    pub fn mach_vm_deallocate(
        target: vm_map_t,
        address: mach_vm_address_t,
        size: mach_vm_size_t,
    ) -> kern_return_t;

    #[allow(clippy::too_many_arguments)]
    pub fn mach_vm_remap(
        target_task: vm_map_t,
        target_address: *mut mach_vm_address_t,
        size: mach_vm_size_t,
        mask: mach_vm_address_t,
        flags: i32,
        src_task: vm_map_t,
        src_address: mach_vm_address_t,
        copy: boolean_t,
        cur_protection: *mut vm_prot_t,
        max_protection: *mut vm_prot_t,
        inheritance: vm_inherit_t,
    ) -> kern_return_t;

    pub fn vm_copy(
        target_task: vm_map_t,
        source_address: vm_address_t,
        size: vm_size_t,
        dest_address: vm_address_t,
    ) -> kern_return_t;

    pub fn task_info(
        target_task: mach_port_t,
        flavor: i32,
        task_info_out: *mut natural_t,
        task_info_outCnt: *mut u32,
    ) -> kern_return_t;

    pub fn mach_error_string(error_value: kern_return_t) -> *const std::os::raw::c_char;
}

#[inline]
pub fn task_self() -> mach_port_t {
    // SAFETY: reading an immutable global set up by libSystem before main.
    unsafe { mach_task_self_ }
}

pub fn kr_name(kr: kern_return_t) -> String {
    if kr == KERN_SUCCESS {
        return "KERN_SUCCESS".to_string();
    }
    // SAFETY: mach_error_string returns a static NUL-terminated string for any value.
    unsafe {
        let p = mach_error_string(kr);
        if p.is_null() {
            format!("kern_return_t {kr}")
        } else {
            format!(
                "{} (kern_return_t {kr})",
                std::ffi::CStr::from_ptr(p).to_string_lossy()
            )
        }
    }
}

pub fn check(what: &str, kr: kern_return_t) -> io::Result<()> {
    if kr == KERN_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::other(format!("{what}: {}", kr_name(kr))))
    }
}

/// Resident size of this task, in bytes.
///
/// Kernel copy-on-write costs do not show up in any heap accounting, so this is how the
/// benchmark attributes them: pages the kernel has actually had to copy become resident,
/// pages still shared do not.
pub fn resident_bytes() -> u64 {
    let mut info = mach_task_basic_info::default();
    let mut count = MACH_TASK_BASIC_INFO_COUNT;
    // SAFETY: info is a correctly sized mach_task_basic_info and count matches it.
    let kr = unsafe {
        task_info(
            task_self(),
            MACH_TASK_BASIC_INFO,
            &mut info as *mut _ as *mut natural_t,
            &mut count,
        )
    };
    if kr == KERN_SUCCESS {
        info.resident_size
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_info_struct_matches_its_declared_count() {
        // If this ever fails, task_info would write past the struct.
        assert_eq!(
            std::mem::size_of::<mach_task_basic_info>(),
            MACH_TASK_BASIC_INFO_COUNT as usize * std::mem::size_of::<natural_t>()
        );
    }

    #[test]
    fn resident_size_is_plausible() {
        assert!(resident_bytes() > 256 * 1024, "resident size looks unset");
    }
}
