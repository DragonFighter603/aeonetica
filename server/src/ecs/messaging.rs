
use std::cell::RefCell;
use std::collections::{HashSet};
use std::collections::hash_set::Iter;
use std::fmt::Debug;
use std::rc::Rc;
use aeonetica_engine::{ClientId, EntityId, Id, TypeId};
use aeonetica_engine::nanoserde::{DeBin, SerBin};
use aeonetica_engine::networking::server_packets::{ServerMessage, ServerPacket};
use aeonetica_engine::util::type_to_id;
use crate::ecs::{Module, Engine};
use crate::networking::NetworkServer;
use aeonetica_engine::networking::messaging::ClientEntity;
use aeonetica_engine::networking::SendMode;
use aeonetica_engine::util::id_map::IdMap;
use aeonetica_engine::util::nullable::Nullable;

pub trait Message: SerBin + DeBin + Debug {}

/// # Safety
///
/// This trait is for internal use only.
pub unsafe trait ClientMessenger {}
/// # Safety
///
/// This trait is for internal use only.
pub unsafe trait Renderer {}
/// # Safety
///
/// This trait is for internal use only.
pub unsafe trait DataStore {}

pub struct Messenger {
    ns: Option<Rc<RefCell<NetworkServer>>>,
    handle_type: TypeId,
    entity_id: EntityId,
    pub(crate) receivers: HashSet<ClientId>,
    pub(crate) receiver_functions: IdMap<Box<dyn Fn(&EntityId, &mut Engine, &ClientId, &Vec<u8>)>>
}

impl Module for Messenger {
    fn start(id: &Id, engine: &mut Engine) where Self: Sized {
        let ns = engine.runtime.ns.clone();
        let module = engine.mut_module_of::<Self>(id).unwrap();
        module.entity_id = *id;
        module.ns = Some(ns)
    }
}

impl Messenger {
    pub fn new<H: ClientEntity + Sized + 'static>() -> Self {
        Self {
            ns: None,
            receivers: Default::default(),
            handle_type: type_to_id::<H>(),
            entity_id: Id::new(),
            receiver_functions: Default::default()
        }
    }

    pub fn register_receiver<F: Fn(&EntityId, &mut Engine, &ClientId, M) + 'static, M: SerBin + DeBin>(&mut self, f: F) {
        let m = move |id: &Id, engine: &mut Engine, sender: &ClientId, data: &Vec<u8>|
            f(id, engine, sender, M::deserialize_bin(data).unwrap());
        self.receiver_functions.insert(type_to_id::<F>(), Box::new(m));
    }

    pub fn unregister_receiver<F: Fn(&EntityId, &mut Engine, &ClientId, M) + 'static, M: SerBin + DeBin>(&mut self, _: F) {
        self.receiver_functions.remove(&type_to_id::<F>());
    }

    pub fn call_client_fn<F: Fn(&mut T, &mut TClientMessenger, Nullable<&mut TRenderer>, &mut TDataStore, M), T: ClientEntity, TClientMessenger: ClientMessenger, TRenderer: Renderer, TDataStore: DataStore, M: SerBin + DeBin>(&mut self, _: F, message: M, mode: SendMode) {
        let id = type_to_id::<F>();
        for client in &self.receivers {
            let _ = self.ns.as_ref().unwrap().borrow().send(client, &ServerPacket {
                conv_id: Id::new(),
                message: ServerMessage::ModMessage(self.entity_id, id, message.serialize_bin()),
            }, mode);
        }
    }

    pub fn call_client_fn_for<F: Fn(&mut T, &mut TClientMessenger, Nullable<&mut TRenderer>, &mut TDataStore, M), T: ClientEntity, TClientMessenger: ClientMessenger, TRenderer: Renderer, TDataStore: DataStore, M: SerBin + DeBin>(&mut self, _: F, client: &ClientId, message: M, mode: SendMode) {
        let id = type_to_id::<F>();
        let _ = self.ns.as_ref().unwrap().borrow().send(client, &ServerPacket {
            conv_id: Id::new(),
            message: ServerMessage::ModMessage(self.entity_id, id, message.serialize_bin()),
        }, mode);
    }

    pub fn clients(&self) -> Iter<ClientId> {
        self.receivers.iter()
    }

    pub fn has_client(&self, id: &ClientId) -> bool {
        self.receivers.contains(id)
    }

    pub fn add_client(&mut self, id: ClientId) -> bool {
        if !self.receivers.contains(&id) && self.ns.as_ref().unwrap().borrow().clients.contains_key(&id) {
            self.receivers.insert(id);
            let _ = self.ns.as_ref().unwrap().borrow().send(&id, &ServerPacket {
                conv_id: Id::new(),
                message: ServerMessage::AddClientHandle(self.entity_id, self.handle_type),
            }, SendMode::Safe);
            true
        } else { false }
    }

    pub fn remove_client(&mut self, id: &ClientId) -> bool {
        if self.receivers.contains(id) {
            self.receivers.remove(id);
            let _ = self.ns.as_ref().unwrap().borrow().send(id, &ServerPacket {
                conv_id: Id::new(),
                message: ServerMessage::RemoveClientHandle(self.entity_id),
            }, SendMode::Safe);
            true
        } else { false }
    }
}