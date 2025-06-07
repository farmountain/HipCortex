fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "grpc-server")]
    {
        tonic_build::configure()
            .build_client(true)
            .compile(&["proto/memory.proto"], &["proto"])?;
    }
    Ok(())
}
