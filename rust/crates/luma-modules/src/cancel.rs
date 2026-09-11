//! Helpers for honouring cancellation around awaitable side effects.

use tokio_util::sync::CancellationToken;

/// Race `fut` against `cancel`. Returns `None` if cancelled before or while waiting.
///
/// Cancel boundary: once `fut` has completed, the side effect is considered committed
/// even if the token flips afterwards (caller should still check before starting `fut`).
pub async fn await_unless_cancelled<T>(
    cancel: &CancellationToken,
    fut: impl std::future::Future<Output = T>,
) -> Option<T> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => None,
        value = fut => Some(value),
    }
}
