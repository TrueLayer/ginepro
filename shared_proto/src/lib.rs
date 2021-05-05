//! Contains share protobuf definitions for the entire project.

pub mod pb {
    tonic::include_proto!("test");

    tonic::include_proto!("echo");
}
