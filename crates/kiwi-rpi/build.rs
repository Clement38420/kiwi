use std::process::Command;

fn main() {
    // Instruct Cargo to rerun this build script if any of the following files change
    println!("cargo:rerun-if-changed=../../gcs/src");
    println!("cargo:rerun-if-changed=../../gcs/index.html");
    println!("cargo:rerun-if-changed=../../gcs/package.json");

    let status = Command::new("pnpm")
        .args(["--dir", "../../gcs", "build-only"])
        .status()
        .expect("Failed to run pnpm build for gcs");

    assert!(status.success(), "GCS frontend build failed!");
}
