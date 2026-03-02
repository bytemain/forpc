fn main() {
  napi_build::setup();

  // On Windows, the vendored nng library requires explicit linking to the
  // Windows CRT and system libraries.
  #[cfg(target_os = "windows")]
  {
    // Link against Windows system libraries required by nng
    println!("cargo:rustc-link-lib=ws2_32");
    println!("cargo:rustc-link-lib=mswsock");
    println!("cargo:rustc-link-lib=advapi32");

    // For static CRT linking (configured in .cargo/config.toml with +crt-static),
    // we need to link against the static versions of the CRT libraries.
    // The static CRT libraries are: libcmt (C runtime), libucrt (universal CRT),
    // and libvcruntime (VC runtime).
    // Note: When using +crt-static, Rust automatically handles most CRT linking,
    // but we explicitly link libcmt to ensure all C runtime symbols are resolved.
    println!("cargo:rustc-link-lib=static=libcmt");
  }
}
