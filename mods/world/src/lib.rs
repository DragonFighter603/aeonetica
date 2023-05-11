use aeonetica_engine::register;

pub mod client;
mod server;
mod common;

register!(client::WorldModClient{}, server::WorldModServer::new());