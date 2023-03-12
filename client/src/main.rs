use aeonetica_engine::error::{AError, AET};
use aeonetica_engine::log;
use client::client::run;

fn main() {
    // nc -u 127.0.01 6090
    // cargo run -- 127.0.0.1:9000 127.0.0.1:6090
    let args: Vec<_> = std::env::args().skip(1).collect();
    log!("started client with args {args:?}");
    if args.len() < 2 {
        let e = AError::new(AET::ValueError(format!("expected command line arg <local_ip:port> <server_ip:port>, got {}", args.len())));
        e.log_exit();
    }

    run(&args[0], &args[1])
}