use std::sync::atomic::AtomicU64;

#[derive(Debug)]
pub struct Stats {
    recv_bytes: AtomicU64,
    sent_bytes: AtomicU64,
    start_time: std::time::Instant,
    total_connections: AtomicU64,
    concurrent_connections: AtomicU64,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            recv_bytes: AtomicU64::new(0),
            sent_bytes: AtomicU64::new(0),
            start_time: std::time::Instant::now(),
            total_connections: AtomicU64::new(0),
            concurrent_connections: AtomicU64::new(0),
        }
    }

    pub fn get_recv_bytes(&self) -> u64 {
        self.recv_bytes.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_recv_bytes(&self, bytes: u64) {
        self.recv_bytes
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_sent_bytes(&self) -> u64 {
        self.sent_bytes.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_send_bytes(&self, bytes: u64) {
        self.sent_bytes
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_duration(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn get_globals_connections(&self) -> u64 {
        self.total_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_global_connection(&self) {
        self.total_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_concurrent_connections(&self) -> u64 {
        self.concurrent_connections
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_concurrent_connection(&self) {
        self.concurrent_connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn sub_concurrent_connection(&self) {
        self.concurrent_connections
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}
