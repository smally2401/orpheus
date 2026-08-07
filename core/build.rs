fn main() {
    println!("cargo:rerun-if-changed=c_src/miniaudio.c");
    println!("cargo:rerun-if-changed=c_src/miniaudio.h");

    cc::Build::new()
    .file("c_src/miniaudio.c")
    .include("c_src")
    .compile("miniaudio");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    match target_os.as_str() {
        "linux" => {
            println!("cargo:rustc-link.lib=pthread");
            println!("cargo:rustc-link.lib=m");
            println!("cargo:rustc-link.lib=dl");
        }
        "macos" => {
            println!("cargo:rustc-link.lib=framework=CoreAudio");
            println!("cargo:rustc-link.lib=framework=AudioToolbox");
            println!("cargo:rustc-link.lib=framework=CoreFoundation");
        }
        "windows" => {
            println!("cargo:rustc-link.lib=winmm");
            println!("cargo:rustc-link.lib=ole32");
        }
        _ => {}
    }
}
