
fn main() {
    let pwd = std::env::current_dir().unwrap();
    let mut ancestors = pwd.ancestors();
    let linker_path = ancestors.nth(1).unwrap().join("linker.ld");
    println!("cargo:rerun-if-changed=*/src");
    println!("cargo:rerun-if-changed=./build.rs");
    println!("cargo:rustc-link-args=-fpic -nostartfiles -T{}", linker_path.display());

}