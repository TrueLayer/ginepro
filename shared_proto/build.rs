//! Compile a grpc service defintion to be exposed and used
//! for testing in this crate and others that want to test
//! tonic functionality.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/test.proto", "proto/echo.proto"], &["proto/"])?;
    Ok(())
}
