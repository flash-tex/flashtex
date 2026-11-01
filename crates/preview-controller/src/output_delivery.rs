//! Required FIFO plus one optional frame. An already-started write is not preemptible.
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
#[derive(Default)]
struct State {
    required: usize,
    epoch: u64,
    optional: Option<Frame>,
    clock: Option<Instant>,
    sequence: u64,
}
#[derive(Clone)]
pub struct Sender {
    tx: mpsc::SyncSender<Frame>,
    state: Arc<Mutex<State>>,
}
pub struct Receiver {
    rx: mpsc::Receiver<Frame>,
    state: Arc<Mutex<State>>,
}
#[derive(Debug)]
pub struct Frame {
    pub bytes: Vec<u8>,
    required: bool,
    generation: Option<u64>,
    trace: Option<(u64, Instant, f64)>,
}
#[cfg(test)]
pub fn channel(slots: usize) -> (Sender, Receiver) {
    channel_with_diagnostics(slots, false)
}
pub fn channel_with_diagnostics(slots: usize, enabled: bool) -> (Sender, Receiver) {
    let (tx, rx) = mpsc::sync_channel(slots);
    let state = Arc::new(Mutex::new(State {
        clock: enabled.then(Instant::now),
        ..State::default()
    }));
    (
        Sender {
            tx,
            state: state.clone(),
        },
        Receiver { rx, state },
    )
}
fn trace_record(
    trace: Option<(u64, Instant, f64)>,
    required: bool,
    bytes: usize,
    generation: Option<u64>,
    outcome: &str,
) {
    if let Some((sequence, clock, admitted_ms)) = trace {
        eprintln!(
            "{}",
            serde_json::json!({"phase":"output_frame","sequence":sequence,
            "compile_revision":generation,"class":if required {"required"} else {"optional"},"bytes":bytes,
            "admission_attempt_ms":admitted_ms,"at_ms":clock.elapsed().as_secs_f64()*1000.0,
            "outcome":outcome})
        );
    }
}
impl Frame {
    pub fn sequence(&self) -> Option<u64> {
        self.trace.map(|trace| trace.0)
    }
    pub fn trace(&self, outcome: &str) {
        trace_record(
            self.trace,
            self.required,
            self.bytes.len(),
            self.generation,
            outcome,
        );
    }
}
impl State {
    fn frame(&mut self, bytes: Vec<u8>, required: bool) -> Frame {
        let trace = self.clock.and_then(|clock| {
            self.sequence = self.sequence.checked_add(1)?;
            Some((self.sequence, clock, clock.elapsed().as_secs_f64() * 1000.0))
        });
        Frame {
            bytes,
            required,
            generation: None,
            trace,
        }
    }
}
impl Sender {
    pub fn diagnostic_ms(&self) -> Option<f64> {
        self.state
            .lock()
            .unwrap()
            .clock
            .map(|clock| clock.elapsed().as_secs_f64() * 1000.0)
    }
    pub fn try_send(&self, bytes: Vec<u8>) -> Result<(), mpsc::TrySendError<Vec<u8>>> {
        let mut state = self.state.lock().unwrap();
        let evicted = state.optional.take();
        let frame = state.frame(bytes, true);
        let trace = frame.trace;
        let length = frame.bytes.len();
        let outcome = self.tx.try_send(frame);
        if outcome.is_ok() {
            state.required += 1;
        }
        drop(state);
        if let Some(frame) = evicted {
            frame.trace("evicted_by_required");
        }
        match outcome {
            Ok(()) => {
                trace_record(trace, true, length, None, "admitted");
                Ok(())
            }
            Err(mpsc::TrySendError::Full(frame)) => {
                frame.trace("refused_full");
                Err(mpsc::TrySendError::Full(frame.bytes))
            }
            Err(mpsc::TrySendError::Disconnected(frame)) => {
                frame.trace("refused_disconnected");
                Err(mpsc::TrySendError::Disconnected(frame.bytes))
            }
        }
    }
    pub fn reset_optional(&self) -> u64 {
        let mut state = self.state.lock().unwrap();
        let evicted = state.optional.take();
        state.epoch = state.epoch.saturating_add(1);
        let epoch = state.epoch;
        drop(state);
        if let Some(frame) = evicted {
            frame.trace("evicted_by_reset");
        }
        epoch
    }
    pub fn can_offer(&self, epoch: u64) -> bool {
        let state = self.state.lock().unwrap();
        state.required == 0 && epoch == state.epoch && epoch != u64::MAX
    }
    #[cfg(test)]
    pub fn optional(&self, epoch: u64, bytes: Vec<u8>) -> bool {
        self.optional_with_generation(epoch, bytes, None)
    }
    pub fn optional_with_generation(
        &self,
        epoch: u64,
        bytes: Vec<u8>,
        generation: Option<u64>,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        let mut frame = state.frame(bytes, false);
        frame.generation = generation;
        if state.required != 0 || epoch != state.epoch || epoch == u64::MAX {
            drop(state);
            frame.trace("refused_busy_or_epoch");
            return false;
        }
        let trace = frame.trace;
        let length = frame.bytes.len();
        let evicted = state.optional.replace(frame);
        drop(state);
        if let Some(frame) = evicted {
            frame.trace("replaced_optional");
        }
        trace_record(trace, false, length, generation, "admitted");
        true
    }
}
impl Receiver {
    pub fn next(&self, timeout: Duration) -> Result<Frame, mpsc::RecvTimeoutError> {
        match self.rx.recv_timeout(timeout) {
            Ok(frame) => {
                frame.trace("dequeued");
                Ok(frame)
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let mut state = self.state.lock().unwrap();
                if state.required == 0 {
                    if let Some(frame) = state.optional.take() {
                        drop(state);
                        frame.trace("dequeued");
                        return Ok(frame);
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout)
            }
            Err(error) => Err(error),
        }
    }
    pub fn written(&self, frame: &Frame) {
        if frame.required {
            self.state.lock().unwrap().required -= 1;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagnostic_sequences_survive_replacement_refusal_and_inflight_writes() {
        let (tx, rx) = channel_with_diagnostics(1, true);
        assert!(tx.optional(0, b"old".to_vec()));
        let old = tx
            .state
            .lock()
            .unwrap()
            .optional
            .as_ref()
            .unwrap()
            .sequence()
            .unwrap();
        assert!(tx.optional_with_generation(0, b"new".to_vec(), Some(15)));
        let active = rx.next(Duration::ZERO).unwrap();
        assert!(active.sequence().unwrap() > old);
        tx.try_send(b"ack".to_vec()).unwrap();
        let error = tx.try_send(b"overflow".to_vec()).unwrap_err();
        assert!(matches!(error, mpsc::TrySendError::Full(bytes) if bytes == b"overflow"));
        // An optional write already dequeued is not replaced by required admission.
        assert_eq!(active.bytes, b"new");
        assert_eq!(active.generation, Some(15));
        active.trace("write_started");
        let ack = rx.next(Duration::ZERO).unwrap();
        assert!(ack.sequence().unwrap() > active.sequence().unwrap());
        assert!(!tx.can_offer(0));
        active.trace("write_failed");
        rx.written(&active);
        assert!(!tx.can_offer(0));
        rx.written(&ack);
        assert!(tx.can_offer(0));
    }
    #[test]
    fn disabled_diagnostics_allocate_no_identity_and_preserve_bytes() {
        let (tx, rx) = channel(1);
        tx.try_send(b"original".to_vec()).unwrap();
        let frame = rx.next(Duration::ZERO).unwrap();
        assert!(frame.trace.is_none());
        assert_eq!(frame.bytes, b"original");
        assert_eq!(tx.state.lock().unwrap().sequence, 0);
    }
    #[test]
    fn diagnostic_sequence_exhaustion_does_not_wrap_or_refuse_output() {
        let (tx, rx) = channel_with_diagnostics(1, true);
        tx.state.lock().unwrap().sequence = u64::MAX;
        tx.try_send(b"still delivered".to_vec()).unwrap();
        let frame = rx.next(Duration::ZERO).unwrap();
        assert!(frame.trace.is_none());
        assert_eq!(frame.bytes, b"still delivered");
    }
    #[test]
    fn required_pending_and_inflight_both_exclude_optional() {
        let (tx, rx) = channel(2);
        assert!(tx.optional(0, b"history\n".to_vec()));
        tx.try_send(b"ack\n".to_vec()).unwrap();
        assert!(!tx.optional(0, b"late\n".to_vec()));
        let ack = rx.next(Duration::ZERO).unwrap();
        assert_eq!(ack.bytes, b"ack\n");
        assert!(!tx.optional(0, b"while-writing\n".to_vec()));
        rx.written(&ack);
        assert!(rx.next(Duration::ZERO).is_err());
        assert!(tx.optional(0, b"new\n".to_vec()));
        assert_eq!(rx.next(Duration::ZERO).unwrap().bytes, b"new\n");
    }
    #[test]
    fn bounded_optional_replacement_reset_and_required_full_preserve_fifo() {
        let (tx, rx) = channel(1);
        for _ in 0..100 {
            assert!(tx.optional(0, b"history\n".to_vec()));
        }
        let epoch = tx.reset_optional();
        assert!(!tx.optional(0, b"old\n".to_vec()));
        assert!(rx.next(Duration::ZERO).is_err());
        assert!(tx.optional(epoch, b"new\n".to_vec()));
        tx.try_send(b"ack1\n".to_vec()).unwrap();
        let error = tx.try_send(b"ack2\n".to_vec()).unwrap_err();
        let mpsc::TrySendError::Full(bytes) = error else {
            panic!("expected full")
        };
        assert_eq!(bytes, b"ack2\n");
        let first = rx.next(Duration::ZERO).unwrap();
        rx.written(&first);
        tx.try_send(bytes).unwrap();
        let second = rx.next(Duration::ZERO).unwrap();
        assert_eq!(second.bytes, b"ack2\n");
        rx.written(&second);
        assert!(rx.next(Duration::ZERO).is_err());
    }
}
