/// Normalize a vector to unit L2 norm.
pub fn l2_normalize(mut v: Vec<f32>) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for val in v.iter_mut() {
            *val /= norm;
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn normalizes() {
        let out = l2_normalize(vec![3.0, 4.0]);
        assert_relative_eq!(out[0] * out[0] + out[1] * out[1], 1.0, epsilon = 1e-6);
    }
}
