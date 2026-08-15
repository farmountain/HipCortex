// Entity Tracking with Kalman Filtering
//
// Implements optimal linear state estimation with predict/update cycle.
// Detects anomalies using Mahalanobis distance and maintains entity permanence.

use std::time::{Duration, Instant};

/// Entity state representation (mean and covariance)
#[derive(Debug, Clone)]
pub struct EntityState {
    /// State vector (position, velocity, etc.)
    pub properties: Vec<f64>,

    /// Covariance matrix (uncertainty)
    pub covariance: Vec<Vec<f64>>,
}

/// Measurement observation for Kalman update
#[derive(Debug, Clone)]
pub struct EntityObservation {
    /// Observed properties
    pub measured_properties: Vec<f64>,

    /// Measurement noise covariance
    pub measurement_noise: Vec<Vec<f64>>,

    /// Timestamp
    pub timestamp: Instant,
}

/// Anomaly detection result
#[derive(Debug, Clone)]
pub struct Anomaly {
    pub severity: f64,  // Mahalanobis distance
    pub threshold: f64, // Detection threshold (typically 3σ)
    pub description: String,
}

/// Kalman filter for entity tracking
///
/// Implements the standard Kalman filter equations:
/// Prediction:  x' = F×x, P' = F×P×F^T + Q
/// Update: K = P×H^T×(H×P×H^T + R)^-1, x = x + K×(z - H×x), P = (I - K×H)×P
pub struct EntityTracker {
    /// Current state estimate
    state: EntityState,

    /// State transition matrix F (identity for now)
    transition_matrix: Vec<Vec<f64>>,

    /// Process noise covariance Q
    process_noise: Vec<Vec<f64>>,

    /// Observation matrix H (identity - direct observation)
    observation_matrix: Vec<Vec<f64>>,

    /// Last update timestamp
    last_update: Instant,

    /// Anomaly threshold (Mahalanobis distance)
    anomaly_threshold: f64,

    /// Entity permanence timeout (seconds)
    permanence_timeout: Duration,

    /// Detected anomalies
    anomalies: Vec<Anomaly>,
}

/// Configuration for EntityTracker allowing custom Kalman F matrix.
#[derive(Debug, Clone, Default)]
pub struct EntityConfig {
    /// Optional custom state-transition matrix F (must be dim×dim).
    /// If None, identity matrix is used.
    pub f_matrix: Option<Vec<Vec<f64>>>,
}

impl EntityTracker {
    /// Create new tracker with initial state
    pub fn new(initial_state: EntityState) -> Self {
        let dim = initial_state.properties.len();

        Self {
            state: initial_state,
            transition_matrix: identity_matrix(dim),
            process_noise: diagonal_matrix(dim, 0.01), // Small process noise
            observation_matrix: identity_matrix(dim),
            last_update: Instant::now(),
            anomaly_threshold: 3.0, // 3σ threshold
            permanence_timeout: Duration::from_secs(60),
            anomalies: Vec::new(),
        }
    }

    /// Create tracker with custom config (e.g. custom F matrix).
    pub fn with_config(initial_state: EntityState, config: EntityConfig) -> Self {
        let mut tracker = Self::new(initial_state);
        if let Some(f) = config.f_matrix {
            tracker.transition_matrix = f;
        }
        tracker
    }

    /// Predict state N steps into future
    pub fn predict(&self, steps: usize) -> Result<EntityState, String> {
        let mut state = self.state.clone();

        for _ in 0..steps {
            // x' = F × x
            state.properties = matrix_vector_multiply(&self.transition_matrix, &state.properties)?;

            // P' = F × P × F^T + Q
            let fp = matrix_multiply(&self.transition_matrix, &state.covariance)?;
            let fpft = matrix_multiply(&fp, &matrix_transpose(&self.transition_matrix))?;
            state.covariance = matrix_add(&fpft, &self.process_noise)?;
        }

        Ok(state)
    }

