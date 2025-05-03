use axum::{extract::ws::WebSocketUpgrade, response::Response};

pub async fn websocket_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| async move {
        if let Err(error) = hannibal::build(crate::peer_actor::Peer)
            .on_stream(socket)
            .spawn()
            .await
        {
            tracing::warn!("websocket peer failed {error}")
        }
    })
}
