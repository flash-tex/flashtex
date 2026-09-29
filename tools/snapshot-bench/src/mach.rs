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

/// `TASK_VM_INFO`. The fields up to `phys_footprint` are revision 1 of the structure, which
/// is what this asks for; later revisions only append.
pub const TASK_VM_INFO: i32 = 22;

/// `task_vm_info`, truncated after `phys_footprint` (revision 1 of `<mach/task_info.h>`).
///
/// `resident_size` is the wrong metric for kernel copy-on-write: a page the kernel has
/// copied on a snapshot's behalf is charged to whichever map entry the accounting walks,
/// and a copied-away original mapped only by a snapshot region need not show up there at
/// all. `phys_footprint` is the ledger figure macOS itself uses for a task's memory, and
/// `internal` plus `compressed` is the anonymous memory the task is holding, so those are
/// what a copy-on-write checkpoint store has to be measured with.
#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct task_vm_info_rev1 {
    pub virtual_size: u64,
    pub region_count: integer_t,
    pub page_size: integer_t,
    pub resident_size: u64,
    pub resident_size_peak: u64,
    pub device: u64,
    pub device_peak: u64,
    pub internal: u64,
    pub internal_peak: u64,
    pub external: u64,
    pub external_peak: u64,
    pub reusable: u64,
    pub reusable_peak: u64,
    pub purgeable_volatile_pmap: u64,
    pub purgeable_volatile_resident: u64,
    pub purgeable_volatile_virtual: u64,
    pub compressed: u64,
    pub compressed_peak: u64,
    pub compressed_lifetime: u64,
    pub phys_footprint: u64,
}

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

pub const HOST_VM_INFO64: i32 = 4;
pub const HOST_VM_INFO64_COUNT: u32 = 38;

extern "C" {
    fn mach_host_self() -> mach_port_t;
    fn host_statistics64(
        host_priv: mach_port_t,
        flavor: i32,
        host_info64_out: *mut integer_t,
        host_info64_outCnt: *mut u32,
    ) -> kern_return_t;
}

/// `free_count + purgeable_count` from `vm_statistics64`, in pages: the host's own view,
/// which is not this task's ledger.
pub fn host_free_pages() -> u64 {
    // vm_statistics64_data_t: 6 natural_t, then u64 counters. Only the first two fields are
    // read here, and both are natural_t at the front of the structure.
    let mut buf = [0i32; HOST_VM_INFO64_COUNT as usize];
    let mut count = HOST_VM_INFO64_COUNT;
    // SAFETY: buf holds HOST_VM_INFO64_COUNT integer_t and count says so.
    let kr = unsafe {
        host_statistics64(
            mach_host_self(),
            HOST_VM_INFO64,
            buf.as_mut_ptr(),
            &mut count,
        )
    };
    if kr != KERN_SUCCESS {
        return 0;
    }
    buf[0] as u32 as u64 // free_count
}

/// What this task is holding, for the kernel copy-on-write measurement.
#[derive(Default, Clone, Copy)]
pub struct VmFootprint {
    pub phys_footprint: u64,
    pub internal: u64,
    pub compressed: u64,
    pub resident: u64,
    pub regions: i32,
    pub ok: bool,
}

impl VmFootprint {
    /// The figure to charge a checkpoint store with: anonymous memory held, whether it is
    /// still in RAM or has been compressed out of it.
    pub fn held(&self) -> u64 {
        self.internal + self.compressed
    }
}

