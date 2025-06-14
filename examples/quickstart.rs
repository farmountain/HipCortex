use hipcortex::semantic_compression::compress_embedding;

fn main() {
    let embedding: Vec<f32> = (0..16).map(|v| v as f32).collect();
    let compressed = compress_embedding(&embedding, 4);
    println!("compressed embedding: {:?}", compressed);
}
