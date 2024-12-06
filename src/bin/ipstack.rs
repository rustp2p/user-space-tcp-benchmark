use ::ipstack::stream::IpStackStream;
use clap::Parser;
use std::net::SocketAddr;
use tokio::net::TcpStream;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    server: SocketAddr,
}

const MTU: u16 = 1500;

#[tokio::main]
async fn main() {
    env_logger::init();
    let Args { server } = Args::parse();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    ctrlc2::set_async_handler(async move {
        tx.send(())
            .await
            .expect("Could not send signal on channel.");
    })
    .await;

    let mut ipstack_config = ::ipstack::IpStackConfig::default();
    ipstack_config.mtu(MTU);
    ipstack_config.tcp_timeout(std::time::Duration::from_secs(5));
    ipstack_config.udp_timeout(std::time::Duration::from_secs(5));
    let mut config = tun::Configuration::default();
    config
        .address((10, 0, 0, 9))
        .netmask((255, 255, 255, 0))
        .mtu(MTU)
        .up();
    let dev = tun::create_as_async(&config).unwrap();
    let mut ip_stack = ipstack::IpStack::new(ipstack_config, dev);

    tokio::spawn(async move {
        loop {
            match ip_stack.accept().await.unwrap() {
                IpStackStream::Tcp(mut tcp) => {
                    let mut s = match TcpStream::connect(server).await {
                        Ok(s) => s,
                        Err(e) => {
                            log::info!("connect TCP server failed \"{}\"", e);
                            continue;
                        }
                    };
                    tokio::spawn(async move {
                        if let Err(err) = tokio::io::copy_bidirectional(&mut tcp, &mut s).await {
                            log::info!("TCP error: {}", err);
                        }
                    });
                }
                IpStackStream::Udp(_udp) => {}
                IpStackStream::UnknownTransport(_u) => {}
                IpStackStream::UnknownNetwork(pkt) => {
                    log::info!("unknown transport - {} bytes", pkt.len());
                    continue;
                }
            };
        }
    });

    rx.recv().await.expect("Could not receive from channel.");
    println!("Got it! Exiting...");
}
