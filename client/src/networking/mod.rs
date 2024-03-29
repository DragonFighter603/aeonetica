use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use aeonetica_engine::error::{Error, Fatality, ErrorResult};
use aeonetica_engine::error::builtin::NetworkError;
use aeonetica_engine::{log};
use aeonetica_engine::nanoserde::{SerBin, DeBin};
use aeonetica_engine::networking::{MAX_PACKET_SIZE, SendMode};
use aeonetica_engine::networking::client_packets::{ClientPacket};
use aeonetica_engine::networking::server_packets::ServerPacket;

mod protocol;
pub mod messaging;

pub(crate) struct NetworkClient {
    pub(crate) udp: UdpSocket,
    pub(crate) tcp: RefCell<TcpStream>,
    received: Arc<Mutex<Vec<ServerPacket>>>
}

impl NetworkClient {
    pub(crate) fn start(addr: &str, server: &str) -> ErrorResult<Self>{
        let tcp = TcpStream::connect(server)?;
        tcp.set_nonblocking(false).unwrap();
        let udp = UdpSocket::bind(addr)?;
        udp.connect(server)?;
        let udp_sock = udp.try_clone()?;
        let mut tcp_sock = tcp.try_clone()?;
        let received = Arc::new(Mutex::new(vec![]));
        let recv_udp = received.clone();
        let recv_tcp = received.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; MAX_PACKET_SIZE];
            loop {
                match udp_sock.recv_from(&mut buf) {
                    Ok((len, src)) => match DeBin::deserialize_bin(&buf[..len]) {
                       Ok(packet) => recv_udp.lock().unwrap().push(packet),
                       Err(e) => log!(ERROR, "invalid server packet from {src}: {e}")
                    },
                    Err(e) => {
                        log!(ERROR, "couldn't recieve a datagram: {}", e);
                    }
                }
            }
        });
        std::thread::spawn(move || {
            loop {
                let mut size = [0u8;4];
                tcp_sock.read_exact(&mut size).unwrap();
                let size = u32::from_le_bytes(size);
                let mut buffer: Vec<u8> = vec![0;size as usize];
                tcp_sock.read_exact(&mut buffer[..]).unwrap();
                match DeBin::deserialize_bin(&buffer[..]) {
                    Ok(packet) => recv_tcp.lock().unwrap().push(packet),
                    Err(e) => log!(ERROR, "invalid server packet: {e}")
                }
            }
        });
        Ok(Self {
            udp,
            tcp: RefCell::new(tcp),
            received
        })
    }

    pub(crate) fn queued_packets(&mut self) -> Vec<ServerPacket> {
        let mut packets = vec![];
        std::mem::swap(&mut self.received.lock().unwrap() as &mut Vec<ServerPacket>, &mut packets);
        packets
    }

    pub(crate) fn send(&self, packet: &ClientPacket, mode: SendMode) -> ErrorResult<()> {
        let data = SerBin::serialize_bin(packet);
        match mode {
            SendMode::Quick => {
                if data.len() > MAX_PACKET_SIZE {
                    return Err(Error::new(NetworkError(format!("Packet is too large: {} > {}", data.len(), MAX_PACKET_SIZE)), Fatality::WARN, false))
                }
                let sock = self.udp.try_clone()?;
                std::thread::spawn(move || sock.send(&data[..]).map_err(|e| {
                    let e: Box<Error> = e.into();
                    e.log();
                }));
            }
            SendMode::Safe => {
                let mut tcp = self.tcp.borrow_mut();
                let _ = tcp.write_all(&(data.len() as u32).to_le_bytes()).map_err(|e| {
                    let e: Box<Error> = e.into();
                    e.log();
                });
                let _ = tcp.write_all(&data[..]).map_err(|e| {
                    let e: Box<Error> = e.into();
                    e.log();
                });
            }
        }
        Ok(())
    }
}