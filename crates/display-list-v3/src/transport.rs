//! The reliable byte stream display-list-v3 runs over (spec §6.1;
//! DESIGN.md §16 rule 1): a stream socket bound to a file-system path.
//!
//! * Unix: `AF_UNIX` through the standard library ([`Stream`] and
//!   [`Listener`] are `UnixStream` and `UnixListener`).
//! * Windows 10 1803 and later: `AF_UNIX` stream sockets through Winsock,
//!   with the same path semantics (no `socketpair`, no descriptor passing;
//!   Microsoft, "AF_UNIX comes to Windows"). [`Stream`] wraps the socket in
//!   a `std::net::TcpStream`, whose reads, writes, timeouts, `try_clone`
//!   and `shutdown` are plain Winsock calls that work on any stream socket.
//!
//! Every OS call of this crate's transport lives here; an unknown target
//! is a compile error (DESIGN.md §16 rule 2), except WASI, which has no
//! sockets to bind: there the types exist and every call fails, so the
//! format code (frames, pages, resources) still builds.

#[cfg(not(any(unix, windows, target_os = "wasi")))]
compile_error!("display-list-v3 transport: add this OS to src/transport.rs");

#[cfg(unix)]
pub use std::os::unix::net::{UnixListener as Listener, UnixStream as Stream};

#[cfg(windows)]
pub use windows::{Listener, Stream};

#[cfg(target_os = "wasi")]
pub use wasi::{Listener, Stream};

/// Widen a stream's send and receive buffers to 4 MiB. macOS gives a Unix
/// stream socket 8 KiB each way by default, which splits a font program or
/// a large page into hundreds of reads and writes; both ends of every
/// display-list connection call this. Errors are ignored (the default
/// buffers still work).
pub fn widen_socket_buffers(s: &Stream) {
    #[cfg(unix)]
    {
        use std::os::fd::AsRawFd;
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        const SOL_SOCKET: i32 = 0xffff;
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        const SO_SNDBUF: i32 = 0x1001;
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        const SO_RCVBUF: i32 = 0x1002;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const SOL_SOCKET: i32 = 1;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const SO_SNDBUF: i32 = 7;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const SO_RCVBUF: i32 = 8;
        #[cfg(not(any(
            target_os = "macos",
            target_os = "ios",
            target_os = "linux",
            target_os = "android"
        )))]
        compile_error!("display-list-v3 transport: add this Unix's SOL_SOCKET/SO_SNDBUF/SO_RCVBUF");
        extern "C" {
            fn setsockopt(
                fd: i32,
                level: i32,
                name: i32,
                value: *const std::ffi::c_void,
                len: u32,
            ) -> i32;
        }
        let size: i32 = 4 << 20;
        for opt in [SO_SNDBUF, SO_RCVBUF] {
            // SAFETY: a valid descriptor and a 4-byte int option value.
            unsafe {
                setsockopt(
                    s.as_raw_fd(),
                    SOL_SOCKET,
                    opt,
                    &size as *const i32 as *const _,
                    4,
                );
            }
        }
    }
    #[cfg(windows)]
    windows::widen(s);
    #[cfg(target_os = "wasi")]
    let _ = s;
}

#[cfg(windows)]
mod windows {
    use std::io::{self, Read, Write};
    use std::net::{Shutdown, TcpStream};
    use std::os::windows::io::{AsRawSocket, FromRawSocket, RawSocket};
    use std::path::Path;
    use std::sync::Once;
    use std::time::Duration;

    type Socket = usize;
    const INVALID_SOCKET: Socket = !0;
    const AF_UNIX: u16 = 1;
    const SOCK_STREAM: i32 = 1;
    const SOL_SOCKET: i32 = 0xffff;
    const SO_SNDBUF: i32 = 0x1001;
    const SO_RCVBUF: i32 = 0x1002;
    const SOMAXCONN: i32 = 0x7fff_ffff;

