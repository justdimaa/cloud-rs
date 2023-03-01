use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    tonic_build::compile_protos("proto/auth.proto")?;
    tonic_build::compile_protos("proto/file.proto")?;
    tonic_build::compile_protos("proto/user.proto")?;
    Ok(())
}
