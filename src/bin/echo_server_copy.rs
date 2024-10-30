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
                let mut buf = [0; 1024];
                loop {
                    let _ = reader.readable().await.unwrap();
                    let n = reader.try_read(&mut buf).unwrap_or_default();
                    if n == 0 {
                        break;
                    }
                    relay_writer.write_all(&buf[..n]).await.unwrap();
                }
            });

            let write_task = task::spawn(async move {
                let mut buf = [0; 1024];
                loop {
                    let _ = relay_reader.readable().await.unwrap();
                    let n = relay_reader.try_read(&mut buf).unwrap_or_default();
                    if n == 0 {
                        break;
                    }
                    writer.write_all(&buf[..n]).await.unwrap();
                }
            });

            read_task.await.unwrap();
            write_task.await.unwrap();
        });
    }
}
