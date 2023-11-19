use std::{io::{Result, Write}, fs::File};


fn main() {
    let pwd = std::env::current_dir().unwrap();
    let mut ancestors = pwd.ancestors();
    let linker_path = ancestors.nth(1).unwrap().join("linker.ld");
    println!("cargo:rerun-if-changed=*/src");
    println!("cargo:rerun-if-changed=./build.rs");
    println!("cargo:rustc-link-args=-fpic -nostartfiles -T{}", linker_path.display());
    insert_info().unwrap();
}


fn insert_info() -> Result<()> {
    let pwd = std::env::current_dir().unwrap();
    let info_path = pwd.join("info.txt");
    let mut f = File::create(pwd.join("src/module_info.asm")).unwrap();

    writeln!(
        f,
r#".align 3
.section .module_info
.global smodule_info
.global emodule_info
smodule_info:
    .incbin {:?}
emodule_info:"#, 
    info_path

    )?;
    Ok(())
}