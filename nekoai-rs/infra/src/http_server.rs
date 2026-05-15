use std::{net::SocketAddr, sync::Arc};

use axum::{Router, extract::State, http::Method, response::sse::Event, routing::get};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use tower_http::cors::{Any, CorsLayer};

use crate::web_ui_agent::WebUiAgent;

#[derive(Clone)]
pub struct HttpServerState {
    pub agent: Arc<dyn WebUiAgent>,
}

pub struct HttpServer {
    state: HttpServerState,
}

impl HttpServer {
    pub fn new(agent: Arc<dyn WebUiAgent>) -> Self {
        Self {
            state: HttpServerState { agent },
        }
    }

    pub async fn serve(self, addr: SocketAddr) -> anyhow::Result<()> {
        let cors = CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
            .allow_origin(Any)
            .allow_headers(Any);

        let app = Router::new()
            .route("/api/events", get(sse_handler))
            .route("/api/metrics", get(metrics_handler))
            .layer(cors)
            .with_state(self.state);

        tracing::info!(addr = %addr, "starting HTTP server");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn sse_handler(
    State(state): State<HttpServerState>,
) -> axum::response::Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>>
{
    let rx = state.agent.event_bus().subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => {
            let json = serde_json::to_string(&event).unwrap_or_default();
            Some(Ok(Event::default().data(json)))
        }
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(n)) => {
            tracing::warn!(skipped = n, "SSE client lagged");
            None
        }
    });
    axum::response::Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}

async fn metrics_handler(State(state): State<HttpServerState>) -> String {
    state.agent.metrics().collect_prometheus()
}
