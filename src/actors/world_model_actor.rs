#[cfg(feature = "tokio")]
use crate::world_model_enhanced::{
    EntityObservation, EntityState, InterventionQuery, TransitionPrediction, WorldModelEnhanced,
};
#[cfg(feature = "tokio")]
use std::collections::HashMap;
#[cfg(feature = "tokio")]
use tokio::sync::{mpsc, oneshot};

#[cfg(feature = "tokio")]
#[derive(Debug)]
pub enum WorldModelMessage {
    RecordPerceivedAction {
        action: String,
    },
    ObserveTransition {
        from_state: String,
        action: String,
        to_state: String,
        responder: oneshot::Sender<Result<(), String>>,
    },
    PredictNextState {
        current_state: String,
        action: String,
        responder: oneshot::Sender<Result<TransitionPrediction, String>>,
    },
    RegisterEntity {
        entity_id: String,
        initial_state: EntityState,
        responder: oneshot::Sender<Result<(), String>>,
    },
    UpdateEntity {
        entity_id: String,
        observation: EntityObservation,
        responder: oneshot::Sender<Result<(), String>>,
    },
    PredictEntity {
        entity_id: String,
        steps: usize,
        responder: oneshot::Sender<Result<EntityState, String>>,
    },
    AddCausalEdge {
        from: String,
        to: String,
        responder: oneshot::Sender<Result<(), String>>,
    },
    CausalIntervention {
        query: InterventionQuery,
        responder: oneshot::Sender<Result<HashMap<String, f64>, String>>,
    },
    Counterfactual {
        actual_state: HashMap<String, f64>,
        intervention_var: String,
        intervention_value: f64,
        responder: oneshot::Sender<Result<HashMap<String, f64>, String>>,
    },
}

#[cfg(feature = "tokio")]
#[derive(Clone)]
pub struct WorldModelActorClient {
    tx: mpsc::Sender<WorldModelMessage>,
}

