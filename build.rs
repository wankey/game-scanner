use std::path::PathBuf;

fn main() {
    let blizzard_proto_path = PathBuf::from("src").join("blizzard").join("proto");
    println!(
        "cargo:rerun-if-changed={}",
        blizzard_proto_path.to_string_lossy()
    );

    println!("Compiling protos...");
    let protoc_path = protoc_bin_vendored::protoc_bin_path().unwrap();
    prost_build::Config::new()
        .btree_map(&["."])
        .out_dir(&blizzard_proto_path)
        .protoc_executable(protoc_path)
        .compile_protos(
            &[blizzard_proto_path.join("product_db.proto")],
            &[blizzard_proto_path.clone()],
        )
        .unwrap();
}
