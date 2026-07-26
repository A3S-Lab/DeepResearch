use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

/// Cooperative cancellation shared by one DeepResearch run and its ports.
///
/// The token is runtime-agnostic so the engine does not require a particular
/// async executor. Dropping a cancelled port future remains the port's signal
/// to stop any in-flight work.
#[derive(Clone, Debug, Default)]
pub struct DeepResearchCancellation {
    state: Arc<CancellationState>,
}

#[derive(Debug, Default)]
struct CancellationState {
    cancelled: AtomicBool,
    waiters: Mutex<Vec<Waker>>,
}

impl DeepResearchCancellation {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        if self.state.cancelled.swap(true, Ordering::AcqRel) {
            return;
        }
        let waiters = self
            .state
            .waiters
            .lock()
            .map(|mut waiters| std::mem::take(&mut *waiters))
            .unwrap_or_default();
        for waiter in waiters {
            waiter.wake();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::Acquire)
    }

    pub(crate) fn cancelled(&self) -> CancellationFuture {
        CancellationFuture {
            state: Arc::clone(&self.state),
        }
    }
}

pub(crate) struct CancellationFuture {
    state: Arc<CancellationState>,
}

impl Future for CancellationFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.state.cancelled.load(Ordering::Acquire) {
            return Poll::Ready(());
        }
        let Ok(mut waiters) = self.state.waiters.lock() else {
            // A poisoned cancellation mutex must fail closed.
            return Poll::Ready(());
        };
        if self.state.cancelled.load(Ordering::Acquire) {
            return Poll::Ready(());
        }
        if !waiters
            .iter()
            .any(|waiter| waiter.will_wake(context.waker()))
        {
            waiters.push(context.waker().clone());
        }
        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_is_idempotent_and_shared_by_clones() {
        let cancellation = DeepResearchCancellation::new();
        let clone = cancellation.clone();

        assert!(!clone.is_cancelled());
        cancellation.cancel();
        cancellation.cancel();

        assert!(clone.is_cancelled());
    }
}
