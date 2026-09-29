//! Defines `has_kernel_cow` when mechanism (a) can be built: on macOS, unless
//! `SNAPSHOT_BENCH_NO_KERNEL` is set. Setting it on a Mac compiles exactly the code path a
//! Linux build takes, which is how that path is checked without a Linux toolchain.

fn main() {
    println!("cargo::rustc-check-cfg=cfg(has_kernel_cow)");
    println!("cargo::rerun-if-env-changed=SNAPSHOT_BENCH_NO_KERNEL");
    let macos = std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos");
    let disabled = std::env::var_os("SNAPSHOT_BENCH_NO_KERNEL").is_some();
    if macos && !disabled {
        println!("cargo::rustc-cfg=has_kernel_cow");
    }
}
