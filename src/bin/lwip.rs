use futures::{SinkExt, StreamExt};
use clap::Parser;
use std::net::SocketAddr;
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    server: SocketAddr,
}


#[tokio::main]
async fn main() {
    let Args{server} = Args::parse();
    env_logger::init();
    let (stack, mut tcp_listener, udp_socket) = ::lwip::NetStack::new().unwrap();
    let mut config = tun::Configuration::default();
    config
        .address((10, 0, 0, 9))
        .netmask((255, 255, 255, 0))
        .up();
    let mut dev = tun::create_as_async(&config)?;
    let framed = dev.into_framed();
    let (mut tun_sink, mut tun_stream) = framed.split();
    // Reads packet from stack and sends to TUN.
    tokio::spawn(async move {
        while let Some(pkt) = stack_stream.next().await {
            if let Ok(pkt) = pkt {
                tun_sink.send(pkt).await.unwrap();
            }
        }
    });
    // Reads packet from TUN and sends to stack.
    tokio::spawn(async move {
        while let Some(pkt) = tun_stream.next().await {
            if let Ok(pkt) = pkt {
                stack_sink.send(pkt).await.unwrap();
            }
        }
    });
    // Extracts TCP connections from stack and sends them to the dispatcher.
    tokio::spawn(async move {
        while let Some((mut stream, local_addr, remote_addr)) = tcp_listener.next().await {
            tokio::spawn(async move{
                let mut remote = tokio::net::TcpStream::connect(server).await.unwrap();
                if let Err(e) = tokio::io::copy_bidirectional(& mut stream, & mut remote).await{
                    log::error!("copy_bidirectional err: {e:?}");
                }
            });
        }
    });
}
