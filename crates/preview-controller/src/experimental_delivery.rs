//! Opt-in scheduling prototype, not connected to stdio. Required frames retain FIFO order.
//! Priority applies before dequeue; a writer cannot preempt an already-started JSONL frame.
use std::collections::VecDeque;

#[derive(Debug, PartialEq, Eq)]
pub enum Kind {
    Durable,
    Current { generation: u64 },
    Historical { generation: u64, origin: String },
}
#[derive(Debug)]
pub struct Frame {
    session: String,
    project: String,
    epoch: u64,
    kind: Kind,
    bytes: Vec<u8>,
}
impl Frame {
    /// Capture epoch and origin with the original work, not at delayed completion time.
    pub fn new(session: String, project: String, epoch: u64, kind: Kind, bytes: Vec<u8>) -> Self {
        Self {
            session,
            project,
            epoch,
            kind,
            bytes,
        }
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn kind(&self) -> &Kind {
        &self.kind
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    Closed,
    WrongIdentity,
    InvalidFrame,
    Capacity,
    Stale,
}
#[derive(Debug)]
pub struct NotAdmitted {
    pub reason: Rejection,
    /// Ownership returned so a required response is never silently dropped on backpressure.
    pub frame: Frame,
}
#[derive(Debug, PartialEq, Eq)]
pub enum Admission {
    Required,
    Historical,
    ReplacedHistorical,
}

pub struct DeliveryQueue {
    session: String,
    project: String,
    epoch: u64,
    current_floor: u64,
    closed: bool,
    required: VecDeque<Frame>,
    historical: Option<Frame>,
    required_bytes: usize,
    slots: usize,
    byte_budget: usize,
    frame_limit: usize,
}
fn identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && !value.chars().any(char::is_control)
}
impl DeliveryQueue {
    /// Required payload capacity is reserved independently of one optional frame.
    /// Budget counts Vec capacity, not just length; allocator overhead is not a process-RSS cap.
    pub fn new(
        session: String,
        project: String,
        slots: usize,
        byte_budget: usize,
        frame_limit: usize,
    ) -> Result<Self, &'static str> {
        if !identity(&session)
            || !identity(&project)
            || session.capacity() > 128
            || project.capacity() > 128
            || !(1..=64).contains(&slots)
            || frame_limit == 0
            || frame_limit > 16 * 1024 * 1024
            || byte_budget < frame_limit
            || byte_budget > 256 * 1024 * 1024
        {
            return Err("invalid delivery queue limits or identity");
        }
        Ok(Self {
            session,
            project,
            epoch: 0,
            current_floor: 0,
            closed: false,
            required: VecDeque::new(),
            historical: None,
            required_bytes: 0,
            slots,
            byte_budget,
            frame_limit,
        })
    }
    pub fn display_epoch(&self) -> u64 {
        self.epoch
    }
    pub fn required_residency(&self) -> (usize, usize) {
        (self.required.len(), self.required_bytes)
    }
    pub fn historical_capacity(&self) -> usize {
        self.historical
            .as_ref()
            .map_or(0, |frame| frame.bytes.capacity())
    }
    pub fn admit(&mut self, frame: Frame) -> Result<Admission, NotAdmitted> {
        let reason = if self.closed {
            Some(Rejection::Closed)
        } else if frame.session != self.session || frame.project != self.project {
            Some(Rejection::WrongIdentity)
        } else if frame.bytes.last() != Some(&b'\n')
            || frame.bytes.capacity() > self.frame_limit
            || frame.session.capacity() > 128
            || frame.project.capacity() > 128
        {
            Some(Rejection::InvalidFrame)
        } else {
            match &frame.kind {
                Kind::Durable => None,
                Kind::Current { generation } | Kind::Historical { generation, .. }
                    if frame.epoch != self.epoch || *generation <= self.current_floor =>
                {
                    Some(Rejection::Stale)
                }
                Kind::Historical { origin, .. }
                    if origin.is_empty()
                        || origin.capacity() > 1024
                        || origin.chars().any(char::is_control) =>
                {
                    Some(Rejection::InvalidFrame)
                }
                _ => None,
            }
        };
        if let Some(reason) = reason {
            return Err(NotAdmitted { reason, frame });
        }
        match &frame.kind {
            Kind::Historical { generation, .. } => {
                if self.historical.as_ref().is_some_and(|old| match old.kind {
                    Kind::Historical {
                        generation: old, ..
                    } => old >= *generation,
                    _ => false,
                }) {
                    return Err(NotAdmitted {
                        reason: Rejection::Stale,
                        frame,
                    });
                }
                let replaced = self.historical.replace(frame).is_some();
                Ok(if replaced {
                    Admission::ReplacedHistorical
                } else {
                    Admission::Historical
                })
            }
            Kind::Durable | Kind::Current { .. } => {
                if self.required.len() == self.slots
                    || frame.bytes.capacity() > self.byte_budget - self.required_bytes
                {
                    return Err(NotAdmitted {
                        reason: Rejection::Capacity,
                        frame,
                    });
                }
                if let Kind::Current { generation } = frame.kind {
                    self.current_floor = generation;
                    if self.historical.as_ref().is_some_and(|old| match old.kind {
                        Kind::Historical {
                            generation: old, ..
                        } => old <= generation,
                        _ => false,
                    }) {
                        self.historical = None;
                    }
                }
                self.required_bytes += frame.bytes.capacity();
                self.required.push_back(frame);
                Ok(Admission::Required)
            }
        }
    }
    /// Always service already-admitted required frames before optional historical work.
    pub fn next_frame(&mut self) -> Option<Frame> {
        if let Some(frame) = self.required.pop_front() {
            self.required_bytes -= frame.bytes.capacity();
            Some(frame)
        } else {
            self.historical.take()
        }
    }
    /// Cancel display work, including delayed old-epoch arrivals, while preserving durable ACKs.
    pub fn invalidate_display(&mut self) {
        if let Some(epoch) = self.epoch.checked_add(1) {
            self.epoch = epoch;
        } else {
            self.closed = true;
        }
        self.historical = None;
        self.required
            .retain(|frame| matches!(frame.kind, Kind::Durable));
        self.required_bytes = self
            .required
            .iter()
            .map(|frame| frame.bytes.capacity())
            .sum();
    }
    /// Stop admission and display. Already-admitted durable responses remain drainable.
    pub fn close(&mut self) {
        self.invalidate_display();
        self.closed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(epoch: u64, kind: Kind, label: &str) -> Frame {
        Frame::new(
            "s".into(),
            "p".into(),
            epoch,
            kind,
            format!("{label}\n").into_bytes(),
        )
    }
    fn historical(epoch: u64, generation: u64) -> Frame {
        frame(
            epoch,
            Kind::Historical {
                generation,
                origin: format!("source-{generation}"),
            },
            "history",
        )
    }
    #[test]
    fn optional_flood_cannot_consume_required_capacity_or_reorder_acks() {
        let mut queue = DeliveryQueue::new("s".into(), "p".into(), 2, 64, 32).unwrap();
        for generation in 1..100 {
            queue.admit(historical(0, generation)).unwrap();
        }
        assert_eq!(queue.required_residency(), (0, 0));
        queue.admit(frame(0, Kind::Durable, "ack-1")).unwrap();
        queue.admit(frame(0, Kind::Durable, "ack-2")).unwrap();
        assert_eq!(queue.next_frame().unwrap().bytes(), b"ack-1\n");
        assert_eq!(queue.next_frame().unwrap().bytes(), b"ack-2\n");
        assert_eq!(
            queue.next_frame().unwrap().kind(),
            &Kind::Historical {
                generation: 99,
                origin: "source-99".into()
            }
        );
        assert!(queue.next_frame().is_none());
    }
    #[test]
    fn required_backpressure_returns_exact_frame_and_current_supersedes_history() {
        let mut queue = DeliveryQueue::new("s".into(), "p".into(), 1, 32, 32).unwrap();
        queue.admit(frame(0, Kind::Durable, "ack")).unwrap();
        queue.admit(historical(0, 2)).unwrap();
        let rejected = queue
            .admit(frame(0, Kind::Current { generation: 3 }, "current"))
            .unwrap_err();
        assert_eq!(rejected.reason, Rejection::Capacity);
        assert_eq!(rejected.frame.bytes(), b"current\n");
        assert!(queue.historical_capacity() > 0);
        queue.next_frame().unwrap();
        queue.admit(rejected.frame).unwrap();
        assert_eq!(queue.historical_capacity(), 0);
        assert_eq!(queue.next_frame().unwrap().bytes(), b"current\n");
        assert_eq!(
            queue.admit(historical(0, 2)).unwrap_err().reason,
            Rejection::Stale
        );
    }
    #[test]
    fn cancellation_and_close_invalidate_late_display_but_drain_durable_acks() {
        let mut queue = DeliveryQueue::new("s".into(), "p".into(), 3, 96, 32).unwrap();
        queue.admit(frame(0, Kind::Durable, "ack")).unwrap();
        queue
            .admit(frame(0, Kind::Current { generation: 2 }, "current"))
            .unwrap();
        queue.admit(historical(0, 3)).unwrap();
        queue.invalidate_display();
        assert_eq!(
            queue.admit(historical(0, 4)).unwrap_err().reason,
            Rejection::Stale
        );
        queue.admit(historical(queue.display_epoch(), 4)).unwrap();
        queue.close();
        assert_eq!(queue.next_frame().unwrap().bytes(), b"ack\n");
        assert!(queue.next_frame().is_none());
        assert_eq!(
            queue.admit(historical(1, 5)).unwrap_err().reason,
            Rejection::Closed
        );
    }
    #[test]
    fn rejects_wrong_incarnation_and_charges_allocation_capacity() {
        let mut queue = DeliveryQueue::new("s".into(), "p".into(), 2, 32, 16).unwrap();
        let mut wrong = historical(0, 1);
        wrong.session = "old-session".into();
        assert_eq!(
            queue.admit(wrong).unwrap_err().reason,
            Rejection::WrongIdentity
        );
        let mut bytes = Vec::with_capacity(1024);
        bytes.push(b'\n');
        assert_eq!(
            queue
                .admit(Frame::new("s".into(), "p".into(), 0, Kind::Durable, bytes))
                .unwrap_err()
                .reason,
            Rejection::InvalidFrame
        );
        assert_eq!(queue.required_residency(), (0, 0));
    }
    #[test]
    fn byte_budget_returns_required_frame_and_metadata_capacity_is_bounded() {
        let mut queue = DeliveryQueue::new("s".into(), "p".into(), 4, 16, 16).unwrap();
        let mut bytes = vec![b'x'; 15];
        bytes[14] = b'\n';
        queue
            .admit(Frame::new("s".into(), "p".into(), 0, Kind::Durable, bytes))
            .unwrap();
        let rejected = queue.admit(frame(0, Kind::Durable, "a")).unwrap_err();
        assert_eq!(rejected.reason, Rejection::Capacity);
        assert_eq!(queue.required_residency(), (1, 15));
        queue.next_frame().unwrap();
        assert_eq!(queue.required_residency(), (0, 0));
        queue.admit(rejected.frame).unwrap();
        assert_eq!(queue.next_frame().unwrap().bytes(), b"a\n");
        let mut origin = String::with_capacity(4096);
        origin.push('x');
        assert_eq!(
            queue
                .admit(frame(
                    0,
                    Kind::Historical {
                        generation: 1,
                        origin
                    },
                    "h"
                ))
                .unwrap_err()
                .reason,
            Rejection::InvalidFrame
        );
        let mut session = String::with_capacity(4096);
        session.push('s');

        let oversized = Frame::new(session, "p".into(), 0, Kind::Durable, b"a\n".to_vec());
        assert_eq!(
            queue.admit(oversized).unwrap_err().reason,
            Rejection::InvalidFrame
        );
    }
}
