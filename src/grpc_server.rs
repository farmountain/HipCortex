#[cfg(feature = "grpc-server")]
pub mod grpc {
    tonic::include_proto!("hipcortex");
}

#[cfg(feature = "grpc-server")]
use crate::memory_record::{MemoryRecord, MemoryType};
#[cfg(feature = "grpc-server")]
use crate::memory_store::MemoryStore;
#[cfg(feature = "grpc-server")]
use crate::persistence::MemoryBackend;
#[cfg(feature = "grpc-server")]
use chrono::TimeZone;
#[cfg(feature = "grpc-server")]
use grpc::memory_service_server::{MemoryService, MemoryServiceServer};
#[cfg(feature = "grpc-server")]
use grpc::{AddRecordRequest, AddRecordResponse, ListRecordsRequest, ListRecordsResponse};
#[cfg(feature = "grpc-server")]
use std::net::SocketAddr;
#[cfg(feature = "grpc-server")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "grpc-server")]
#[derive(Clone)]
struct MemoryServiceImpl<B: MemoryBackend + Send + 'static> {
    store: Arc<Mutex<MemoryStore<B>>>,
}

#[cfg(feature = "grpc-server")]
#[tonic::async_trait]
impl<B: MemoryBackend + Send + 'static> MemoryService for MemoryServiceImpl<B> {
    async fn add_record(
        &self,
        request: tonic::Request<AddRecordRequest>,
    ) -> Result<tonic::Response<AddRecordResponse>, tonic::Status> {
        let rec = request
            .into_inner()
            .record
            .ok_or_else(|| tonic::Status::invalid_argument("missing record"))?;
        let mtype: MemoryType = match rec.record_type.as_str() {
            "Temporal" => MemoryType::Temporal,
            "Symbolic" => MemoryType::Symbolic,
            "Procedural" => MemoryType::Procedural,
            "Reflexion" => MemoryType::Reflexion,
            "Perception" => MemoryType::Perception,
            _ => return Err(tonic::Status::invalid_argument("record_type")),
        };
        let mut record = MemoryRecord {
            id: rec
                .id
                .parse()
                .map_err(|_| tonic::Status::invalid_argument("id"))?,
            record_type: mtype,
            timestamp: chrono::Utc
                .timestamp_opt(rec.timestamp, 0)
                .single()
                .ok_or_else(|| tonic::Status::invalid_argument("timestamp"))?,
            actor: rec.actor,
            action: rec.action,
            target: rec.target,
            metadata: serde_json::from_str(&rec.metadata)
                .map_err(|_| tonic::Status::invalid_argument("metadata"))?,
            integrity: None,
        };
        let hash = record.compute_hash();
        record.integrity = Some(hash);
        {
            let mut store = self.store.lock().unwrap();
            store
                .add(record)
                .map_err(|e| tonic::Status::internal(e.to_string()))?;
        }
        Ok(tonic::Response::new(AddRecordResponse { ok: true }))
    }

    async fn list_records(
        &self,
        _req: tonic::Request<ListRecordsRequest>,
    ) -> Result<tonic::Response<ListRecordsResponse>, tonic::Status> {
        let store = self.store.lock().unwrap();
        let records = store
            .all()
            .iter()
            .map(|r| grpc::MemoryRecord {
                id: r.id.to_string(),
                record_type: format!("{:?}", r.record_type),
                timestamp: r.timestamp.timestamp(),
                actor: r.actor.clone(),
                action: r.action.clone(),
                target: r.target.clone(),
                metadata: serde_json::to_string(&r.metadata).unwrap_or_default(),
            })
            .collect();
        Ok(tonic::Response::new(ListRecordsResponse { records }))
    }
}

#[cfg(feature = "grpc-server")]
pub async fn serve<B: MemoryBackend + Send + 'static>(
    addr: SocketAddr,
    store: Arc<Mutex<MemoryStore<B>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let svc = MemoryServiceImpl { store };
    tonic::transport::Server::builder()
        .add_service(MemoryServiceServer::new(svc))
        .serve(addr)
        .await?;
    Ok(())
}