#[cfg(feature = "tokio")]
impl WorldModelActorClient {
    pub fn new(buffer_size: usize) -> Self {
        let (tx, mut rx) = mpsc::channel::<WorldModelMessage>(buffer_size);

        tokio::spawn(async move {
            let world_model = WorldModelEnhanced::new();
            while let Some(msg) = rx.recv().await {
                match msg {
                    WorldModelMessage::RecordPerceivedAction { action } => {
                        world_model.record_perceived_action(action);
                    }
                    WorldModelMessage::ObserveTransition {
                        from_state,
                        action,
                        to_state,
                        responder,
                    } => {
                        let res = world_model.observe_transition(from_state, action, to_state);
                        let _ = responder.send(res);
                    }
                    WorldModelMessage::PredictNextState {
                        current_state,
                        action,
                        responder,
                    } => {
                        let res = world_model.predict_next_state(&current_state, &action);
                        let _ = responder.send(res);
                    }
                    WorldModelMessage::RegisterEntity {
                        entity_id,
                        initial_state,
                        responder,
                    } => {
                        let res = world_model.register_entity(entity_id, initial_state);
                        let _ = responder.send(res);
                    }
                    WorldModelMessage::UpdateEntity {
                        entity_id,
                        observation,
                        responder,
                    } => {
                        let res = world_model.update_entity(&entity_id, observation);
                        let _ = responder.send(res);
                    }
                    WorldModelMessage::PredictEntity {
                        entity_id,
                        steps,
                        responder,
                    } => {
                        let res = world_model.predict_entity(&entity_id, steps);
                        let _ = responder.send(res);
                    }
                    WorldModelMessage::AddCausalEdge {
                        from,
                        to,
                        responder,
                    } => {
                        let res = world_model.add_causal_edge(from, to);
                        let _ = responder.send(res);
                    }
                    WorldModelMessage::CausalIntervention { query, responder } => {
                        let res = world_model.causal_intervention(query);
                        let _ = responder.send(res);
                    }
                    WorldModelMessage::Counterfactual {
                        actual_state,
                        intervention_var,
                        intervention_value,
                        responder,
                    } => {
                        let res = world_model.counterfactual(
                            actual_state,
                            intervention_var,
                            intervention_value,
                        );
                        let _ = responder.send(res);
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn record_perceived_action(
        &self,
        action: String,
    ) -> Result<(), mpsc::error::SendError<WorldModelMessage>> {
        self.tx
            .send(WorldModelMessage::RecordPerceivedAction { action })
            .await
    }

    pub async fn observe_transition(
        &self,
        from_state: String,
        action: String,
        to_state: String,
    ) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldModelMessage::ObserveTransition {
                from_state,
                action,
                to_state,
                responder: tx,
            })
            .await
            .map_err(|e| format!("Actor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }

    pub async fn predict_next_state(
        &self,
        current_state: String,
        action: String,
    ) -> Result<TransitionPrediction, String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldModelMessage::PredictNextState {
                current_state,
                action,
                responder: tx,
            })
            .await
            .map_err(|e| format!("Actor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }

    pub async fn register_entity(
        &self,
        entity_id: String,
        initial_state: EntityState,
    ) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldModelMessage::RegisterEntity {
                entity_id,
                initial_state,
                responder: tx,
            })
            .await
            .map_err(|e| format!("Actor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }

    pub async fn update_entity(
        &self,
        entity_id: String,
        observation: EntityObservation,
    ) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldModelMessage::UpdateEntity {
                entity_id,
                observation,
                responder: tx,
            })
            .await
            .map_err(|e| format!("Actor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }

    pub async fn predict_entity(
        &self,
        entity_id: String,
        steps: usize,
    ) -> Result<EntityState, String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldModelMessage::PredictEntity {
                entity_id,
                steps,
                responder: tx,
            })
            .await
            .map_err(|e| format!("Actor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }

    pub async fn add_causal_edge(&self, from: String, to: String) -> Result<(), String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldModelMessage::AddCausalEdge {
                from,
                to,
                responder: tx,
            })
            .await
            .map_err(|e| format!("Actor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }

    pub async fn causal_intervention(
        &self,
        query: InterventionQuery,
    ) -> Result<HashMap<String, f64>, String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldModelMessage::CausalIntervention {
                query,
                responder: tx,
            })
            .await
            .map_err(|e| format!("Actor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }

    pub async fn counterfactual(
        &self,
        actual_state: HashMap<String, f64>,
        intervention_var: String,
        intervention_value: f64,
    ) -> Result<HashMap<String, f64>, String> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(WorldModelMessage::Counterfactual {
                actual_state,
                intervention_var,
                intervention_value,
                responder: tx,
            })
            .await
            .map_err(|e| format!("Actor channel closed: {}", e))?;
        rx.await.map_err(|e| format!("Oneshot closed: {}", e))?
    }
}

#[cfg(all(test, feature = "tokio"))]
mod tests {
    use super::*;
    use crate::world_model_enhanced::{EntityObservation, EntityState, InterventionQuery};
    use std::collections::HashMap;
    use std::time::Instant;

    #[tokio::test]
    async fn test_world_model_actor_client_transitions_and_predictions() {
        let client = WorldModelActorClient::new(32);
        client
            .record_perceived_action("move_forward".to_string())
            .await
            .unwrap();

        client
            .observe_transition(
                "room_a".to_string(),
                "move_forward".to_string(),
                "room_b".to_string(),
            )
            .await
            .unwrap();
        let prediction = client
            .predict_next_state("room_a".to_string(), "move_forward".to_string())
            .await
            .unwrap();
        assert!(prediction.probabilities.contains_key("room_b"));
    }

    #[tokio::test]
    async fn test_world_model_actor_client_entities() {
        let client = WorldModelActorClient::new(32);
        let init_state = EntityState {
            properties: vec![0.0, 0.0, 0.0],
            covariance: vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
        };
        client
            .register_entity("robot_1".to_string(), init_state)
            .await
            .unwrap();

        let obs = EntityObservation {
            measured_properties: vec![1.0, 0.0, 0.0],
            measurement_noise: vec![
                vec![0.1, 0.0, 0.0],
                vec![0.0, 0.1, 0.0],
                vec![0.0, 0.0, 0.1],
            ],
            timestamp: Instant::now(),
        };
        client
            .update_entity("robot_1".to_string(), obs)
            .await
            .unwrap();

        let pred = client
            .predict_entity("robot_1".to_string(), 2)
            .await
            .unwrap();
        assert!(!pred.properties.is_empty());
    }

    #[tokio::test]
    async fn test_world_model_actor_client_causal_and_counterfactual() {
        let client = WorldModelActorClient::new(32);
        client
            .add_causal_edge("rain".to_string(), "wet_floor".to_string())
            .await
            .unwrap();

        let query = InterventionQuery {
            outcome: "wet_floor".to_string(),
            intervention_var: "rain".to_string(),
            intervention_value: 1.0,
            conditioned_on: HashMap::new(),
            intervention_label: None,
        };
        let intervention_res = client.causal_intervention(query).await.unwrap();
        assert!(intervention_res.contains_key("wet_floor") || intervention_res.is_empty());

        let mut actual = HashMap::new();
        actual.insert("rain".to_string(), 0.0);
        actual.insert("wet_floor".to_string(), 0.0);
        let cf_res = client
            .counterfactual(actual, "rain".to_string(), 1.0)
            .await
            .unwrap();
        assert!(cf_res.contains_key("wet_floor") || cf_res.is_empty());
    }
}
