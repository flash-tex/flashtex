//! An engine can move to another thread: `Globals` is `Send`, as the pdfTeX
//! engine's word space (`Arena`) is. A field that is not `Send` (an `Rc`, a
//! raw pointer) fails to compile here.

fn assert_send<T: Send>() {}

#[test]
fn globals_is_send() {
    assert_send::<flashtex_xetex::Globals>();
    assert_send::<flashtex_xetex::state::Host>();
}
