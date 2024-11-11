use core::fmt;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{atomic::AtomicUsize, Arc, Mutex};
use std::task::{Context, Poll, Waker};

static EVENT_ID: AtomicUsize = AtomicUsize::new(0);

pub struct Event {
    state: Arc<Mutex<State>>,
    waker_id: usize,
}

struct State {
    value: bool,
    wakers: HashMap<usize, Waker>, // Usa Weak<Waker> en lugar de Waker
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("value", &self.value)
            .field("wakers", &self.wakers.len())
            .finish()
    }
}

impl Event {
    pub fn new() -> Self {
        Event {
            waker_id: EVENT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            state: Arc::new(Mutex::new(State {
                value: false,
                wakers: HashMap::new(),
            })),
        }
    }

    pub fn set(&self) {
        let mut state = self.state.lock().unwrap();
        state.value = true;

        // Will clean all wakers as soon as they are just used
        for waker in state.wakers.drain() {
            waker.1.wake();
        }
    }
}

impl Clone for Event {
    fn clone(&self) -> Self {
        Event {
            waker_id: EVENT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
            state: self.state.clone(),
        }
    }
}

impl Drop for Event {
    fn drop(&mut self) {
        let mut state = self.state.lock().unwrap();
        state.wakers.remove(&self.waker_id);
    }
}

impl Future for Event {
    type Output = bool;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.lock().unwrap();
        if state.value {
            Poll::Ready(true)
        } else {
            // Clean up the wakers that have been dropped
            // Add the current waker to the list
            state.wakers.insert(self.waker_id, cx.waker().clone());
            Poll::Pending
        }
    }
}

#[cfg(test)]
#[tokio::test]
async fn test_event_cleans_up_wakers() {
    let event = Event::new();
    let mut tasks = Vec::new();
    for _i in 0..4192 {
        let event_clone = event.clone();
        let task = tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                }
                _ = event_clone => {
                }
            }
        });
        tasks.push(task);
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    // As all events are out of scope, the wakers should be cleaned up
    assert!(event.state.lock().unwrap().wakers.is_empty());

    for task in tasks {
        task.await.unwrap_or_else(|e| {
            println!("Task failed: {:?}", e);
        });
    }
}

#[tokio::test]
async fn test_event_wakes_all_wakers() {
    let event = Event::new();
    let mut tasks = Vec::new();
    for _i in 0..4192 {
        let event_clone = event.clone();
        let task = tokio::spawn(async move {
            tokio::select! {
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                }
                _ = event_clone => {
                }
            }
        });
        tasks.push(task);
    }
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    event.set();
    for task in tasks {
        task.await.unwrap_or_else(|e| {
            println!("Task failed: {:?}", e);
        });
    }
    assert!(event.state.lock().unwrap().wakers.is_empty());
}