    /// `struct sockaddr_un` (afunix.h): `UNIX_PATH_MAX` is 108.
    #[repr(C)]
    struct SockaddrUn {
        family: u16,
        path: [u8; 108],
    }

    #[link(name = "ws2_32")]
    extern "system" {
        fn WSAStartup(version: u16, data: *mut u8) -> i32;
        fn WSAGetLastError() -> i32;
        fn WSASocketW(
            af: i32,
            ty: i32,
            protocol: i32,
            info: *mut u8,
            group: u32,
            flags: u32,
        ) -> Socket;
        fn ioctlsocket(s: Socket, cmd: i32, arg: *mut u32) -> i32;
        fn bind(s: Socket, name: *const SockaddrUn, len: i32) -> i32;
        fn listen(s: Socket, backlog: i32) -> i32;
        fn accept(s: Socket, addr: *mut u8, len: *mut i32) -> Socket;
        fn connect(s: Socket, name: *const SockaddrUn, len: i32) -> i32;
        fn closesocket(s: Socket) -> i32;
        fn setsockopt(s: Socket, level: i32, name: i32, value: *const u8, len: i32) -> i32;
        fn WSAIoctl(
            s: Socket,
            code: u32,
            in_buf: *const u8,
            in_len: u32,
            out_buf: *mut u8,
            out_len: u32,
            returned: *mut u32,
            overlapped: *mut u8,
            completion: *mut u8,
        ) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn SetHandleInformation(h: usize, mask: u32, flags: u32) -> i32;
    }
    const WSA_FLAG_OVERLAPPED: u32 = 0x01;
    const WSA_FLAG_NO_HANDLE_INHERIT: u32 = 0x80;
    const HANDLE_FLAG_INHERIT: u32 = 0x01;
    const FIONBIO: i32 = 0x8004_667E_u32 as i32;
    /// afunix.h: `_WSAIOR(IOC_VENDOR, 256)`.
    const SIO_AF_UNIX_GETPEERPID: u32 = 0x5800_0100;
    const WSAEINVAL: i32 = 10022;
    const WSAEOPNOTSUPP: i32 = 10045;

    fn last_error() -> io::Error {
        // SAFETY: no preconditions.
        io::Error::from_raw_os_error(unsafe { WSAGetLastError() })
    }

    /// Winsock must be started before the first `socket` call; the
    /// standard library starts it only for its own sockets. WSAStartup is
    /// reference counted, so a second start is harmless.
    fn startup() {
        static START: Once = Once::new();
        START.call_once(|| {
            // WSADATA is 408 bytes on 64-bit Windows; 512 is room enough.
            let mut data = [0u8; 512];
            // SAFETY: `data` outlives the call and is larger than WSADATA.
            unsafe { WSAStartup(0x0202, data.as_mut_ptr()) };
        });
    }

