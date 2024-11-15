# UDS Tunnel Server

This project is an experimental Rust implementation of a OpenUDS tunnel server. 
For more info about uds-tunnel-server, please visit the [project page](https://github.com/VirtualCable/uds-tunnel-server)

# Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo (Rust package manager)

### Installation

1. Clone the repository:
    ```sh
    git clone https://github.com/yourusername/uds-tunnel-server.git
    cd uds-tunnel-server
    ```

2. Build the project:
    ```sh
    cargo build --release
    ```

### Configuration

There is a sample udstunnel.conf file in the root directory of the project. You can copy this file to /etc/udstunnel.conf and modify it according to your needs.
