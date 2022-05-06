use geng::net;
use geng::prelude::*;

mod lobby;

pub use lobby::*;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
#[cfg(not(target_arch = "wasm32"))]
pub use server::*;

pub trait Model: Diff + net::Message {
    type PlayerId: net::Message + Clone;
    type Message: net::Message;
    type Event: net::Message + Clone;
    const TICKS_PER_SECOND: f32;
    fn new_player(&mut self) -> Self::PlayerId;
    fn drop_player(&mut self, player_id: &Self::PlayerId);
    fn handle_message(&mut self, player_id: &Self::PlayerId, message: Self::Message);
    fn tick(&mut self, events: &mut Vec<Self::Event>);
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage<T: Model> {
    PlayerId(T::PlayerId),
    Delta(#[serde(bound = "")] <T as Diff>::Delta),
    Full(#[serde(bound = "")] T),
    Events(Vec<T::Event>),
}

#[derive(Clone)]
pub struct Remote<T: Model> {
    connection: Rc<RefCell<net::client::Connection<ServerMessage<T>, T::Message>>>,
    model: Rc<RefCell<T>>,
}

impl<T: Model> Remote<T> {
    pub fn update(&self) -> Vec<T::Event> {
        let mut model = self.model.borrow_mut();
        let mut events = Vec::new();
        for message in self.connection.borrow_mut().new_messages() {
            match message {
                ServerMessage::Full(state) => *model = state,
                ServerMessage::Delta(delta) => model.update(&delta),
                ServerMessage::PlayerId(_) => unreachable!(),
                ServerMessage::Events(e) => events.extend(e),
            }
        }
        events
    }
    pub fn get(&self) -> Ref<T> {
        self.model.borrow()
    }
    pub fn send(&self, message: T::Message) {
        self.connection.borrow_mut().send(message);
    }
    pub fn traffic(&self) -> net::Traffic {
        self.connection.borrow().traffic()
    }
}
