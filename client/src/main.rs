#![feature(let_chains)]
#![feature(result_flattening)]
#![feature(addr_parse_ascii)]

use std::net::{IpAddr, SocketAddr};

use aeonetica_engine::{log, Id, log_err};
use client::{client::run, data_store::DataStore, client_runtime::ClientRuntime};

mod defaults {
    pub(crate) const CLIENT_IP: &str = "127.0.0.1:9000";
    pub(crate) const SERVER_IP: &str = "127.0.0.1:6090";
}

fn main() {
    // nc -u 127.0.01 6090
    // cargo run -- 127.0.0.1:9000 127.0.0.1:6090
    let args: Vec<_> = std::env::args().skip(1).collect();
    log!("started client with args {args:?}");

    let mut client_ip = defaults::CLIENT_IP;
    let mut server_ip = defaults::SERVER_IP;

    match args.as_slice() {
        [a, ..] if a == "--help" => {
            log!("Usage: {} [<client ip>] [<server ip>] | --help", std::env::args().next().unwrap());
            return;
        }
        [c_ip, _] if SocketAddr::parse_ascii(c_ip.as_bytes()).is_err() => {
            log!("`{c_ip}` is not a valid IP address");
        }
        [_, s_ip] if SocketAddr::parse_ascii(s_ip.as_bytes()).is_err() => {
            log!("`{s_ip}` is not a valid IP address");
        }
        [c_ip, s_ip] => {
            client_ip = c_ip;
            server_ip = s_ip;
        }
        [] => {
            log!("using default arguments:\n\tclient ip: {client_ip}\tserver ip: {server_ip}");
        }
        _ => {
            log_err!("unexpected arguments: {args:?}; use `--help` for help");
            std::process::exit(2);
        }
    }
    
    let client_id = Id::new();
    
    let mut store = DataStore::new();
    let client = ClientRuntime::create(client_id, client_ip, server_ip, &mut store).map_err(|e| {
        e.log_exit();
    }).unwrap();

    run(client, client_id, &mut store)
}
