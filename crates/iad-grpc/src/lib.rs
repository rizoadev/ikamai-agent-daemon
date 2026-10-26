//! IAD gRPC server — Phase 4.1-4.3.

use iad_core::{AgentRegistry, DaemonConfig};
use iad_storage::Storage;
use std::sync::Arc;
use tonic::transport::Server;
use tracing::info;

pub mod proto {
    tonic::include_proto!("iad.v1");
}

use proto::{
    agent_service_server::*, task_service_server::*, daemon_service_server::*, *,
};

#[derive(Clone)]
pub struct GrpcState {
    pub registry: Arc<AgentRegistry>,
    pub storage:  Storage,
    pub cfg:      DaemonConfig,
}

// ─── AgentService ─────────────────────────────────────────────────────────────
pub struct AgentSvc(pub GrpcState);

#[tonic::async_trait]
impl AgentService for AgentSvc {
    async fn list_agents(&self, _r: tonic::Request<ListAgentsRequest>) -> tonic::Result<tonic::Response<ListAgentsResponse>> {
        let agents = self.0.registry.list().await.into_iter().map(to_proto).collect();
        Ok(tonic::Response::new(ListAgentsResponse { agents }))
    }
    async fn get_agent(&self, r: tonic::Request<AgentRef>) -> tonic::Result<tonic::Response<Agent>> {
        let name = r.into_inner().name;
        self.0.registry.get(&name).await
            .map(|a| tonic::Response::new(to_proto(a)))
            .map_err(|_| tonic::Status::not_found(format!("'{name}' not found")))
    }
    async fn register_agent(&self, r: tonic::Request<RegisterAgentRequest>) -> tonic::Result<tonic::Response<Agent>> {
        let req = r.into_inner();
        let manifest = iad_core::AgentManifest {
            name: req.name, version: req.version, runtime: req.runtime,
            entrypoint: req.entrypoint, args: req.args, capabilities: req.capabilities,
            env: req.env, restart: iad_core::RestartPolicy::OnFailure, max_restarts: req.max_restarts,
        };
        self.0.registry.register(manifest).await
            .map(|a| { let _ = self.0.storage.upsert_agent(&a); tonic::Response::new(to_proto(a)) })
            .map_err(|e| tonic::Status::already_exists(e.to_string()))
    }
    async fn deregister_agent(&self, r: tonic::Request<AgentRef>) -> tonic::Result<tonic::Response<Empty>> {
        let name = r.into_inner().name;
        self.0.registry.deregister(&name).await
            .map(|_| { let _ = self.0.storage.delete_agent_by_name(&name); tonic::Response::new(Empty {}) })
            .map_err(|_| tonic::Status::not_found(format!("'{name}' not found")))
    }
    async fn start_agent(&self, r: tonic::Request<AgentRef>) -> tonic::Result<tonic::Response<Agent>> {
        let name = r.into_inner().name;
        self.0.registry.update(&name, |a| a.status = iad_core::AgentStatus::Registered).await
            .map(|a| tonic::Response::new(to_proto(a)))
            .map_err(|_| tonic::Status::not_found(format!("'{name}' not found")))
    }
    async fn stop_agent(&self, r: tonic::Request<AgentRef>) -> tonic::Result<tonic::Response<Agent>> {
        let name = r.into_inner().name;
        self.0.registry.get(&name).await
            .map(|a| tonic::Response::new(to_proto(a)))
            .map_err(|_| tonic::Status::not_found(format!("'{name}' not found")))
    }
}

// ─── TaskService ──────────────────────────────────────────────────────────────
pub struct TaskSvc(pub GrpcState);

#[tonic::async_trait]
impl TaskService for TaskSvc {
    async fn submit_task(&self, _r: tonic::Request<SubmitTaskRequest>) -> tonic::Result<tonic::Response<SubmitTaskResponse>> {
        Ok(tonic::Response::new(SubmitTaskResponse { task_id: ulid::Ulid::new().to_string(), status: "pending".into() }))
    }
    async fn get_task(&self, r: tonic::Request<TaskRef>) -> tonic::Result<tonic::Response<Task>> {
        Err(tonic::Status::not_found(format!("'{}' not found", r.into_inner().id)))
    }
    async fn list_tasks(&self, _r: tonic::Request<ListTasksRequest>) -> tonic::Result<tonic::Response<ListTasksResponse>> {
        Ok(tonic::Response::new(ListTasksResponse { tasks: vec![] }))
    }
    async fn cancel_task(&self, _r: tonic::Request<TaskRef>) -> tonic::Result<tonic::Response<Empty>> {
        Ok(tonic::Response::new(Empty {}))
    }
}

// ─── DaemonService ────────────────────────────────────────────────────────────
pub struct DaemonSvc(pub GrpcState);

#[tonic::async_trait]
impl DaemonService for DaemonSvc {
    async fn status(&self, _r: tonic::Request<Empty>) -> tonic::Result<tonic::Response<DaemonStatusResponse>> {
        let agents = self.0.registry.list().await;
        Ok(tonic::Response::new(DaemonStatusResponse {
            version: env!("CARGO_PKG_VERSION").into(),
            agents:  agents.len() as u32,
            tasks:   0,
            uptime:  "unknown".into(),
        }))
    }
    async fn reload(&self, _r: tonic::Request<Empty>) -> tonic::Result<tonic::Response<Empty>> {
        unsafe { libc::kill(std::process::id() as i32, libc::SIGHUP); }
        Ok(tonic::Response::new(Empty {}))
    }
}

pub async fn serve(state: GrpcState, addr: std::net::SocketAddr) -> Result<(), tonic::transport::Error> {
    info!("gRPC server listening on {addr}");
    Server::builder()
        .add_service(AgentServiceServer::new(AgentSvc(state.clone())))
        .add_service(TaskServiceServer::new(TaskSvc(state.clone())))
        .add_service(DaemonServiceServer::new(DaemonSvc(state)))
        .serve(addr)
        .await
}

fn to_proto(a: iad_core::Agent) -> Agent {
    Agent {
        id:           a.id,
        name:         a.name,
        version:      a.version,
        status:       match a.status {
            iad_core::AgentStatus::Registered => 1, iad_core::AgentStatus::Starting => 2,
            iad_core::AgentStatus::Running    => 3, iad_core::AgentStatus::Stopping => 4,
            iad_core::AgentStatus::Stopped    => 5, iad_core::AgentStatus::Failed   => 6,
            iad_core::AgentStatus::Crashed    => 7, iad_core::AgentStatus::Disabled => 8,
        },
        pid:           a.pid,
        generation_id: a.generation_id,
        failure_count: a.failure_count,
        capabilities:  a.manifest.capabilities,
        created_at:    a.created_at.to_rfc3339(),
    }
}