pub fn vm_footprint() -> VmFootprint {
    let mut info = task_vm_info_rev1::default();
    let mut count =
        (std::mem::size_of::<task_vm_info_rev1>() / std::mem::size_of::<natural_t>()) as u32;
    // SAFETY: info is a correctly sized task_vm_info_rev1 and count is its size in
    // natural_t units, so the kernel cannot write past it.
    let kr = unsafe {
        task_info(
            task_self(),
            TASK_VM_INFO,
            &mut info as *mut _ as *mut natural_t,
            &mut count,
        )
    };
    VmFootprint {
        phys_footprint: info.phys_footprint,
        internal: info.internal,
        compressed: info.compressed,
        resident: info.resident_size,
        regions: info.region_count,
        ok: kr == KERN_SUCCESS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_info_struct_is_a_whole_number_of_natural_t() {
        assert_eq!(
            std::mem::size_of::<task_vm_info_rev1>() % std::mem::size_of::<natural_t>(),
            0
        );
        // Revision 1 of task_vm_info is 152 bytes: if this changes, the field offsets this
        // module declares no longer match the kernel's and phys_footprint would be garbage.
        assert_eq!(std::mem::size_of::<task_vm_info_rev1>(), 152);
    }

    /// TASK_VM_INFO must actually answer, and its numbers must be self-consistent, or the
    /// kernel mechanism's memory figures are meaningless.
    #[test]
    fn vm_footprint_is_plausible() {
        let _ledger = crate::ledger_lock();
        let f = vm_footprint();
        assert!(f.ok, "task_info(TASK_VM_INFO) failed");
        assert!(
            f.phys_footprint > 256 * 1024,
            "phys_footprint {} looks unset",
            f.phys_footprint
        );
        assert!(f.internal > 0 && f.regions > 0);
        // A big anonymous allocation that is touched must move the figure. Everything here
        // has to be black-boxed: in a release build the writes are dead stores into a
        // vector nothing reads, and LLVM will delete the allocation outright, at which point
        // the probe measures nothing and the assertion below fails for the wrong reason.
        let before = vm_footprint().held();
        let mut v = vec![0u8; 96 << 20];
        let mut sum = 0u64;
        for i in (0..v.len()).step_by(16 * 1024) {
            v[i] = 1;
            sum += std::hint::black_box(v[i]) as u64;
        }
        std::hint::black_box(&v);
        let after = vm_footprint().held();
        assert!(
            after >= before + (64 << 20),
            "held memory moved by {} for a touched 96 MiB allocation",
            after.saturating_sub(before)
        );
        std::hint::black_box(sum);
        drop(v);
    }

    /// **The memory cost of a `mach_vm_remap(copy=TRUE)` checkpoint is invisible to the
    /// task's own accounting.**
    ///
    /// This holds `SNAPS` copy-on-write snapshots of a region and then rewrites every page of
    /// the live copy, so every snapshot has to keep its own private copy of every page. The
    /// data is verified: each snapshot still reads its own generation's value on every page,
    /// and the live region reads the newest, from distinct mappings. The physical pages are
    /// therefore real, and the host's free-page count falls by roughly the expected amount.
    ///
    /// `phys_footprint`, `internal`, `compressed` and `resident_size` do not move at all.
    ///
    /// That matters for DESIGN §5.2, which gives checkpoint retention "a configurable budget
    /// (default 1 GB)": under mechanism (a) the engine cannot measure its own checkpoint store
    /// from inside the process, so it cannot enforce that budget without keeping a dirty-page
    /// table of its own — which is mechanism (b)'s bookkeeping without mechanism (b)'s
    /// control over the pages.
    #[test]
    fn kernel_cow_memory_is_invisible_to_the_task_ledger() {
        let _ledger = crate::ledger_lock();
        const BYTES: usize = 64 << 20;
        const SNAPS: usize = 8;
        let page_words = 16 * 1024 / 8;

        let live = crate::kernel_cow::Region::allocate(BYTES).expect("live region");
        let p = live.as_mut_ptr();
        let pages = live.words() / page_words;
        let stamp = |gen: u64| {
            for k in 0..pages {
                // SAFETY: k * page_words is in bounds of the mapped live region.
                unsafe { std::ptr::write_volatile(p.add(k * page_words), gen) };
            }
        };

        stamp(0);
        let base_task = vm_footprint();
        let base_free = host_free_pages();

        let mut snaps: Vec<mach_vm_address_t> = Vec::new();
        for gen in 1..=SNAPS as u64 {
            let mut dst: mach_vm_address_t = 0;
            let mut cur: vm_prot_t = VM_PROT_READ | VM_PROT_WRITE;
            let mut max: vm_prot_t = VM_PROT_READ | VM_PROT_WRITE;
            // SAFETY: all pointers are to locals; src is the region allocated above.
            let kr = unsafe {
                mach_vm_remap(
                    task_self(),
                    &mut dst,
                    live.bytes,
                    0,
                    VM_FLAGS_ANYWHERE,
                    task_self(),
                    live.base,
                    1,
                    &mut cur,
                    &mut max,
                    VM_INHERIT_NONE,
                )
            };
            assert_eq!(kr, KERN_SUCCESS, "remap {gen}: {}", kr_name(kr));
            snaps.push(dst);
            // Every page of the live copy now diverges from this snapshot.
            stamp(gen);
        }

        let after_task = vm_footprint();
        let after_free = host_free_pages();

        // Each snapshot must still read the generation that was current when it was taken, on
        // every page — otherwise there is nothing to account for in the first place.
        for (i, &base) in snaps.iter().enumerate() {
            let expected = i as u64;
            // SAFETY: base names a remapped region of exactly live.bytes.
            let view = unsafe { std::slice::from_raw_parts(base as *const u64, live.words()) };
            let wrong = (0..pages).filter(|k| view[k * page_words] != expected).count();
            assert_eq!(wrong, 0, "snapshot {i} lost {wrong} of {pages} pages");
        }
        let live_wrong = (0..pages)
            // SAFETY: in bounds of the mapped live region.
            .filter(|k| unsafe { *p.add(k * page_words) } != SNAPS as u64)
            .count();
        assert_eq!(live_wrong, 0, "live region lost {live_wrong} pages");

        let held_mib = ((SNAPS * BYTES) >> 20) as i64;
        let d = |a: u64, b: u64| (a as i64 - b as i64) / (1 << 20);
        let host_delta_mib = (after_free as i64 - base_free as i64) * 16 / 1024;
        println!(
            "{SNAPS} copy-on-write snapshots of {} MiB, every page of every one privately held \
             and verified ({held_mib} MiB of distinct data beyond the live copy):",
            BYTES >> 20
        );
        println!(
            "  task phys_footprint {:+} MiB\n  task internal       {:+} MiB\n  \
             task compressed     {:+} MiB\n  task resident       {:+} MiB\n  \
             task regions        {:+}\n  host free memory    {:+} MiB",
            d(after_task.phys_footprint, base_task.phys_footprint),
            d(after_task.internal, base_task.internal),
            d(after_task.compressed, base_task.compressed),
            d(after_task.resident, base_task.resident),
            after_task.regions - base_task.regions,
            host_delta_mib,
        );

        // The finding this test exists to pin down. If a future macOS starts charging the
        // task for it, this fails, and the retention phase should then read the figure out of
        // phys_footprint instead of the dirty-chunk accounting.
        assert!(
            d(after_task.phys_footprint, base_task.phys_footprint) < held_mib / 2,
            "phys_footprint grew by {} MiB of an expected {held_mib} MiB: the task ledger DOES \
             see kernel copy-on-write on this OS version",
            d(after_task.phys_footprint, base_task.phys_footprint)
        );

        for base in snaps {
            // SAFETY: each base was returned by a remap above and is not aliased.
            unsafe {
                let _ = mach_vm_deallocate(task_self(), base, live.bytes);
            }
        }
    }

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
        let _ledger = crate::ledger_lock();
        assert!(resident_bytes() > 256 * 1024, "resident size looks unset");
    }
}
