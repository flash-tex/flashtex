//! Typed experimental envelope; retains the validated raw body without a Value tree.
use serde::Serialize;

pub fn envelope<'a>(
    session: &'a str,
    payload: &'a flashtex_preview_controller::RawDisplayPayload,
) -> impl Serialize + 'a {
    #[derive(Serialize)]
    struct Envelope<'a> {
        protocol_version: u8,
        session_id: &'a str,
        id: Option<()>,
        #[serde(rename = "type")]
        kind: &'static str,
        payload: &'a flashtex_preview_controller::RawDisplayPayload,
    }
    Envelope {
        protocol_version: 1,
        session_id: session,
        id: None,
        kind: "update",
        payload,
    }
}
