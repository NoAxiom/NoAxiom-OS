use std::{
    fs::{read_dir, File},
    io::{Result, Write},
};

fn main() {
    println!("cargo:rerun-if-changed=../../NoAxiom-OS-User/apps/");
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    insert_app_data().unwrap();
}

static TARGET_PATH: &str = "../NoAxiom-OS-User/bin/";
static FINAL_TARGET_PATH: &str = "../NoAxiom-OS-Test/official/final-bin/";

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_apps.S").unwrap();
    let mut apps: Vec<(String, String)> = read_dir("../../NoAxiom-OS-User/bin")?
        .filter_map(|entry| entry.ok())
        .map(|entry| {
            (
                entry.file_name().into_string().unwrap(),
                TARGET_PATH.to_string(),
            )
        })
        .collect();
    apps.sort();
    let mut final_apps: Vec<(String, String)> =
        read_dir("../../NoAxiom-OS-Test/official/final-bin")?
            .filter_map(|entry| entry.ok())
            .map(|entry| {
                (
                    entry.file_name().into_string().unwrap(),
                    FINAL_TARGET_PATH.to_string(),
                )
            })
            .collect();
    final_apps.sort();

    apps.extend(final_apps);

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

    for (app, _) in apps.iter() {
        writeln!(f, r#"    .quad {}_start"#, app)?;
        writeln!(f, r#"    .quad {}_end"#, app)?;
    }

    writeln!(
        f,
        r#"
    .global _app_names
_app_names:"#
    )?;
    for (app, _) in apps.iter() {
        writeln!(f, r#"    .string "{}""#, app)?;
    }

    for (idx, (app, target)) in apps.iter().enumerate() {
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
            app, target
        )?;
    }
    Ok(())
}
