use nalgebra::DMatrix;

/// Reduce a single embedding using a simple PCA.
/// Assumes data is 1-dimensional vector.
/// Uses sliding window to form matrix then computes SVD.
pub fn pca_reduce(data: &[f32], target_dim: usize) -> Vec<f32> {
    if target_dim == 0 || data.is_empty() {
        return Vec::new();
    }
    if data.len() < target_dim {
        return data.to_vec();
    }
    // Build matrix using overlapping windows
    let rows = data.len() - target_dim + 1;
    let mut mat_data = Vec::with_capacity(rows * target_dim);
    for i in 0..rows {
        for j in 0..target_dim {
            mat_data.push(data[i + j]);
        }
    }
    let mut m = DMatrix::from_row_slice(rows, target_dim, &mat_data);
    // Center columns
    for c in 0..target_dim {
        let mean: f32 = (0..rows).map(|r| m[(r, c)]).sum::<f32>() / rows as f32;
        for r in 0..rows {
            m[(r, c)] -= mean;
        }
    }
    let svd = m.svd(true, true);
    let vt = svd.v_t.unwrap();
    let mut out = vec![0.0; target_dim];
    for i in 0..target_dim {
        let mut val = 0.0;
        for j in 0..target_dim {
            val += data[j] * vt[(i, j)];
        }
        out[i] = val;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn reduces_dimension() {
        let input: Vec<f32> = (0..10).map(|v| v as f32).collect();
        let out = pca_reduce(&input, 3);
        assert_eq!(out.len(), 3);
        let var: f32 = out.iter().map(|v| v * v).sum();
        assert!(var > 0.0);
    }
}
