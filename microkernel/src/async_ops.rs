use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use polyhal::time::Time;

pub struct NextTime(pub usize);

impl Future for NextTime {
    type Output = ();

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Time::now().to_msec() >= self.0 {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

#[inline]
pub async fn sleep(ms: usize) {
    NextTime(Time::now().to_msec() + ms).await;
}
