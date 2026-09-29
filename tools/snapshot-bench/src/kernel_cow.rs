//! Mechanism (a): kernel copy-on-write over the arenas.
//!
//! The whole state lives in one `mach_vm_allocate`d region. A snapshot is a
//! `mach_vm_remap(copy=TRUE)` of that region to a fresh address; a restore is a second
//! `mach_vm_remap(copy=TRUE)` back over the live range with `VM_FLAGS_OVERWRITE`.
//! [`VmCopySnapshot`] measures the `vm_copy` alternative against the same state.
//!
//! # Why the state cannot live in a `Vec`
//!
//! `mach_vm_remap` and `vm_copy` work on whole pages of a VM region. A `Vec<u64>` comes
//! from `malloc`, which hands out sub-page pieces of much larger spans and keeps its own
//! metadata in them. Remapping over such a range would either fail or silently take
//! somebody else's bytes along. **Mechanism (a) therefore forces every mutable engine
//! arena through a dedicated page-aligned allocator** — that is a design condition on
//! §5.2(a), not an implementation detail.
//!
//! # What the write barrier is
//!
//! There is no instruction-level barrier: reads and writes are plain loads and stores at
//! the same cost as `plain`. The cost is paid as a kernel trap on the first store to
//! each page after a snapshot, because `mach_vm_remap(copy=TRUE)` leaves the *source*
//! entry needing a copy as well as the destination. So (a)'s barrier is not a percentage
//! on the hot loop; it is a burst of page faults immediately after each checkpoint.
//! [`KernelCow::fault_probe`] measures that directly.

use std::io;

use crate::backend::{Backend, Snapshot};
use crate::layout::{Layout, CHUNK_BYTES};
use crate::mach::*;

pub struct Region {
    pub base: mach_vm_address_t,
    pub bytes: mach_vm_size_t,
}

impl Region {
    pub fn allocate(bytes: usize) -> io::Result<Region> {
        let bytes = bytes as mach_vm_size_t;
        let mut base: mach_vm_address_t = 0;
        // SAFETY: base and bytes are valid for the call; ANYWHERE lets the kernel choose.
        let kr = unsafe { mach_vm_allocate(task_self(), &mut base, bytes, VM_FLAGS_ANYWHERE) };
        check("mach_vm_allocate", kr)?;
        Ok(Region { base, bytes })
    }

    #[inline(always)]
    pub fn as_ptr(&self) -> *const u64 {
        self.base as *const u64
    }
    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut u64 {
        self.base as *mut u64
    }
    pub fn words(&self) -> usize {
        (self.bytes / 8) as usize
    }
}

impl Drop for Region {
    fn drop(&mut self) {
        if self.base != 0 {
            // SAFETY: this region was allocated or remapped by us and is not aliased by
            // any live Rust reference at drop time.
            unsafe {
                let _ = mach_vm_deallocate(task_self(), self.base, self.bytes);
            }
        }
    }
}

/// A `mach_vm_remap(copy=TRUE)` snapshot.
pub struct RemapSnap {
    region: Region,
    /// Resident bytes attributable to this snapshot, filled in by the harness when it
    /// samples `resident_bytes()` around the snapshot's lifetime.
    pub measured_bytes: usize,
}

impl Snapshot for RemapSnap {
    fn nominal_bytes(&self) -> usize {
        self.measured_bytes
    }
}

pub struct KernelCow {
    live: Region,
    /// Protections the kernel reported for the last remap, recorded because
    /// `mach_vm_remap`'s protection arguments behave differently across XNU versions and
    /// a restore that silently drops `VM_PROT_WRITE` would fault forever.
    pub last_cur_prot: vm_prot_t,
    pub last_max_prot: vm_prot_t,
}

impl KernelCow {
    pub fn try_new(layout: &Layout) -> io::Result<KernelCow> {
        let live = Region::allocate(layout.total_bytes())?;
        Ok(KernelCow {
            live,
            last_cur_prot: 0,
            last_max_prot: 0,
        })
    }

