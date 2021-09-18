use tokio::sync::mpsc;
use tracing_subscriber::EnvFilter;
use warp::{http::Uri, ws::WebSocket, Filter};

mod broker;
mod peer;

use crate::{broker::Broker, peer::Peer};

type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

#[allow(clippy::cognitive_complexity)]
async fn peer_connected(ws: WebSocket, broker: Broker) {
    tracing::debug!("user connected{:#?}", ws);

    let mut peer = Peer::new(ws, broker.addr());
    peer.register_at_broker();
    peer.send_welcome().await;
    peer.start().await;
}

#[tokio::main]
#[tracing::instrument]
async fn main() {
    color_backtrace::install();

    tracing_subscriber::fmt()
        .pretty()
        .with_thread_names(true)
        // enable everything
        .with_max_level(tracing::Level::TRACE)
        .with_env_filter(EnvFilter::from_default_env())
        // sets this to be the default, global collector for this application.
        .init();

    let (broker, broker_loop) = Broker::create();
    let broker = warp::any().map(move || broker.clone());

    let channel =
        warp::path("ws")
            .and(warp::ws())
            .and(broker)
            .map(|ws: warp::ws::Ws, broker: Broker| {
                ws.on_upgrade(move |socket| peer_connected(socket, broker))
            });

    let redirect_to_app = warp::any().map(|| warp::redirect(Uri::from_static("/app/")));
    let test = warp::path("test").map(|| warp::reply::html(include_str!("../static/index.html")));
    let app = warp::path("app").and(warp::fs::dir("./app/public/"));

    let routes = test.or(app).or(channel).or(redirect_to_app);

    let listen_on = std::net::SocketAddr::from(([0, 0, 0, 0], 3030));
    tracing::info!("listening on {}", listen_on);

    tokio::select! {
        _ = broker_loop => {},
        _ = warp::serve(routes)
            .tls()
            .cert_path("testcerts/cert.pem")
            .key_path("testcerts/key.pem")
            .run(listen_on) => {},
    };
}
