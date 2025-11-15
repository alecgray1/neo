use crate::actors::PubSubBroker;
use crate::messages::StationMsg;
use crate::types::ServiceState;
use tracing::info;

/// Root station actor - supervises all subsystems
#[derive(kameo::Actor)]
pub struct StationActor {
    name: String,
    #[allow(dead_code)]
    pubsub: Option<kameo::actor::ActorRef<PubSubBroker>>,
}

impl StationActor {
    pub fn new(name: String) -> Self {
        Self {
            name,
            pubsub: None,
        }
    }
}

impl kameo::message::Message<StationMsg> for StationActor {
    type Reply = StationReply;

    async fn handle(
        &mut self,
        msg: StationMsg,
        _ctx: &mut kameo::message::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            StationMsg::GetStatus => StationReply::Status {
                name: self.name.clone(),
                state: ServiceState::Running,
            },

            StationMsg::GetStats => StationReply::Stats {
                name: self.name.clone(),
            },

            StationMsg::Save => {
                info!("Saving station '{}'...", self.name);
                // TODO: Implement persistence
                StationReply::Saved
            }

            StationMsg::Shutdown => {
                info!("Shutdown requested for station '{}'", self.name);
                StationReply::ShuttingDown
            }
        }
    }
}

#[derive(Debug, kameo::Reply)]
pub enum StationReply {
    Status { name: String, state: ServiceState },
    Stats { name: String },
    Saved,
    ShuttingDown,
}
