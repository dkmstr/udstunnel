
// Get a free por for the configuration, so we can run multiple tests
pub fn find_free_port(listen_address: Option<&str>) -> u16 {
    let listen_address = listen_address.unwrap_or("[::1]");
    match std::net::TcpListener::bind(format!("{}:0", listen_address)) {
        Ok(listener) => {
            let addr = listener.local_addr().unwrap();
            addr.port()
        }
        Err(e) => {
            panic!("Error binding listener: {:?}", e);
        }
    }
}
