
fn main() {
    let pwd = std::env::current_dir().unwrap();
    let linker_path = pwd.join("linker.ld");
    println!("cargo:rerun-if-changed=/src/");
    println!("cargo:rustc-link-arg=-T{}", linker_path.display());

}
