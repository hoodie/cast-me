use axum::{response::Redirect, routing::get, Router};
use axum_server::tls_rustls::RustlsConfig;
use tokio::sync::mpsc;
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;
use warp::{http::Uri, ws::WebSocket, Filter};

mod broker;
mod peer;
mod peer_actor;
mod routes;

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

#[derive(Debug, serde::Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub log_config: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(config::Environment::default())
            .build()?
            .try_deserialize()
    }
}

#[tokio::main]
#[tracing::instrument]
async fn main() {
    color_backtrace::install();
    dotenv::dotenv().unwrap();
    let config = Config::from_env().unwrap();

    tracing_subscriber::fmt()
        // .pretty()
        .with_thread_names(false)
        // enable everything
        .with_max_level(tracing::Level::TRACE)
        .with_env_filter(EnvFilter::from_default_env())
        // sets this to be the default, global collector for this application.
        .init();
    // console_subscriber::init();

    #[allow(unused)]
    let (warp_server, broker_loop) =
        {
            let (broker, broker_loop) = Broker::create();
            let broker = warp::any().map(move || broker.clone());

            let channel = warp::path("ws").and(warp::ws()).and(broker).map(
                |ws: warp::ws::Ws, broker: Broker| {
                    ws.on_upgrade(move |socket| peer_connected(socket, broker))
                },
            );

            let redirect_to_app = warp::any().map(|| warp::redirect(Uri::from_static("/app/")));
            let test =
                warp::path("test").map(|| warp::reply::html(include_str!("../static/index.html")));
            let app = warp::path("app").and(warp::fs::dir("./app/dist/"));

            let routes = test.or(app).or(channel).or(redirect_to_app);

            let listen_on: std::net::SocketAddr =
                format!("{}:{}", config.server.host, config.server.port)
                    .parse()
                    .unwrap();

            let warp_server = warp::serve(routes)
                .tls()
                .cert_path("testcerts/cert.pem")
                .key_path("testcerts/key.pem")
                .run(listen_on);
            (warp_server, broker_loop)
        };

    let axum_server = async {
        let app = Router::new()
            .route("/ws", axum::routing::get(routes::websocket_handler))
            .nest_service("/app", ServeDir::new("./app/dist"))
            .route("/", get(|| async { Redirect::permanent("/app") }));

        let tls_config = RustlsConfig::from_pem_file("testcerts/cert.pem", "testcerts/key.pem")
            .await
            .unwrap();

        let listen_on: std::net::SocketAddr =
            format!("{}:{}", config.server.host, config.server.port + 1)
                .parse()
                .unwrap();
        tracing::info!("listening on {}", listen_on);
        axum_server::tls_rustls::bind_rustls(listen_on, tls_config)
            .serve(app.into_make_service())
            .await
            .unwrap()

        // let listener = tokio::net::TcpListener::bind(listen_on).await.unwrap();
        // axum::serve(listener, app)
    };

    tokio::select! {
        // _ = broker_loop => {},
        // _ = warp_server => {},
        _ = axum_server => {},
    };
}
