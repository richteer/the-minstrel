
use std::collections::HashSet;

use yew_agent::{
    Agent,
    AgentLink,
    Context as AgentContext,
    HandlerId,
};

use super::ToastType;


pub struct ToastBus {
    link: AgentLink<ToastBus>,
    subs: HashSet<HandlerId>,
}

impl Agent for ToastBus {
    type Reach = AgentContext<Self>;
    type Message = ();
    type Input = ToastType;
    type Output = ToastType;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            subs: HashSet::new(),
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        for s in self.subs.iter().filter(|s| s.is_respondable()) {
            self.link.respond(*s, msg.clone())
        }
    }

    fn connected(&mut self, id: HandlerId) {
        self.subs.insert(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subs.remove(&id);
    }
}