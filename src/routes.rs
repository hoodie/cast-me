pub mod axum {
    use axum::{extract::ws::WebSocketUpgrade, response::Response};
    use futures::StreamExt;

    use crate::actors::Peer;

    pub async fn peer_connected(ws: WebSocketUpgrade) -> Response {
        ws.on_upgrade(|socket| async move {
            let (sender, messages) = socket.split();
            if let Err(error) = hannibal::build(Peer::new(sender))
                .on_stream(messages)
                .spawn()
                .await
            {
                tracing::warn!("websocket peer failed {error}")
            }
            tracing::info!("peer ended")
        })
    }
}
pub mod warp {
    use warp::filters::ws::WebSocket;

    use crate::basic::{Broker, Peer};

    pub async fn peer_connected(ws: WebSocket, broker: Broker) {
        tracing::debug!("user connected{:#?}", ws);

        let mut peer = Peer::new(ws, broker.addr());
        peer.register_at_broker();
        peer.send_welcome().await;
        peer.start().await;
    }
}
