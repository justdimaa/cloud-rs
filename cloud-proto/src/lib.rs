pub mod proto {
    tonic::include_proto!("auth");
    tonic::include_proto!("file");
    tonic::include_proto!("user");
}

pub use prost_types;
