//! Asynchronous reads stay queued while a gesture owns a recoverable document checkpoint.
use std::cell::RefCell;

pub fn take_ready<T>(queue: &RefCell<Vec<T>>, interaction_busy: bool) -> Vec<T> {
    if interaction_busy { Vec::new() } else { std::mem::take(&mut *queue.borrow_mut()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pending_gesture_keeps_late_imports_and_their_order() {
        let queue = RefCell::new(vec!["first audio"]);
        assert!(take_ready(&queue, true).is_empty());
        queue.borrow_mut().push("second audio");
        assert!(take_ready(&queue, true).is_empty());
        assert_eq!(*queue.borrow(), ["first audio", "second audio"]);
        assert_eq!(take_ready(&queue, false), ["first audio", "second audio"]);
        assert!(take_ready(&queue, false).is_empty());
    }

    #[test]
    fn cancel_or_release_cannot_consume_a_different_queue_or_repeat_imports() {
        let projects = RefCell::new(vec![vec![1_u8, 2, 3]]);
        let samples = RefCell::new(vec![vec![4_u8, 5, 6]]);
        assert!(take_ready(&projects, true).is_empty());
        assert!(take_ready(&samples, true).is_empty());
        assert_eq!(take_ready(&projects, false), [vec![1, 2, 3]]);
        assert_eq!(*samples.borrow(), [vec![4, 5, 6]]);
        assert_eq!(take_ready(&samples, false), [vec![4, 5, 6]]);
        assert!(take_ready(&projects, false).is_empty());
        assert!(take_ready(&samples, false).is_empty());
    }
}
