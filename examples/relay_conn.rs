use tokio::{io::{self, AsyncWriteExt}, net::{TcpListener, TcpStream}, task};

#[tokio::main]
async fn main() -> io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:3389").await?;

    loop {
        let (socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let (reader, mut writer) = socket.into_split();

            let relay = TcpStream::connect("dc.dkmon.local:3389").await.unwrap();
            let (relay_reader, mut relay_writer) = relay.into_split();

            let read_task = task::spawn(async move {
                // Using a buf on heap so transfer between tasks is faster
                // Only need to transfer the pointer, not the data as in the case of [u8; 1024]
                let mut buf = vec![0; 1024];
                loop {
                    let _ = reader.readable().await.unwrap();
                    match reader.try_read(&mut buf) {
                        Ok(0) => {
                            break;
                        }
                        Ok(n) => {
                            relay_writer.write_all(&buf[..n]).await.unwrap();
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => {  // Last one, move value
                            println!("Error reading from relay: {:?}", e);
                            break;
                        }
                    }
                }
            });

            let write_task = task::spawn(async move {
                // Using a buf on heap so transfer between tasks is faster
                // Only need to transfer the pointer, not the data as in the case of [u8; 1024]
                let mut buf = vec![0; 1024];
                loop {
                    relay_reader.readable().await.unwrap();
                    match relay_reader.try_read(&mut buf) {
                        Ok(0) => {
                            break;
                        }
                        Ok(n) => {
                            writer.write_all(&buf[..n]).await.unwrap();
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) => {  // Last one, move value
                            println!("Error reading from relay: {:?}", e);
                            break;
                        }
                    }
                }
            });

            // As soon as one of the tasks completes, the other task will be dropped
            // and the connection will be closed.
            tokio::select! {
                res = read_task => {
                    println!("Read task completed: {:?}", res);
                }
                res = write_task => {
                    println!("Write task completed: {:?}", res);
                }
            }
        });
    }
}
