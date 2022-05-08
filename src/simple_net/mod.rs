use geng::net;
use geng::prelude::*;

mod lobby;

pub use lobby::*;
#[cfg(not(target_arch = "wasm32"))]
pub mod server;
#[cfg(not(target_arch = "wasm32"))]
pub use server::*;

pub trait Model: 'static + Send {
    type SharedState: Diff + net::Message;
    fn shared_state(&self) -> &Self::SharedState;
    type PlayerId: net::Message + Clone;
    type Message: net::Message;
    type Event: net::Message + Clone;
    const TICKS_PER_SECOND: f32;
    fn new_player(&mut self, events: &mut Vec<Self::Event>) -> Self::PlayerId;
    fn drop_player(&mut self, events: &mut Vec<Self::Event>, player_id: &Self::PlayerId);
    fn handle_message(
        &mut self,
        events: &mut Vec<Self::Event>,
        player_id: &Self::PlayerId,
        message: Self::Message,
    );
    fn tick(&mut self, events: &mut Vec<Self::Event>);
}

#[derive(Serialize, Deserialize, Derivative)]
#[derivative(Debug(bound = ""))]
pub enum ServerMessage<T: Model> {
    PlayerId(T::PlayerId),
    Delta(#[serde(bound = "")] <T::SharedState as Diff>::Delta),
    Full(#[serde(bound = "")] T::SharedState),
    Events(Vec<T::Event>),
}

#[derive(Derivative)]
#[derivative(Clone(bound = ""))]
pub struct Remote<T: Model> {
    connection: Rc<RefCell<net::client::Connection<ServerMessage<T>, T::Message>>>,
    model: Rc<RefCell<T::SharedState>>,
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
    pub fn get(&self) -> Ref<T::SharedState> {
        self.model.borrow()
    }
    pub fn send(&self, message: T::Message) {
        self.connection.borrow_mut().send(message);
    }
    pub fn traffic(&self) -> net::Traffic {
        self.connection.borrow().traffic()
    }
}
