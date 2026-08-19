//! Continuous dynamics layer: VectorField trait + RK4 integrator + diagonal covariance tracking.
//!
//! Chain-of-thought: Existing engine is purely discrete (MCTS + Dirichlet + Kalman).
//! ContinuousDynamics adds a residual continuous flow that integrates entity state
//! with RK4 between discrete steps, tracking diagonal covariance growth and halting
//! when uncertainty exceeds max_covariance.

use uuid::Uuid;

// ─── Public trait ─────────────────────────────────────────────────────────────

/// Context injected into each vector field evaluation.
pub struct DynamicsContext<'a> {
    /// (entity_id, state_vec) pairs from WorldModelEnhanced at call time.
    pub entity_states: &'a [(Uuid, Vec<f64>)],
    /// Current resource vector from SelfModel (empty slice if unavailable).
    pub resource_vec: &'a [f64],
    /// Current TxLog cursor for provenance.
    pub tx_cursor: u64,
}

/// Differentiable vector field: dstate/dt = eval(t, state, ctx).
pub trait VectorField: Send + Sync {
    fn dim(&self) -> usize;
    fn eval(&self, t: f64, state: &[f64], ctx: &DynamicsContext<'_>) -> Vec<f64>;
}

// ─── Kalman vector field ───────────────────────────────────────────────────────

/// Simplest continuous field: dμ/dt = A·μ with diagonal A.
/// Default: identity diagonal (unit growth per time unit).
pub struct KalmanVectorField {
    diag: Vec<f64>,
}

impl KalmanVectorField {
    /// Unit diagonal (identity).
    pub fn new(dim: usize) -> Self {
        Self { diag: vec![1.0; dim] }
    }
    /// Custom diagonal transition rates.
    pub fn with_diag(diag: Vec<f64>) -> Self {
        Self { diag }
    }
}

impl VectorField for KalmanVectorField {
    fn dim(&self) -> usize {
        self.diag.len()
    }
    fn eval(&self, _t: f64, state: &[f64], _ctx: &DynamicsContext<'_>) -> Vec<f64> {
        state.iter().zip(&self.diag).map(|(s, a)| a * s).collect()
    }
}

// ─── ContinuousDynamics ───────────────────────────────────────────────────────

/// RK4 integrator with diagonal covariance tracking.
/// Halts integration when sigma_norm() exceeds max_covariance.
pub struct ContinuousDynamics {
    pub vector_field: Box<dyn VectorField>,
    pub dt: f64,
    pub max_covariance: f64,
    /// Diagonal covariance Σ_ii (grows monotonically).
    sigma: Vec<f64>,
    halted: bool,
}

impl ContinuousDynamics {
    pub fn new(vector_field: Box<dyn VectorField>, dt: f64, max_covariance: f64) -> Self {
        let dim = vector_field.dim();
        Self {
            vector_field,
            dt,
            max_covariance,
            sigma: vec![1e-4; dim.max(1)],
            halted: false,
        }
    }

    /// Advance state by dt using RK4. Updates diagonal covariance. Returns new state.
    pub fn step(
        &mut self,
        t: f64,
        state: &[f64],
        ctx: &DynamicsContext<'_>,
    ) -> Result<Vec<f64>, String> {
        if self.halted {
            return Err("dynamics halted: max_covariance exceeded".into());
        }
        let dt = self.dt;
        let k1 = self.vector_field.eval(t, state, ctx);
        let s2: Vec<f64> = state.iter().zip(&k1).map(|(s, k)| s + 0.5 * dt * k).collect();
        let k2 = self.vector_field.eval(t + 0.5 * dt, &s2, ctx);
        let s3: Vec<f64> = state.iter().zip(&k2).map(|(s, k)| s + 0.5 * dt * k).collect();
        let k3 = self.vector_field.eval(t + 0.5 * dt, &s3, ctx);
        let s4: Vec<f64> = state.iter().zip(&k3).map(|(s, k)| s + dt * k).collect();
        let k4 = self.vector_field.eval(t + dt, &s4, ctx);

        let new_state: Vec<f64> = state
            .iter()
            .enumerate()
            .map(|(i, s)| s + (dt / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]))
            .collect();

        // Diagonal covariance growth: σ_i += dt * |dstate_i/dt|² (additive noise model)
        for (i, sig) in self.sigma.iter_mut().enumerate() {
            let deriv = k1.get(i).copied().unwrap_or(0.0);
            *sig += dt * deriv * deriv;
        }

        if self.sigma_norm() > self.max_covariance {
            self.halted = true;
        }

        Ok(new_state)
    }

    /// L2 norm of diagonal covariance vector.
    pub fn sigma_norm(&self) -> f64 {
        self.sigma.iter().map(|x| x * x).sum::<f64>().sqrt()
    }

    pub fn is_halted(&self) -> bool {
        self.halted
    }

    /// Reset covariance and halted flag (used on twin sync).
    pub fn reset_covariance(&mut self) {
        let dim = self.sigma.len();
        self.sigma = vec![1e-4; dim];
        self.halted = false;
    }

    pub fn dim(&self) -> usize {
        self.vector_field.dim()
    }
}

impl Clone for ContinuousDynamics {
    fn clone(&self) -> Self {
        Self {
            vector_field: Box::new(KalmanVectorField::with_diag(
                vec![1.0; self.vector_field.dim()],
            )),
            dt: self.dt,
            max_covariance: self.max_covariance,
            sigma: self.sigma.clone(),
            halted: self.halted,
        }
    }
}
