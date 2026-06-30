fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &["proto/plate_solver.proto"],
            &["proto"],
        )?;

    tonic_build::configure()
        .build_server(false)
        .build_client(false)
        .compile(
            &["proto/cedar_detect.proto"],
            &["proto"],
        )?;
    Ok(())
}