    fn address(path: &Path) -> io::Result<(SockaddrUn, i32)> {
        let bytes = path
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket path is not UTF-8"))?
            .as_bytes();
        let mut a = SockaddrUn {
            family: AF_UNIX,
            path: [0; 108],
        };
        if bytes.len() >= a.path.len() || bytes.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "socket path longer than 107 bytes or containing NUL",
            ));
        }
        a.path[..bytes.len()].copy_from_slice(bytes);
        Ok((a, (2 + bytes.len() + 1) as i32))
    }

    /// A new socket, not inherited by child processes (the standard
    /// library's own sockets are not either): an engine child holding a
    /// client's connection would keep it open after the host closed it.
    fn new_socket() -> io::Result<Socket> {
        startup();
        // SAFETY: no preconditions.
        let s = unsafe {
            WSASocketW(
                AF_UNIX as i32,
                SOCK_STREAM,
                0,
                std::ptr::null_mut(),
                0,
                WSA_FLAG_OVERLAPPED | WSA_FLAG_NO_HANDLE_INHERIT,
            )
        };
        if s == INVALID_SOCKET {
            Err(last_error())
        } else {
            Ok(s)
        }
    }

    /// A connected `AF_UNIX` stream socket.
    #[derive(Debug)]
    pub struct Stream(TcpStream);

    impl Stream {
        pub fn connect<P: AsRef<Path>>(path: P) -> io::Result<Stream> {
            let (a, len) = address(path.as_ref())?;
            let s = new_socket()?;
            // SAFETY: `a` is a valid sockaddr_un of length `len`.
            if unsafe { connect(s, &a, len) } != 0 {
                let e = last_error();
                // SAFETY: `s` is ours and not used again.
                unsafe { closesocket(s) };
                return Err(e);
            }
            // SAFETY: `s` is a connected socket nothing else owns.
            Ok(Stream(unsafe {
                TcpStream::from_raw_socket(s as RawSocket)
            }))
        }
        pub fn try_clone(&self) -> io::Result<Stream> {
            self.0.try_clone().map(Stream)
        }
        pub fn shutdown(&self, how: Shutdown) -> io::Result<()> {
            self.0.shutdown(how)
        }
        pub fn set_read_timeout(&self, d: Option<Duration>) -> io::Result<()> {
            self.0.set_read_timeout(d)
        }
        pub fn set_write_timeout(&self, d: Option<Duration>) -> io::Result<()> {
            self.0.set_write_timeout(d)
        }
        pub fn set_nonblocking(&self, on: bool) -> io::Result<()> {
            self.0.set_nonblocking(on)
        }
        /// The process id of the other end (`SIO_AF_UNIX_GETPEERPID`,
        /// Windows 10 1803 and later). Where Windows does not know the
        /// control code, the error is `ErrorKind::Unsupported`: Winsock
        /// answers an unknown `WSAIoctl` code with `WSAEOPNOTSUPP` or, for
        /// a code outside its table, `WSAEINVAL` (the arguments here are
        /// otherwise valid). Any other failure keeps its own error.
        pub fn peer_pid(&self) -> io::Result<u32> {
            let mut pid = 0u32;
            let mut n = 0u32;
            // SAFETY: a 4-byte output buffer, no input, synchronous.
            let r = unsafe {
                WSAIoctl(
                    self.0.as_raw_socket() as Socket,
                    SIO_AF_UNIX_GETPEERPID,
                    std::ptr::null(),
                    0,
                    &mut pid as *mut u32 as *mut u8,
                    4,
                    &mut n,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            if r != 0 {
                let e = last_error();
                return Err(match e.raw_os_error() {
                    Some(WSAEOPNOTSUPP | WSAEINVAL) => io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("SIO_AF_UNIX_GETPEERPID: {e} (Windows before 10 1803)"),
                    ),
                    _ => e,
                });
            }
            Ok(pid)
        }
    }

    impl Read for Stream {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            self.0.read(b)
        }
    }
    impl Read for &Stream {
        fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
            (&self.0).read(b)
        }
    }
    impl Write for Stream {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            self.0.write(b)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Write for &Stream {
        fn write(&mut self, b: &[u8]) -> io::Result<usize> {
            (&self.0).write(b)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// A listening `AF_UNIX` stream socket bound to a path.
    #[derive(Debug)]
    pub struct Listener(Socket);

    impl Listener {
        pub fn bind<P: AsRef<Path>>(path: P) -> io::Result<Listener> {
            let (a, len) = address(path.as_ref())?;
            let s = new_socket()?;
            let l = Listener(s);
            // SAFETY: `a` is a valid sockaddr_un of length `len`.
            if unsafe { bind(s, &a, len) } != 0 || unsafe { listen(s, SOMAXCONN) } != 0 {
                return Err(last_error());
            }
            Ok(l)
        }
        /// The next connection; the peer's address is not reported.
        pub fn accept(&self) -> io::Result<(Stream, ())> {
            // SAFETY: a null address buffer asks for no peer address.
            let s = unsafe { accept(self.0, std::ptr::null_mut(), std::ptr::null_mut()) };
            if s == INVALID_SOCKET {
                return Err(last_error());
            }
            // SAFETY: `s` is a fresh socket; not inherited (see `new_socket`).
            unsafe { SetHandleInformation(s, HANDLE_FLAG_INHERIT, 0) };
            // SAFETY: `s` is a fresh connected socket nothing else owns.
            Ok((
                Stream(unsafe { TcpStream::from_raw_socket(s as RawSocket) }),
                (),
            ))
        }
        pub fn incoming(&self) -> impl Iterator<Item = io::Result<Stream>> + '_ {
            std::iter::repeat_with(move || self.accept().map(|(s, ())| s))
        }
        /// Whether `accept` returns `WouldBlock` instead of waiting. A
        /// socket it then accepts is non-blocking too (Winsock copies the
        /// mode); the caller makes it blocking again.
        pub fn set_nonblocking(&self, on: bool) -> io::Result<()> {
            let mut arg = on as u32;
            // SAFETY: FIONBIO takes a u32 flag.
            if unsafe { ioctlsocket(self.0, FIONBIO, &mut arg) } != 0 {
                return Err(last_error());
            }
            Ok(())
        }
    }

    impl Drop for Listener {
        fn drop(&mut self) {
            // SAFETY: the socket is ours and not used again.
            unsafe { closesocket(self.0) };
        }
    }

    pub(super) fn widen(s: &Stream) {
        let size: i32 = 4 << 20;
        for opt in [SO_SNDBUF, SO_RCVBUF] {
            // SAFETY: a valid socket and a 4-byte int option value.
            unsafe {
                setsockopt(
                    s.0.as_raw_socket() as Socket,
                    SOL_SOCKET,
                    opt,
                    &size as *const i32 as *const u8,
                    4,
                );
            }
        }
    }
}

#[cfg(target_os = "wasi")]
mod wasi {
    use std::io::{self, Read, Write};
    use std::net::Shutdown;
    use std::path::Path;
    use std::time::Duration;

    fn unsupported<T>() -> io::Result<T> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "display-list-v3 sockets are unavailable on WASI; write the stream to a file",
        ))
    }

    /// No value of this type is ever made: `connect` always fails.
    #[derive(Debug)]
    pub struct Stream(());

    impl Stream {
        pub fn connect<P: AsRef<Path>>(_path: P) -> io::Result<Stream> {
            unsupported()
        }
        pub fn try_clone(&self) -> io::Result<Stream> {
            unsupported()
        }
        pub fn shutdown(&self, _how: Shutdown) -> io::Result<()> {
            unsupported()
        }
        pub fn set_read_timeout(&self, _d: Option<Duration>) -> io::Result<()> {
            unsupported()
        }
        pub fn set_write_timeout(&self, _d: Option<Duration>) -> io::Result<()> {
            unsupported()
        }
        pub fn set_nonblocking(&self, _on: bool) -> io::Result<()> {
            unsupported()
        }
    }
    impl Read for Stream {
        fn read(&mut self, _b: &mut [u8]) -> io::Result<usize> {
            unsupported()
        }
    }
    impl Read for &Stream {
        fn read(&mut self, _b: &mut [u8]) -> io::Result<usize> {
            unsupported()
        }
    }
    impl Write for Stream {
        fn write(&mut self, _b: &[u8]) -> io::Result<usize> {
            unsupported()
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Write for &Stream {
        fn write(&mut self, _b: &[u8]) -> io::Result<usize> {
            unsupported()
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// No value of this type is ever made: `bind` always fails.
    #[derive(Debug)]
    pub struct Listener(());

    impl Listener {
        pub fn bind<P: AsRef<Path>>(_path: P) -> io::Result<Listener> {
            unsupported()
        }
        pub fn accept(&self) -> io::Result<(Stream, ())> {
            unsupported()
        }
        pub fn incoming(&self) -> impl Iterator<Item = io::Result<Stream>> + '_ {
            std::iter::once(unsupported())
        }
        pub fn set_nonblocking(&self, _on: bool) -> io::Result<()> {
            unsupported()
        }
    }
}
