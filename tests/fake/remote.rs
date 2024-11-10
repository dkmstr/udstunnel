use std::sync::{Arc, Mutex};

struct Remote {
    data: Arc<Mutex<Vec<String>>>,
}