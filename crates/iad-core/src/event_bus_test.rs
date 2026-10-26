#[cfg(test)]
mod tests {
    use crate::event_bus::EventBus;

    #[tokio::test]
    async fn publish_subscribe() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe("agent").await;
        bus.publish("agent", "agent.started", "test", serde_json::json!({"name":"a"})).await;
        let event = rx.recv().await.unwrap();
        assert_eq!(event.topic, "agent");
        assert_eq!(event.event_type, "agent.started");
        assert_eq!(event.sequence, 1);
    }

    #[tokio::test]
    async fn sequence_increments() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe("t").await;
        bus.publish("t", "e1", "src", serde_json::json!({})).await;
        bus.publish("t", "e2", "src", serde_json::json!({})).await;
        let e1 = rx.recv().await.unwrap();
        let e2 = rx.recv().await.unwrap();
        assert_eq!(e1.sequence, 1);
        assert_eq!(e2.sequence, 2);
    }

    #[tokio::test]
    async fn multiple_topics_independent() {
        let bus = EventBus::new();
        let mut rx_agent = bus.subscribe("agent").await;
        let mut rx_task  = bus.subscribe("task").await;
        bus.publish("task", "task.done", "api", serde_json::json!({})).await;
        bus.publish("agent", "agent.crash", "daemon", serde_json::json!({})).await;
        let t = rx_task.recv().await.unwrap();
        let a = rx_agent.recv().await.unwrap();
        assert_eq!(t.topic, "task");
        assert_eq!(a.topic, "agent");
    }
}
