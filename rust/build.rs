fn main() {
    if std::env::var("FORPC_GENERATE_PROTO").is_ok_and(|v| v == "1" || v == "true") {
        prost_build::Config::new()
            .out_dir("src/gen")
            .compile_protos(
                &["../proto/forpc.proto", "../proto/forpc_test.proto"],
                &["../proto/"],
            )
            .expect("Failed to compile protobuf files");
    }
}
