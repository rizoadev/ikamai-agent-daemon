#[cfg(test)]
mod tests {
    use axum::{body::Body, http::{Request, StatusCode}};
    use tower::ServiceExt;
    use iad_core::{AgentRegistry, DaemonConfig, event_bus::EventBus};
    use iad_storage::Storage;
    use std::sync::Arc;
    use crate::{build_router, AppState, metrics::Metrics};

    fn test_state() -> AppState {
        AppState {
            storage:       Storage::open(std::path::Path::new(":memory:")).unwrap(),
            registry:      Arc::new(AgentRegistry::new()),
            cfg:           DaemonConfig::default(),
            api_token:     None,
            metrics:       Metrics::new(),
            event_bus:     EventBus::new(),
            alert_mgr:     None,
            conn_registry: None,
        }
    }

    #[tokio::test]
    async fn healthz_ok() {
        let resp = build_router(test_state())
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn agents_list_empty() {
        let resp = build_router(test_state())
            .oneshot(Request::builder().uri("/api/v1/agents").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn register_agent_created() {
        let payload = serde_json::json!({
            "manifest": {"name":"t","version":"1.0.0","runtime":"bash","entrypoint":"/bin/echo"}
        });
        let resp = build_router(test_state()).oneshot(
            Request::builder().method("POST").uri("/api/v1/agents")
                .header("Content-Type","application/json")
                .body(Body::from(payload.to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn auth_required() {
        let mut state = test_state();
        state.api_token = Some("tok".into());
        let app = build_router(state);
        let r1 = app.clone().oneshot(Request::builder().uri("/api/v1/agents").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(r1.status(), StatusCode::UNAUTHORIZED);
        let r2 = app.oneshot(
            Request::builder().uri("/api/v1/agents")
                .header("Authorization","Bearer tok").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(r2.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn metrics_endpoint() {
        let resp = build_router(test_state())
            .oneshot(Request::builder().uri("/metrics").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 8192).await.unwrap();
        assert!(String::from_utf8_lossy(&body).contains("iad_tasks_submitted_total"));
    }

    #[tokio::test]
    async fn status_endpoint() {
        let resp = build_router(test_state())
            .oneshot(Request::builder().uri("/api/v1/status").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn submit_task_returns_id() {
        let payload = serde_json::json!({"payload":{"text":"hi"},"capabilities":["echo"]});
        let resp = build_router(test_state()).oneshot(
            Request::builder().method("POST").uri("/api/v1/tasks")
                .header("Content-Type","application/json")
                .body(Body::from(payload.to_string())).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn delete_nonexistent_agent_404() {
        let resp = build_router(test_state()).oneshot(
            Request::builder().method("DELETE").uri("/api/v1/agents/ghost").body(Body::empty()).unwrap()
        ).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn team_list_endpoint() {
        let resp = build_router(test_state())
            .oneshot(Request::builder().uri("/api/v1/teams").body(Body::empty()).unwrap()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(json["teams"].is_object());
    }
}
