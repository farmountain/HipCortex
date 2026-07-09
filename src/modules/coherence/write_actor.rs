#[cfg(feature = "tokio")]
use std::sync::Arc;
#[cfg(feature = "tokio")]
use std::path::PathBuf;
#[cfg(feature = "tokio")]
use tokio::sync::{mpsc, oneshot};
#[cfg(feature = "tokio")]
use crate::snapshot_manager::SnapshotManager;
#[cfg(feature = "tokio")]
use crate::coherence::CoherenceChecker;

#[cfg(feature = "tokio")]
#[derive(Debug, Clone)]
pub struct CoherenceWriteMutation {
    pub mutation_id: String,
    pub payload: Vec<u8>,
    pub target_store: String,
}

#[cfg(feature = "tokio")]
#[derive(Clone)]
pub struct CoherenceWriteActor {
    tx: mpsc::Sender<(CoherenceWriteMutation, oneshot::Sender<Result<(), String>>)>,
}

#[cfg(feature = "tokio")]
impl CoherenceWriteActor {
    pub fn new(
        coherence_checker: Arc<CoherenceChecker>,
        wal_path: PathBuf,
        snapshot_source_path: PathBuf,
        buffer_size: usize,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<(CoherenceWriteMutation, oneshot::Sender<Result<(), String>>)>(buffer_size);

        tokio::spawn(async move {
            use std::io::Write;
            while let Some((mutation, responder)) = rx.recv().await {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&wal_path)
                {
                    let log_entry = format!(
                        "{}:{}\n",
                        mutation.mutation_id,
                        hex::encode(&mutation.payload)
                    );
                    let _ = file.write_all(log_entry.as_bytes());
                }

                let write_gate = coherence_checker.gate_write(&mutation.target_store);
                match write_gate {
                    Ok(()) => {
                        let _ = responder.send(Ok(()));
                    }
                    Err(rejection) => {
                        let has_critical = rejection.violations.iter().any(|v| v.critical);
                        if has_critical {
                            let _ = SnapshotManager::rollback_to_latest(&snapshot_source_path);
                        }
                        let _ = responder.send(Err(rejection.reason));
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn submit_mutation(&self, mutation: CoherenceWriteMutation) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx.send((mutation, tx))
            .await
            .map_err(|e| format!("CoherenceWriteActor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }
}

#[cfg(all(test, feature = "tokio"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_coherence_write_actor_wal_logging_and_gating() {
        let checker = Arc::new(CoherenceChecker::new());
        let temp_dir = std::env::temp_dir();
        let wal_path = temp_dir.join(format!("test_wal_{}.log", uuid::Uuid::new_v4()));
        let snap_path = temp_dir.join(format!("test_snap_{}.json", uuid::Uuid::new_v4()));

        let actor = CoherenceWriteActor::new(
            checker,
            wal_path.clone(),
            snap_path,
            32,
        );

        let mutation = CoherenceWriteMutation {
            mutation_id: "mut_001".to_string(),
            payload: b"test_payload_data".to_vec(),
            target_store: "temporal_indexer".to_string(),
        };

        let res = actor.submit_mutation(mutation).await;
        assert!(res.is_ok());

        assert!(wal_path.exists());
        let wal_contents = std::fs::read_to_string(&wal_path).unwrap();
        assert!(wal_contents.contains("mut_001"));

        let _ = std::fs::remove_file(wal_path);
    }
}
