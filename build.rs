use std::{env, path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=resources/icons");
    println!("cargo:rerun-if-changed=resources/panel.gresource.xml");
    let target = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("panel.gresource");
    let status = Command::new("glib-compile-resources")
        .args([
            "--sourcedir=resources",
            "resources/panel.gresource.xml",
            "--target",
        ])
        .arg(target)
        .status()
        .expect("glib-compile-resources is required (libglib2.0-dev-bin)");
    assert!(status.success(), "Failed to embed panel icons");
}
