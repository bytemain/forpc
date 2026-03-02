fn main() {
  napi_build::setup();

  // On Windows, the vendored nng library requires explicit linking to the
  // Windows CRT and system libraries. The nng-sys crate builds nng with static
  // CRT linking (/MT), so we need to ensure the proper libraries are linked.
  #[cfg(target_os = "windows")]
  {
    // Link against Windows system libraries required by nng
    println!("cargo:rustc-link-lib=ws2_32");
    println!("cargo:rustc-link-lib=mswsock");
    println!("cargo:rustc-link-lib=advapi32");
    // The standard C runtime library for functions like strerror
    println!("cargo:rustc-link-lib=ucrt");
    println!("cargo:rustc-link-lib=vcruntime");
  }
}
