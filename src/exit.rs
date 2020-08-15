// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use async_std::sync::Mutex;

use futures::channel::oneshot::{channel, Sender};
use futures::stream::{Stream, StreamExt as _};

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

struct Helper<T> {
    id: usize,
    inner: Arc<Mutex<Inner>>,
    stream: T,
}

impl<T> Drop for Helper<T> {
    fn drop(&mut self) {
        let inner = self.inner.clone();
        let id = self.id;

        async_std::task::spawn(async move {
            let mut locked = inner.lock().await;
            locked.senders.remove(&id);
        });
    }
}

impl<T> Stream for Helper<T>
where
    T: Stream,
{
    type Item = T::Item;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let projected = unsafe { self.map_unchecked_mut(|h| &mut h.stream) };

        projected.poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

#[derive(Debug)]
pub struct Inner {
    count: usize,
    senders: HashMap<usize, Sender<()>>,
}

#[derive(Debug, Clone)]
pub struct Exit {
    exited: Arc<AtomicBool>,
    inner: Arc<Mutex<Inner>>,
}

impl Default for Exit {
    fn default() -> Self {
        Self::new()
    }
}

impl Exit {
    pub fn new() -> Self {
        Self {
            exited: Default::default(),
            inner: Arc::new(Mutex::new(Inner {
                count: 0,
                senders: HashMap::new(),
            })),
        }
    }

    pub async fn exit(&self) {
        if self.exited.load(Ordering::SeqCst) {
            return;
        }

        let mut inner = self.inner.lock().await;

        if self.exited.fetch_or(true, Ordering::SeqCst) {
            return;
        }

        for (_, sender) in inner.senders.drain() {
            sender.send(()).ok();
        }

        // TODO: Unpark threads that used the interval method.
    }

    pub async fn from<S>(
        &self,
        stream: S,
    ) -> impl Stream<Item = S::Item> + Unpin
    where
        S: Stream + Unpin,
    {
        let id;
        let receiver;

        {
            let mut inner = self.inner.lock().await;

            if self.exited.load(Ordering::SeqCst) {
                drop(inner);
                todo!("exit already triggered");
            }

            let (sender, r) = channel();
            receiver = r;

            id = inner.count;
            inner.count += 1;

            inner.senders.insert(id, sender);
        };

        let out = stream.take_until(receiver);

        Helper {
            id,
            stream: out,
            inner: self.inner.clone(),
        }
    }

    pub fn interval(&self, duration: Duration) -> Interval {
        Interval {
            exited: self.exited.clone(),
            next: Instant::now() + duration,
            no_send: Default::default(),
            duration,
        }
    }
}

#[derive(Debug)]
pub struct Interval {
    next: Instant,
    duration: Duration,
    exited: Arc<AtomicBool>,

    no_send: std::marker::PhantomData<*const ()>,
}

impl Drop for Interval {
    fn drop(&mut self) {
        // TODO: Remove this interval's thread from inner's hashmap.
    }
}

impl Interval {
    pub fn next(&mut self) -> Option<()> {
        loop {
            let now = Instant::now();
            if self.exited.load(Ordering::SeqCst) {
                return None;
            } else if now >= self.next {
                self.next = now + self.duration;
                return Some(());
            } else {
                std::thread::park_timeout(self.next - now);
            }
        }
    }
}