    /// Update state with new observation (Kalman update)
    pub fn update(&mut self, observation: EntityObservation) -> Result<(), String> {
        // Check entity permanence
        let time_since_update = observation.timestamp.duration_since(self.last_update);
        if time_since_update > self.permanence_timeout {
            // Entity lost - rely on prediction only
            let steps = (time_since_update.as_secs() / self.permanence_timeout.as_secs()) as usize;
            self.state = self.predict(steps)?;
        }

        // Innovation: y = z - H×x
        let hx = matrix_vector_multiply(&self.observation_matrix, &self.state.properties)?;
        let innovation: Vec<f64> = observation
            .measured_properties
            .iter()
            .zip(hx.iter())
            .map(|(z, hx_i)| z - hx_i)
            .collect();

        // Innovation covariance: S = H×P×H^T + R
        let hp = matrix_multiply(&self.observation_matrix, &self.state.covariance)?;
        let hpht = matrix_multiply(&hp, &matrix_transpose(&self.observation_matrix))?;
        let innovation_cov = matrix_add(&hpht, &observation.measurement_noise)?;

        // Anomaly detection: Mahalanobis distance d² = y^T × S^-1 × y
        let inv_s = matrix_inverse(&innovation_cov)?;
        let mahalanobis_sq = quadratic_form(&innovation, &inv_s)?;
        let mahalanobis = mahalanobis_sq.sqrt();

        if mahalanobis > self.anomaly_threshold {
            self.anomalies.push(Anomaly {
                severity: mahalanobis,
                threshold: self.anomaly_threshold,
                description: format!(
                    "Observation deviates by {:.2}σ (threshold: {:.2}σ)",
                    mahalanobis, self.anomaly_threshold
                ),
            });
        }

        // Kalman gain: K = P×H^T × S^-1
        let pht = matrix_multiply(
            &self.state.covariance,
            &matrix_transpose(&self.observation_matrix),
        )?;
        let kalman_gain = matrix_multiply(&pht, &inv_s)?;

        // State update: x = x + K×y
        let ky = matrix_vector_multiply(&kalman_gain, &innovation)?;
        self.state.properties = self
            .state
            .properties
            .iter()
            .zip(ky.iter())
            .map(|(x, ky_i)| x + ky_i)
            .collect();

        // Covariance update (Joseph form): P = (I - K×H)×P×(I - K×H)^T + K×R×K^T
        let kh = matrix_multiply(&kalman_gain, &self.observation_matrix)?;
        let i_minus_kh = matrix_subtract(&identity_matrix(self.state.properties.len()), &kh)?;

        // Term 1: (I - KH) * P * (I - KH)^T
        let term1_part1 = matrix_multiply(&i_minus_kh, &self.state.covariance)?;
        let i_minus_kh_t = matrix_transpose(&i_minus_kh);
        let term1 = matrix_multiply(&term1_part1, &i_minus_kh_t)?;

        // Term 2: K * R * K^T
        let kr = matrix_multiply(&kalman_gain, &observation.measurement_noise)?;
        let k_t = matrix_transpose(&kalman_gain);
        let term2 = matrix_multiply(&kr, &k_t)?;

        // P = term1 + term2
        let mut p_new = matrix_add(&term1, &term2)?;

        // Symmetrization: P = (P + P^T) / 2 to prevent float drift
        let n = p_new.len();
        for i in 0..n {
            for j in 0..i {
                let avg = (p_new[i][j] + p_new[j][i]) * 0.5;
                p_new[i][j] = avg;
                p_new[j][i] = avg;
            }
        }
        self.state.covariance = p_new;

        self.last_update = observation.timestamp;

        Ok(())
    }

    /// Get current state estimate
    pub fn get_state(&self) -> EntityState {
        self.state.clone()
    }

    /// Get detected anomalies
    pub fn get_anomalies(&self) -> Vec<Anomaly> {
        self.anomalies.clone()
    }

    /// Clear anomaly history
    pub fn clear_anomalies(&mut self) {
        self.anomalies.clear();
    }

