use std::{future::Future, time::Duration};

use futures_timer::Delay;
use futures_util::{
    FutureExt,
    future::{Either, select},
};

use crate::{GenerationTaskCancellation, KernelError, KernelResult};

/// Cancellation wraps network waiting only. Durable output writes must finish.
pub(super) async fn request<F: Future>(
    future: F,
    cancellation: &dyn GenerationTaskCancellation,
) -> KernelResult<F::Output> {
    let future = future.fuse();
    futures_util::pin_mut!(future);
    loop {
        if cancellation.is_cancelled() {
            return Err(KernelError::GenerationCancelled);
        }
        let tick = Delay::new(Duration::from_millis(20)).fuse();
        futures_util::pin_mut!(tick);
        match select(&mut future, tick).await {
            Either::Left((result, _)) => return Ok(result),
            Either::Right(_) => {}
        }
    }
}
