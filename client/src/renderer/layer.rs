use aeonetica_engine::Id;
use aeonetica_engine::math::camera::Camera;
use aeonetica_engine::time::Time;
use aeonetica_engine::util::id_map::IdMap;

use crate::client_runtime::ClientHandleBox;
use crate::data_store::DataStore;
use crate::renderer::Renderer;
use crate::renderer::window::events::Event;

#[allow(unused_variables)]
pub trait Layer {
    fn instantiate_camera(&self) -> Camera;

    fn attach(&mut self, renderer: &mut Renderer, store: &mut DataStore) {} // run on layer creation
    fn quit(&mut self, renderer: &mut Renderer, store: &mut DataStore) {} // run on layer deletion

    fn update_camera(&mut self, store: &mut DataStore, camera: &mut Camera, time: Time) {}
    fn pre_handles_update(&mut self, store: &mut DataStore, renderer: &mut Renderer, time: Time) {}
    fn post_handles_update(&mut self, store: &mut DataStore, renderer: &mut Renderer, time: Time) {}

    fn event(&mut self, event: &Event, store: &mut DataStore) -> bool { false } // run on window event

    fn active(&self) -> bool { true }
    fn name(&self) -> &'static str { "Layer" }
    fn is_overlay(&self) -> bool { false }
}

pub struct LayerUpdater<'a> {
    layer: &'a mut Box<dyn Layer>,
    handles: &'a mut IdMap<ClientHandleBox>,
    client_id: Id,
    store: &'a mut DataStore
}

impl<'a> LayerUpdater<'a> {
    #[inline(always)]
    pub(crate) fn new(layer: &'a mut Box<dyn Layer>, handles: &'a mut IdMap<ClientHandleBox>, client_id: Id, store: &'a mut DataStore) -> Self {
        Self {
            layer,
            handles,
            client_id,
            store
        }
    }

    #[inline(always)]
    pub fn update(&mut self, renderer: &mut Renderer, time: Time) {
        self.layer.pre_handles_update(self.store, renderer, time);
        self.handles.iter_mut()
            .filter(|(_id, handle_box)| handle_box.handle.owning_layer() == self.client_id)
            .for_each(|(_id, handle_box)| handle_box.handle.update(&mut handle_box.messenger, renderer, self.store, time));
        self.layer.post_handles_update(self.store, renderer, time);
    }

    pub fn store(&mut self) -> &mut &'a mut DataStore {
        &mut self.store
    }
}