use std::time::Instant;
use aeonetica_engine::*;
use aeonetica_engine::error::ErrorResult;
use aeonetica_engine::networking::client_packets::{ClientMessage, ClientPacket};
use aeonetica_engine::networking::SendMode;
use aeonetica_engine::time::Time;
use crate::client_runtime::ClientRuntime;
use crate::data_store::DataStore;
use crate::renderer::context::RenderContext;
use crate::renderer::window::Window;

const FULL_SEC: usize = 1_000_000_000;

pub fn run(mut client: ClientRuntime, client_id: ClientId, store: &mut DataStore) -> ErrorResult<()> {
    let _ = client.nc.borrow().send(&ClientPacket {
        client_id,
        conv_id: Id::new(),
        message: ClientMessage::Login,
    }, SendMode::Safe);

    log!("sent login");

    let mut window = Window::new(false)?;
    let mut time_nanos = 0;
    let mut frames = 0;
    let mut last_full_sec = 0;
    let mut time = Time {
        time: 0.0,
        delta: 0.0,
		raw_delta: 0.0
    };

    let mut context = RenderContext::new();

    client.loaded_mods.iter()
        .for_each(|loaded_mod| { loaded_mod.client_mod.start(store, window.context_provider().with_render(&mut context)); });

    while !window.should_close() {
        let t = Instant::now();

        window.poll_events(&mut client, &mut context, store);
        
        let _ = client.handle_queued(store, &mut context).map_err(|e| {
            log!(ERROR, "{e}")
        });
        
        window.on_render(&mut context, &mut client, store, time);
        
        let delta_time_nanos = t.elapsed().as_nanos();
        time_nanos += delta_time_nanos;
        time.raw_delta = delta_time_nanos as f32 / FULL_SEC as f32;
		time.delta = time.raw_delta.min(0.05);
        time.time = time_nanos as f32 / FULL_SEC as f32;
        
        frames += 1;

        if time_nanos - last_full_sec >= FULL_SEC as u128 {
            log!(PACK, "fps: {}", frames);
            last_full_sec = time_nanos;
            frames = 0;
        }
    }

    log!("shutting down client after {}s", time.time);
    context.finish(store);
    window.finish();
    client.nc.borrow().send(&ClientPacket {
        client_id: client.client_id,
        conv_id: Id::new(),
        message: ClientMessage::Logout,
    }, SendMode::Safe)
}
