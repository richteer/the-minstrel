use yew_agent::{
    Agent,
    AgentLink,
    Context as AgentContext,
    HandlerId,
};

use model::MinstrelBroadcast;

pub struct WsBus {
    link: AgentLink<WsBus>,
    dash: Option<HandlerId>,
}

impl Agent for WsBus {
    type Reach = AgentContext<Self>;
    type Message = ();
    type Input = String;
    type Output = MinstrelBroadcast;

    fn create(link: AgentLink<Self>) -> Self {
        Self {
            link: link,
            dash: None,
        }
    }

    fn update(&mut self, _msg: Self::Message) {}

    fn handle_input(&mut self, msg: Self::Input, _id: HandlerId) {
        if let Ok(data) = serde_json::from_str::<MinstrelBroadcast>(&msg) {
            self.link.respond(self.dash.unwrap(), data);
        } else {
            log::error!("failed to decode json data from websocket");
        };


    }

    fn connected(&mut self, id: HandlerId) {
        self.dash = Some(id);
    }

    fn disconnected(&mut self, _id: HandlerId) {
        self.dash = None;
    }
}