    /// `mach_vm_remap(copy=TRUE)` of the live state to a fresh address.
    pub fn try_snapshot(&mut self) -> io::Result<RemapSnap> {
        let mut dst: mach_vm_address_t = 0;
        let mut cur: vm_prot_t = VM_PROT_READ | VM_PROT_WRITE;
        let mut max: vm_prot_t = VM_PROT_READ | VM_PROT_WRITE;
        // SAFETY: all pointers are to locals; src is our own live region.
        let kr = unsafe {
            mach_vm_remap(
                task_self(),
                &mut dst,
                self.live.bytes,
                0,
                VM_FLAGS_ANYWHERE,
                task_self(),
                self.live.base,
                1, // copy = TRUE: a copy-on-write copy, not a shared mapping
                &mut cur,
                &mut max,
                VM_INHERIT_NONE,
            )
        };
        check("mach_vm_remap(snapshot)", kr)?;
        self.last_cur_prot = cur;
        self.last_max_prot = max;
        Ok(RemapSnap {
            region: Region {
                base: dst,
                bytes: self.live.bytes,
            },
            measured_bytes: 0,
        })
    }

    /// Restore by remapping the snapshot back over the live range.
    ///
    /// `VM_FLAGS_OVERWRITE` replaces the live mapping in place, so every pointer the
    /// engine holds into the arenas stays valid — which is the property that makes (a)
    /// attractive for an engine full of raw arena indices.
    pub fn try_restore(&mut self, snap: &RemapSnap) -> io::Result<()> {
        let mut dst: mach_vm_address_t = self.live.base;
        let mut cur: vm_prot_t = VM_PROT_READ | VM_PROT_WRITE;
        let mut max: vm_prot_t = VM_PROT_READ | VM_PROT_WRITE;
        // SAFETY: dst is our own live region, which we are deliberately replacing; no
        // live Rust reference into it outlives this call.
        let kr = unsafe {
            mach_vm_remap(
                task_self(),
                &mut dst,
                self.live.bytes,
                0,
                VM_FLAGS_FIXED | VM_FLAGS_OVERWRITE,
                task_self(),
                snap.region.base,
                1, // copy = TRUE, so the snapshot survives the restore and can be reused
                &mut cur,
                &mut max,
                VM_INHERIT_NONE,
            )
        };
        check("mach_vm_remap(restore)", kr)?;
        debug_assert_eq!(dst, self.live.base, "OVERWRITE remap moved the live range");
        self.last_cur_prot = cur;
        self.last_max_prot = max;
        Ok(())
    }

    /// `vm_copy` of the live state into an already-allocated region: the other kernel
    /// primitive §5.2(a) names. Unlike remap it does not create a new mapping, so the
    /// destination address is stable, but it is a copy *request* — the kernel is free to
    /// implement it lazily or eagerly.
    pub fn try_vm_copy_into(&self, dst: &Region) -> io::Result<()> {
        assert_eq!(dst.bytes, self.live.bytes);
        // SAFETY: both addresses and the size are page-aligned regions we own.
        let kr = unsafe {
            vm_copy(
                task_self(),
                self.live.base as vm_address_t,
                self.live.bytes as vm_size_t,
                dst.base as vm_address_t,
            )
        };
        check("vm_copy", kr)
    }

    /// Store one word into each of the first `pages` pages. Used to price the copy fault
    /// the kernel takes on the first write to a page after a snapshot: run it once with a
    /// snapshot outstanding and once without, and subtract.
    pub fn fault_probe(&mut self, pages: usize, stamp: u64) {
        let p = self.live.as_mut_ptr();
        let stride = CHUNK_BYTES / 8;
        let n = self.live.words();
        for k in 0..pages {
            let off = k * stride;
            if off >= n {
                break;
            }
            // SAFETY: off < n, and the region is mapped read/write.
            unsafe { std::ptr::write_volatile(p.add(off), stamp) };
        }
    }
}

impl Backend for KernelCow {
    type Snap = RemapSnap;

    fn name(&self) -> &'static str {
        "kernel-remap"
    }

    fn new(layout: &Layout) -> KernelCow {
        KernelCow::try_new(layout).expect("mach_vm_allocate for the engine state")
    }

    #[inline(always)]
    fn get(&self, i: usize) -> u64 {
        debug_assert!(i < self.live.words());
        // SAFETY: i is in bounds of the mapped region.
        unsafe { *self.live.as_ptr().add(i) }
    }

    #[inline(always)]
    fn set(&mut self, i: usize, v: u64) {
        debug_assert!(i < self.live.words());
        // SAFETY: i is in bounds of the mapped region, which is read/write.
        unsafe { *self.live.as_mut_ptr().add(i) = v }
    }

    fn snapshot(&mut self) -> RemapSnap {
        self.try_snapshot().expect("mach_vm_remap(copy=TRUE)")
    }

    fn restore(&mut self, snap: &RemapSnap) {
        self.try_restore(snap).expect("mach_vm_remap(OVERWRITE)")
    }

    fn words(&self) -> usize {
        self.live.words()
    }
}