    /// Check if entity is stale (no updates within permanence timeout)
    pub fn is_stale(&self) -> bool {
        self.last_update.elapsed() > self.permanence_timeout
    }
}

// ============================================================================
// Matrix Operations (Basic Linear Algebra)
// ============================================================================

fn identity_matrix(dim: usize) -> Vec<Vec<f64>> {
    let mut mat = vec![vec![0.0; dim]; dim];
    for i in 0..dim {
        mat[i][i] = 1.0;
    }
    mat
}

fn diagonal_matrix(dim: usize, value: f64) -> Vec<Vec<f64>> {
    let mut mat = vec![vec![0.0; dim]; dim];
    for i in 0..dim {
        mat[i][i] = value;
    }
    mat
}

fn matrix_transpose(mat: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = mat.len();
    let cols = mat[0].len();
    let mut transposed = vec![vec![0.0; rows]; cols];

    for i in 0..rows {
        for j in 0..cols {
            transposed[j][i] = mat[i][j];
        }
    }

    transposed
}

fn matrix_vector_multiply(mat: &[Vec<f64>], vec: &[f64]) -> Result<Vec<f64>, String> {
    if mat[0].len() != vec.len() {
        return Err("Matrix-vector dimension mismatch".to_string());
    }

    Ok(mat
        .iter()
        .map(|row| row.iter().zip(vec.iter()).map(|(a, b)| a * b).sum())
        .collect())
}

fn matrix_multiply(a: &[Vec<f64>], b: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    if a[0].len() != b.len() {
        return Err("Matrix dimension mismatch".to_string());
    }

    let rows = a.len();
    let cols = b[0].len();
    let inner = b.len();

    let mut result = vec![vec![0.0; cols]; rows];

    for i in 0..rows {
        for j in 0..cols {
            for k in 0..inner {
                result[i][j] += a[i][k] * b[k][j];
            }
        }
    }

    Ok(result)
}

fn matrix_add(a: &[Vec<f64>], b: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    if a.len() != b.len() || a[0].len() != b[0].len() {
        return Err("Matrix dimension mismatch".to_string());
    }

    Ok(a.iter()
        .zip(b.iter())
        .map(|(row_a, row_b)| row_a.iter().zip(row_b.iter()).map(|(x, y)| x + y).collect())
        .collect())
}

fn matrix_subtract(a: &[Vec<f64>], b: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    if a.len() != b.len() || a[0].len() != b[0].len() {
        return Err("Matrix dimension mismatch".to_string());
    }

    Ok(a.iter()
        .zip(b.iter())
        .map(|(row_a, row_b)| row_a.iter().zip(row_b.iter()).map(|(x, y)| x - y).collect())
        .collect())
}

