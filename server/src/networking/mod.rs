
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};

use std::sync::{Arc, Mutex};
use std::time::Instant;
use aeonetica_engine::error::{AError, AET};
use aeonetica_engine::{Id, log_err};
use aeonetica_engine::nanoserde::{SerBin, DeBin};
use aeonetica_engine::networking::MAX_PACKET_SIZE;
use aeonetica_engine::networking::client_packets::ClientPacket;
use aeonetica_engine::networking::server_packets::ServerPacket;
use aeonetica_engine::util::id_map::IdMap;
use crate::server_runtime::ServerRuntime;

mod protocol;

pub(crate) struct NetworkServer {
    pub(crate) socket: UdpSocket,
    pub(crate) received: Arc<Mutex<Vec<(SocketAddr, ClientPacket)>>>,
    pub(crate) clients: IdMap<ClientHandle>
}

pub(crate) struct ClientHandle {
    pub(crate) last_seen: Instant,
    pub(crate) client_addr: SocketAddr,
    awaiting_replies: IdMap<Box<dyn Fn(&mut ServerRuntime, &ClientPacket)>>
}

impl NetworkServer {
    pub(crate) fn start(addr: &str) -> Result<Self, AError>{
        let socket = UdpSocket::bind(addr)?;
        let sock = socket.try_clone()?;
        let received = Arc::new(Mutex::new(vec![]));
        let recv = received.clone();
        std::thread::spawn(move || {
            let mut buf = [0u8; MAX_PACKET_SIZE];
            loop {
                match sock.recv_from(&mut buf) {
                    Ok((len, src)) => {
                        match DeBin::deserialize_bin(&buf[..len]) {
                            Ok(packet) => {
                                let trecv = recv.clone();
                                std::thread::spawn(move || {trecv.lock().unwrap().push((src, packet))});
                            },
                            Err(e) => log_err!("invalid client packet from {src}: {e}")
                        }
                    },
                    Err(_e) => {}
                }
            }
        });
        Ok(Self {
            socket,
            received,
            clients: Default::default(),
        })
    }

    pub(crate) fn queued_packets(&mut self) -> Vec<(SocketAddr, ClientPacket)> {
        let mut packets = vec![];
        std::mem::swap(&mut self.received.lock().unwrap() as &mut Vec<(SocketAddr, ClientPacket)>, &mut packets);
        packets
    }

    pub(crate) fn send(&self, client_id: &Id, packet: &ServerPacket) -> Result<(), AError>{
        self.clients.get(client_id).map(|client| {
            self.send_raw(client.client_addr, packet)
        }).unwrap_or(Err(AError::new(AET::NetworkError(format!("client {client_id} does not exist")))))?;

        Ok(())
    }

    pub(crate) fn send_raw(&self, ip_addr: SocketAddr, packet: &ServerPacket) -> Result<(), AError>{
        let data = SerBin::serialize_bin(packet);
        if data.len() > MAX_PACKET_SIZE {
            return Err(AError::new(AET::NetworkError(format!("Packet is too large: {} > {}", data.len(), MAX_PACKET_SIZE))))
        }
        let data = SerBin::serialize_bin(packet);
        let sock = self.socket.try_clone()?;
        std::thread::spawn(move || sock.send_to(&data[..], ip_addr).map_err(|e| {
            let e: AError = e.into();
            e.log();
        }));
        Ok(())
    }
}