//! Stand-in for `kernel_cow` where mechanism (a) cannot exist (anything but macOS, or
//! `SNAPSHOT_BENCH_NO_KERNEL=1`). It has the same names so the harness type-checks, and
//! `SUPPORTED = false` so every call site skips it and reports "unsupported" instead.
//! Nothing here is ever constructed; the constructors say so if they are.

use std::io;

use crate::backend::{Backend, Snapshot};
use crate::layout::Layout;

pub const SUPPORTED: bool = false;

const WHY: &str = "kernel copy-on-write (mechanism (a)) needs macOS Mach VM calls";

pub struct Region;

impl Region {
    pub fn allocate(_bytes: usize) -> io::Result<Region> {
        Err(io::Error::other(WHY))
    }
}

pub struct RemapSnap;

impl Snapshot for RemapSnap {
    fn nominal_bytes(&self) -> usize {
        0
    }
}

pub struct KernelCow {
    pub last_cur_prot: i32,
    pub last_max_prot: i32,
}

impl KernelCow {
    pub fn try_snapshot(&mut self) -> io::Result<RemapSnap> {
        Err(io::Error::other(WHY))
    }
    pub fn try_restore(&mut self, _snap: &RemapSnap) -> io::Result<()> {
        Err(io::Error::other(WHY))
    }
    pub fn try_vm_copy_into(&self, _dst: &Region) -> io::Result<()> {
        Err(io::Error::other(WHY))
    }
    pub fn fault_probe(&mut self, _pages: usize, _stamp: u64) {
        unreachable!("{WHY}")
    }
}

impl Backend for KernelCow {
    type Snap = RemapSnap;
    fn name(&self) -> &'static str {
        "kernel-remap (unsupported)"
    }
    fn new(_layout: &Layout) -> KernelCow {
        panic!("{WHY}; check kernel_cow::SUPPORTED before constructing it")
    }
    fn get(&self, _i: usize) -> u64 {
        unreachable!("{WHY}")
    }
    fn set(&mut self, _i: usize, _v: u64) {
        unreachable!("{WHY}")
    }
    fn snapshot(&mut self) -> RemapSnap {
        unreachable!("{WHY}")
    }
    fn restore(&mut self, _snap: &RemapSnap) {
        unreachable!("{WHY}")
    }
    fn words(&self) -> usize {
        0
    }
}

/// Host free memory in bytes; unknown here.
pub fn host_free_bytes() -> u64 {
    0
}
