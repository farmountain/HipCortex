pub fn compress_embedding(vec: &[f32], new_dim: usize) -> Vec<f32> {
    if new_dim == 0 || vec.is_empty() {
        return vec![];
    }
    let chunk = (vec.len() as f32 / new_dim as f32).ceil() as usize;
    let mut result = Vec::with_capacity(new_dim);
    for i in 0..new_dim {
        let start = i * chunk;
        if start >= vec.len() {
            result.push(0.0);
            continue;
        }
        let end = ((i + 1) * chunk).min(vec.len());
        let slice = &vec[start..end];
        let sum: f32 = slice.iter().copied().sum();
        result.push(sum / slice.len() as f32);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_basic() {
        let input: Vec<f32> = (0..10).map(|v| v as f32).collect();
        let out = compress_embedding(&input, 2);
        assert_eq!(out.len(), 2);
        assert!(out[0] < out[1]);
    }
}
