use core::fmt;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{atomic::AtomicU64, Arc, Mutex};
use std::task::{Context, Poll, Waker};

static EVENT_ID: AtomicU64 = AtomicU64::new(0);

pub struct Event {
    state: Arc<Mutex<State>>,
    waker_id: u64, // Do not need sync, as it will be only written once at creation/clone
}

struct State {
    value: bool,
    wakers: HashMap<u64, Waker>,
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

    pub fn set(&self) -> Result<(), ()> {
        let awakers: Vec<Waker>;

        // To avoid deadlocks, we need to release the lock before waking up the wakers
        {
            let mut state = match self.state.lock() {
                Ok(state) => state,
                Err(_) => {
                    log::error!("Error locking the event state");
                    return Err(());
                }
            };
            // If already set, do nothing
            if state.value {
                return Ok(());
            }
            state.value = true;

            // Drain the wakers to be woken up to release the lock as soon as possible
            // And avoid calling the wakers while holding the lock
            awakers = state.wakers.drain().map(|(_, waker)| waker).collect();
        } // Release the lock, with the wakers to be woken up
        for waker in awakers {
            waker.wake();
        }
        Ok(())
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

// Drop the waker from the list when the event is dropped
// to ensure that the list do not grow indefinitely for not used events
impl Drop for Event {
    fn drop(&mut self) {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                log::error!("Error locking the event state");
                return; // The waker will not be removed from the list, but it is not a big deal because this should not happen
            }
        };
        // clean the waker from the list
        state.wakers.remove(&self.waker_id);
    }
}

impl Future for Event {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // In case of lock error, do our best by returning ready
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => {
                return Poll::Ready(());
            }
        };
        if state.value {
            Poll::Ready(())
        } else {
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
    // That is, after 1 seconds, the task will release the cloned event
    // So the wakers should be cleaned up after 2 seconds
    // Note that whe wakers array will be filled in the "await" call (poll)
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
    event.set().unwrap();
    for task in tasks {
        task.await.unwrap_or_else(|e| {
            println!("Task failed: {:?}", e);
        });
    }
    assert!(event.state.lock().unwrap().wakers.is_empty());
}
