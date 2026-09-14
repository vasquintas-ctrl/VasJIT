use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=jit/");
    println!("cargo:rerun-if-changed=include/");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let obj = out.join("vasjit_cpp.o");
    // Compile all cpp into one translation unit via linking objects
    let sources = ["jit/host_caps.cpp", "jit/strategy.cpp", "jit/emit.cpp"];
    let mut objs = Vec::new();
    for src in &sources {
        let name = PathBuf::from(src).file_stem().unwrap().to_string_lossy().to_string();
        let o = out.join(format!("{name}.o"));
        let status = Command::new("g++")
            .args(["-std=c++17", "-O2", "-fPIC", "-c", src, "-Iinclude", "-Ijit", "-o"])
            .arg(&o)
            .status()
            .expect("g++");
        assert!(status.success(), "g++ failed on {src}");
        objs.push(o);
    }
    let lib = out.join("libvasjit_cpp.a");
    let mut cmd = Command::new("ar");
    cmd.arg("rcs").arg(&lib);
    for o in &objs { cmd.arg(o); }
    assert!(cmd.status().expect("ar").success());
    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=vasjit_cpp");
    println!("cargo:rustc-link-lib=stdc++");
    let _ = obj;
}
