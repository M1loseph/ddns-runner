use std::sync::Arc;

use axum::extract::State;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use log::error;
use prometheus::TextEncoder;

pub struct AppState {
    pub registry: prometheus::Registry,
}

pub async fn prometheus_handler(
    State(state): State<Arc<AppState>>,
) -> Result<(HeaderMap, String), (StatusCode, &'static str)> {
    let encoder = TextEncoder::new();
    let metric_families = state.registry.gather();
    let response = match encoder.encode_to_string(&metric_families) {
        Ok(v) => v,
        Err(e) => {
            error!("Error occurred while encoding metrics to string: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Error occurred while encoding metrics",
            ));
        }
    };

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; version=0.0.4"),
    );
    Ok((headers, response))
}

#[cfg(test)]
mod tests {

    use axum::routing::get;
    use axum::Router;
    use axum_test::TestServer;
    use prometheus::Gauge;

    use super::*;

    #[tokio::test]
    async fn should_return_prometheus_metrics() {
        // given
        let registry = prometheus::Registry::new();
        let gauge = Gauge::new("test_gauge", "A test gauge").unwrap();
        gauge.set(42.0);
        registry.register(Box::new(gauge)).unwrap();

        let app_state = Arc::new(AppState { registry });
        let app = Router::new()
            .route("/internal/status/prometheus", get(prometheus_handler))
            .with_state(app_state);
        let test_server = TestServer::new(app).unwrap();

        // when
        let response = test_server.get("/internal/status/prometheus").await;

        // then
        response.assert_status(StatusCode::OK);
        response.assert_header("content-type", "text/plain; version=0.0.4");
        response.assert_text(
            "# HELP test_gauge A test gauge\n# TYPE test_gauge gauge\ntest_gauge 42\n",
        );
    }
}
