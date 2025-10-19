fn main() {
    println!("cargo:rustc-check-cfg=cfg(foundry_network_restricted)");

    vergen::EmitBuilder::builder().build_timestamp().git_sha(true).emit().unwrap();

    if std::net::TcpListener::bind(("127.0.0.1", 0)).is_err() {
        println!("cargo:rustc-cfg=foundry_network_restricted");
    }
}
