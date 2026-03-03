fn main() {
    prost_build::compile_protos(
        &["../proto/forpc.proto", "../proto/forpc_test.proto"],
        &["../proto/"],
    )
    .unwrap();
}
