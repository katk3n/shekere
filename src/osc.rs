use async_std::channel::unbounded;
use async_std::channel::Receiver;
use async_std::net::{SocketAddrV4, UdpSocket};
use async_std::task;
use rosc::OscPacket;
use std::str::FromStr;

pub async fn osc_start() -> Receiver<OscPacket> {
    let addr = match SocketAddrV4::from_str("0.0.0.0:2020") {
        Ok(addr) => addr,
        Err(_) => panic!("Error"),
    };
    let sock = UdpSocket::bind(addr).await.unwrap();
    println!("Listening to {}", addr);
    let mut buf = [0u8; rosc::decoder::MTU];
    let (sender, receiver) = unbounded();
    task::spawn(async move {
        loop {
            let (size, addr) = sock.recv_from(&mut buf).await.unwrap();
            println!("Received packet with size {} from: {}", size, addr);
            let (_, packet) = rosc::decoder::decode_udp(&buf[..size]).unwrap();
            let _ = sender.send(packet).await;
        }
    });

    receiver
}

pub fn handle_packet(packet: OscPacket) {
    match packet {
        OscPacket::Message(msg) => {
            println!("OSC msg: {} {:?}", msg.addr, msg.args);
        }
        OscPacket::Bundle(bundle) => {
            println!("OSC Bundle: {:?}", bundle);
        }
    }
}
