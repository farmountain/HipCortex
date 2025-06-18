/// Estimate Shannon entropy of a vector by treating values as absolute weights.
pub fn estimate_entropy(v: &[f32]) -> f32 {
    let sum: f32 = v.iter().map(|x| x.abs()).sum();
    if sum == 0.0 {
        return 0.0;
    }
    let mut ent = 0.0;
    for x in v {
        let p = x.abs() / sum;
        if p > 0.0 {
            ent -= p * p.log2();
        }
    }
    ent
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_basic() {
        let v = vec![1.0, 1.0, 1.0, 1.0];
        let e = estimate_entropy(&v);
        assert!((e - 2.0).abs() < 1e-3);
    }
}
