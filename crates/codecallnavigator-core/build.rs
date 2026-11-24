fn main() {
    println!("cargo:rerun-if-changed=src/process_ownership/macos.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        cc::Build::new()
            .file("src/process_ownership/macos.c")
            .compile("ccn_process_observation");
    }
}
