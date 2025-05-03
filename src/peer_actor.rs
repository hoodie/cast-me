use axum::extract::ws::Message;
use hannibal::{Actor, StreamHandler};

pub struct Peer;

impl Actor for Peer {
    async fn started(&mut self, ctx: &mut hannibal::Context<Self>) -> hannibal::DynResult {
        tracing::info!("peer started {}");
        Ok(())
    }
    async fn stopped(&mut self, _: &mut hannibal::Context<Self>) {
        tracing::info!("peer stopped");
    }
}

type WsStreamMessage = std::result::Result<Message, axum::Error>;

impl StreamHandler<WsStreamMessage> for Peer {
    async fn handle(&mut self, ctx: &mut hannibal::Context<Self>, msg: WsStreamMessage) {
        match msg {
            Ok(Message::Text(text)) => {
                tracing::debug!("peer received text message: {}", text);
                if let Ok(crate::peer::Protocol::Connect(uuid)) =
                    serde_json::from_str(text.as_str())
                {
                    tracing::debug!("connecting to {}", uuid);
                    todo!()
                }
            }
            Ok(Message::Close(_close_frame)) => ctx.stop().unwrap(),
            // Ok(Message::Binary(bytes)) => todo!(),
            // Ok(Message::Ping(bytes)) => todo!(),
            // Ok(Message::Pong(bytes)) => todo!(),
            _ => {}
        };
    }
}
