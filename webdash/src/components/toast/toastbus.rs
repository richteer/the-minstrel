
use yew_agent::{
    Agent,
    AgentLink,
    Context as AgentContext,
    HandlerId,
};

use super::ToastType;


pub struct ToastBus {
    link: AgentLink<ToastBus>,
    main: Option<HandlerId>,
}

impl Agent for ToastBus {
    type Reach = AgentContext<Self>;
    type Message = ();
    type Input = ToastType;
    type Output = ToastType;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link,
            main: None,
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        if let Some(m) = self.main {
            self.link.respond(m, msg.clone())
        }
    }

    fn connected(&mut self, id: HandlerId) {
        // TODO: This is kind of sketchy, it only take the first *respondable* connection
        //  This happens because main is going to be the first connection, but since it isn't
        //  Using the use_bridge hook, it won't be a respondable connection like those from
        //  the use_bridge hook. Consider implementing a better approach to this, or just
        //  allow those callbacks to run no-ops.
        if !id.is_respondable() {
            return;
        }

        match self.main {
            Some(_) => (),
            None => {self.main = Some(id);}
        }
    }

    fn disconnected(&mut self, _id: HandlerId) {

    }
}