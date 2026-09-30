//! `FLASHTEX_DISPLAY_LIST=socket:PATH` end to end: a reader listens on a
//! Unix-domain socket, the writer opens the parsed endpoint and sends
//! frames, and the reader gets them back byte for byte (spec §6.6).

#![cfg(unix)]

use flashtex_display_list::endpoint::Endpoint;
use flashtex_display_list::frame::{read_frame, write_frame};
use flashtex_display_list::kind;
use std::ffi::OsString;
use std::io::Write;
use std::os::unix::net::UnixListener;

#[test]
fn socket_endpoint_round_trips_frames() {
    let path = std::env::temp_dir().join(format!("dl3-endpoint-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let listener = UnixListener::bind(&path).unwrap();
    let reader = std::thread::spawn(move || {
        let (mut c, _) = listener.accept().unwrap();
        let mut got = Vec::new();
        while let Some(f) = read_frame(&mut c).unwrap() {
            got.push(f);
        }
        got
    });

    let mut spec = OsString::from("socket:");
    spec.push(&path);
    let ep = Endpoint::parse(&spec).unwrap();
    assert_eq!(ep, Endpoint::Socket(path.clone()));
    let big = vec![0xA5u8; 3 << 20]; // larger than any default socket buffer
    {
        let mut w = ep.open_writer().unwrap();
        write_frame(&mut w, kind::SOURCES, br#"{"files":[]}"#).unwrap();
        write_frame(&mut w, kind::PAGE, &big).unwrap();
        w.flush().unwrap();
    } // dropping the writer closes the socket: the reader sees end of stream
    let got = reader.join().unwrap();
    let _ = std::fs::remove_file(&path);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0], (kind::SOURCES, br#"{"files":[]}"#.to_vec()));
    assert_eq!(got[1].0, kind::PAGE);
    assert!(got[1].1 == big);
}

#[test]
fn socket_endpoint_without_a_listener_fails_cleanly() {
    let path = std::env::temp_dir().join(format!("dl3-endpoint-none-{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&path);
    assert!(Endpoint::Socket(path).open_writer().is_err());
}
