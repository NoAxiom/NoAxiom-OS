use std::{
    fs::{read_dir, File},
    io::{Result, Write},
};

fn main() {
    println!("cargo:rerun-if-changed=../user/apps/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    insert_app_data().unwrap();
}

static TARGET_PATH: &str = "./NoAxiom/user/bin/";

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_apps.S").unwrap();
    let mut apps: Vec<String> = read_dir("../user/bin")?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();
    apps.sort();

    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len() * 2
    )?;

    for app in apps.iter() {
        writeln!(f, r#"    .quad {}_start"#, app)?;
        writeln!(f, r#"    .quad {}_end"#, app)?;
    }

    writeln!(
        f,
        r#"
    .global _app_names
_app_names:"#
    )?;
    for app in apps.iter() {
        writeln!(f, r#"    .string "{}""#, app)?;
    }

    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(
            f,
            r#"
    .section .data
    .global {0}_start
    .global {0}_end
    .align 3
{0}_start:
    .incbin "{1}{0}"
{0}_end:"#,
            app, TARGET_PATH
        )?;
    }
    Ok(())
}
