//! Complete-frame optional admission. Never changes required source-delivery state.
use crate::{output_buffer, output_delivery::Sender};
use serde::Serialize;
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Admitted,
    BusyOrObsolete,
    SerializationRefused,
}
impl Outcome {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Admitted => "admitted",
            Self::BusyOrObsolete => "busy_or_obsolete",
            Self::SerializationRefused => "serialization_refused",
        }
    }
}
pub fn offer<T: Serialize + ?Sized>(
    sender: &Sender,
    epoch: u64,
    value: &T,
    limit: usize,
) -> Outcome {
    offer_with_generation(sender, epoch, value, limit, None)
}
pub fn offer_with_generation<T: Serialize + ?Sized>(
    sender: &Sender,
    epoch: u64,
    value: &T,
    limit: usize,
    generation: Option<u64>,
) -> Outcome {
    if !sender.can_offer(epoch) {
        return Outcome::BusyOrObsolete;
    }
    let Ok(bytes) = output_buffer::serialize(value, limit) else {
        return Outcome::SerializationRefused;
    };
    if sender.optional_with_generation(epoch, bytes, generation) {
        Outcome::Admitted
    } else {
        Outcome::BusyOrObsolete
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, value::RawValue, Value};
    use std::time::Duration;
    #[test]
    fn raw_body_preserves_spelling_and_complete_frame_limit_without_activation() {
        // Serializer boundary only: production must first validate source identity
        // and agree numeric/duplicate-key policy with the runtime owner.
        #[derive(Serialize)]
        struct Frame<'a> {
            kind: &'a str,
            body: &'a RawValue,
        }
        let raw = RawValue::from_string(format!(
            "{{\"number\":1e9,\"text\":\"{}\\u0061\"}}",
            "x".repeat(9000)
        ))
        .unwrap();
        let frame = Frame {
            kind: "probe",
            body: &raw,
        };
        let mut expected = serde_json::to_vec(&frame).unwrap();
        expected.push(b'\n');
        let (tx, rx) = crate::output_delivery::channel(1);
        assert_eq!(
            offer(&tx, 0, &frame, expected.len() - 1),
            Outcome::SerializationRefused
        );
        assert!(rx.next(Duration::ZERO).is_err());
        tx.try_send(b"durable-ack\n".to_vec()).unwrap();
        let ack = rx.next(Duration::ZERO).unwrap();
        assert_eq!(ack.bytes, b"durable-ack\n");
        rx.written(&ack);
        assert_eq!(offer(&tx, 0, &frame, expected.len()), Outcome::Admitted);
        assert_eq!(rx.next(Duration::ZERO).unwrap().bytes, expected);
        assert!(std::str::from_utf8(&expected).unwrap().contains("1e9"));
        assert!(std::str::from_utf8(&expected).unwrap().contains("\\u0061"));
    }
    #[test]
    fn reserialization_growth_is_refused_without_partial_output_or_lost_ack() {
        let (tx, rx) = crate::output_delivery::channel(2);
        // Valid compact numeric input can expand when the JSON Value is serialized.
        let raw = format!("{{\"stress\":[{}]}}", vec!["1e9"; 100].join(","));
        let value: Value = serde_json::from_str(&raw).unwrap();
        assert!(serde_json::to_vec(&value).unwrap().len() > raw.len());
        assert_eq!(
            offer(&tx, 0, &value, raw.len() + 1),
            Outcome::SerializationRefused
        );
        assert!(rx.next(Duration::ZERO).is_err());
        tx.try_send(b"{\"id\":\"durable-ack\"}\n".to_vec()).unwrap();
        assert_eq!(
            offer(&tx, 0, &json!({"candidate":1}), 100),
            Outcome::BusyOrObsolete
        );
        let ack = rx.next(Duration::ZERO).unwrap();
        assert_eq!(ack.bytes, b"{\"id\":\"durable-ack\"}\n");
        rx.written(&ack);
        assert_eq!(
            offer(&tx, 0, &json!({"candidate":2}), 100),
            Outcome::Admitted
        );
        let frame = rx.next(Duration::ZERO).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&frame.bytes).unwrap(),
            json!({"candidate":2})
        );
    }
    #[test]
    fn newline_budget_and_epoch_fence_are_applied_before_admission() {
        let (tx, rx) = crate::output_delivery::channel(1);
        let value = json!({"candidate":1});
        let len = serde_json::to_vec(&value).unwrap().len();
        assert_eq!(offer(&tx, 0, &value, len), Outcome::SerializationRefused);
        assert_eq!(offer(&tx, 0, &value, len + 1), Outcome::Admitted);
        let epoch = tx.reset_optional();
        assert!(rx.next(Duration::ZERO).is_err());
        assert_eq!(offer(&tx, 0, &value, len + 1), Outcome::BusyOrObsolete);
        assert_eq!(offer(&tx, epoch, &value, len + 1), Outcome::Admitted);
        assert_eq!(rx.next(Duration::ZERO).unwrap().bytes.len(), len + 1);
    }
}