fn matrix_inverse(mat: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, String> {
    let n = mat.len();
    if n != mat[0].len() {
        return Err("Matrix must be square".to_string());
    }

    // Simple 2x2 inverse for now
    if n == 2 {
        let det = mat[0][0] * mat[1][1] - mat[0][1] * mat[1][0];
        if det.abs() < 1e-10 {
            return Err("Matrix is singular".to_string());
        }
        return Ok(vec![
            vec![mat[1][1] / det, -mat[0][1] / det],
            vec![-mat[1][0] / det, mat[0][0] / det],
        ]);
    }

    // Gaussian elimination for larger matrices
    let mut aug = mat.to_vec();
    let mut inv = identity_matrix(n);

    // Forward elimination
    for i in 0..n {
        // Find pivot
        let mut max_row = i;
        for k in (i + 1)..n {
            if aug[k][i].abs() > aug[max_row][i].abs() {
                max_row = k;
            }
        }

        aug.swap(i, max_row);
        inv.swap(i, max_row);

        let pivot = aug[i][i];
        if pivot.abs() < 1e-10 {
            return Err("Matrix is singular".to_string());
        }

        // Scale row
        for j in 0..n {
            aug[i][j] /= pivot;
            inv[i][j] /= pivot;
        }

        // Eliminate column
        for k in 0..n {
            if k != i {
                let factor = aug[k][i];
                for j in 0..n {
                    aug[k][j] -= factor * aug[i][j];
                    inv[k][j] -= factor * inv[i][j];
                }
            }
        }
    }

    Ok(inv)
}

fn quadratic_form(vec: &[f64], mat: &[Vec<f64>]) -> Result<f64, String> {
    // Compute v^T × M × v
    let mv = matrix_vector_multiply(mat, vec)?;
    Ok(vec.iter().zip(mv.iter()).map(|(v, mv)| v * mv).sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_matrix() {
        let identity = identity_matrix(3);
        assert_eq!(identity[0][0], 1.0);
        assert_eq!(identity[1][1], 1.0);
        assert_eq!(identity[2][2], 1.0);
        assert_eq!(identity[0][1], 0.0);
    }

    #[test]
    fn test_matrix_transpose() {
        let mat = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]];
        let transposed = matrix_transpose(&mat);
        assert_eq!(transposed.len(), 3);
        assert_eq!(transposed[0].len(), 2);
        assert_eq!(transposed[0][0], 1.0);
        assert_eq!(transposed[1][0], 2.0);
        assert_eq!(transposed[2][1], 6.0);
    }

    #[test]
    fn test_matrix_vector_multiply() {
        let mat = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let vec = vec![5.0, 6.0];
        let result = matrix_vector_multiply(&mat, &vec).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0], 17.0); // 1*5 + 2*6
        assert_eq!(result[1], 39.0); // 3*5 + 4*6
    }

    #[test]
    fn test_matrix_multiply() {
        let a = vec![vec![1.0, 2.0], vec![3.0, 4.0]];
        let b = vec![vec![5.0, 6.0], vec![7.0, 8.0]];
        let result = matrix_multiply(&a, &b).unwrap();

        assert_eq!(result[0][0], 19.0); // 1*5 + 2*7
        assert_eq!(result[0][1], 22.0); // 1*6 + 2*8
        assert_eq!(result[1][0], 43.0); // 3*5 + 4*7
        assert_eq!(result[1][1], 50.0); // 3*6 + 4*8
    }

    #[test]
    fn test_matrix_inverse_2x2() {
        let mat = vec![vec![4.0, 7.0], vec![2.0, 6.0]];
        let inv = matrix_inverse(&mat).unwrap();

        // Verify A × A^-1 = I
        let product = matrix_multiply(&mat, &inv).unwrap();
        assert!((product[0][0] - 1.0).abs() < 1e-6);
        assert!((product[1][1] - 1.0).abs() < 1e-6);
        assert!(product[0][1].abs() < 1e-6);
        assert!(product[1][0].abs() < 1e-6);
    }

    #[test]
    fn test_new_entity_tracker() {
        let initial_state = EntityState {
            properties: vec![1.0, 2.0, 3.0],
            covariance: identity_matrix(3),
        };

        let tracker = EntityTracker::new(initial_state);
        assert_eq!(tracker.get_state().properties.len(), 3);
    }

    #[test]
    fn test_predict_no_motion() {
        let initial_state = EntityState {
            properties: vec![1.0, 2.0],
            covariance: identity_matrix(2),
        };

        let tracker = EntityTracker::new(initial_state);
        let predicted = tracker.predict(1).unwrap();

        // With identity transition matrix, state shouldn't change much
        assert!((predicted.properties[0] - 1.0).abs() < 0.1);
        assert!((predicted.properties[1] - 2.0).abs() < 0.1);

        // Covariance should increase (uncertainty grows)
        assert!(predicted.covariance[0][0] > 1.0);
    }

    #[test]
    fn test_update_reduces_uncertainty() {
        let initial_state = EntityState {
            properties: vec![0.0, 0.0],
            covariance: vec![vec![10.0, 0.0], vec![0.0, 10.0]], // High initial uncertainty
        };

        let mut tracker = EntityTracker::new(initial_state);

        let observation = EntityObservation {
            measured_properties: vec![1.0, 1.0],
            measurement_noise: vec![vec![0.1, 0.0], vec![0.0, 0.1]], // Low noise
            timestamp: Instant::now(),
        };

        tracker.update(observation).unwrap();

        // State should move toward observation
        assert!((tracker.get_state().properties[0] - 1.0).abs() < 1.0);

        // Covariance should decrease (uncertainty reduced)
        assert!(tracker.get_state().covariance[0][0] < 10.0);
    }

    #[test]
    fn test_anomaly_detection() {
        let initial_state = EntityState {
            properties: vec![0.0, 0.0],
            covariance: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        };

        let mut tracker = EntityTracker::new(initial_state);

        // Normal observation - no anomaly
        let normal_obs = EntityObservation {
            measured_properties: vec![0.5, 0.5],
            measurement_noise: vec![vec![0.1, 0.0], vec![0.0, 0.1]],
            timestamp: Instant::now(),
        };
        tracker.update(normal_obs).unwrap();
        assert_eq!(tracker.get_anomalies().len(), 0);

        // Anomalous observation - far from expected
        let anomalous_obs = EntityObservation {
            measured_properties: vec![10.0, 10.0], // 10σ away
            measurement_noise: vec![vec![0.1, 0.0], vec![0.0, 0.1]],
            timestamp: Instant::now(),
        };
        tracker.update(anomalous_obs).unwrap();

        let anomalies = tracker.get_anomalies();
        assert!(anomalies.len() > 0);
        assert!(anomalies[0].severity > 3.0);
    }

    #[test]
    fn test_multi_step_prediction() {
        let initial_state = EntityState {
            properties: vec![1.0, 2.0],
            covariance: identity_matrix(2),
        };

        let tracker = EntityTracker::new(initial_state);
        let predicted = tracker.predict(5).unwrap();

        // After 5 steps, uncertainty should have grown significantly
        assert!(predicted.covariance[0][0] > 1.0);
        assert!(predicted.covariance[1][1] > 1.0);
    }

    #[test]
    fn test_is_stale() {
        let initial_state = EntityState {
            properties: vec![1.0],
            covariance: vec![vec![1.0]],
        };

        let tracker = EntityTracker::new(initial_state);

        // Just created, should not be stale
        assert!(!tracker.is_stale());
    }

    #[test]
    fn test_clear_anomalies() {
        let initial_state = EntityState {
            properties: vec![0.0],
            covariance: vec![vec![1.0]],
        };

        let mut tracker = EntityTracker::new(initial_state);

        // Trigger anomaly
        let anomalous_obs = EntityObservation {
            measured_properties: vec![10.0],
            measurement_noise: vec![vec![0.1]],
            timestamp: Instant::now(),
        };
        tracker.update(anomalous_obs).unwrap();

        assert!(tracker.get_anomalies().len() > 0);

        tracker.clear_anomalies();
        assert_eq!(tracker.get_anomalies().len(), 0);
    }

    #[test]
    fn test_with_config_custom_f_matrix() {
        let initial_state = EntityState {
            properties: vec![1.0, 0.0],
            covariance: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        };
        // F = [[1,1],[0,1]] — constant-velocity model
        let f = vec![vec![1.0, 1.0], vec![0.0, 1.0]];
        let config = EntityConfig {
            f_matrix: Some(f.clone()),
        };
        let tracker = EntityTracker::with_config(initial_state, config);
        assert_eq!(tracker.transition_matrix, f);
        // One-step prediction: [1,0] -> F*[1,0] = [1,0]
        let pred = tracker.predict(1).unwrap();
        assert!((pred.properties[0] - 1.0).abs() < 1e-9);
        assert!((pred.properties[1] - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_with_config_default_uses_identity() {
        let initial_state = EntityState {
            properties: vec![2.0, 3.0],
            covariance: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        };
        let tracker = EntityTracker::with_config(initial_state, EntityConfig::default());
        let identity = identity_matrix(2);
        assert_eq!(tracker.transition_matrix, identity);
    }
}
