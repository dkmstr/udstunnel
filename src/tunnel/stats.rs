use std::sync::atomic::AtomicU64;

#[derive(Debug)]
pub struct Stats {
    recv_bytes: AtomicU64,
    send_bytes: AtomicU64,
    start_time: std::time::Instant,
    connections: AtomicU64,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            recv_bytes: AtomicU64::new(0),
            send_bytes: AtomicU64::new(0),
            start_time: std::time::Instant::now(),
            connections: AtomicU64::new(0),
        }
    }

    pub fn get_recv_bytes(&self) -> u64 {
        self.recv_bytes.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_recv_bytes(&self, bytes: u64) {
        self.recv_bytes
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_send_bytes(&self) -> u64{
        self.send_bytes.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_send_bytes(&self, bytes: u64) {
        self.send_bytes
            .fetch_add(bytes, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_duration(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn get_connections(&self) -> u64 {
        self.connections.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn add_connection(&self) {
        self.connections
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}
