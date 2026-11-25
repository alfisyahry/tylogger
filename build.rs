use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/resources/tylogger.rc");
    println!("cargo:rerun-if-changed=assets/icons/tylogger.ico");

    let out_dir = match env::var("OUT_DIR") {
        Ok(dir) => dir,
        Err(_) => return,
    };

    let res_output = Path::new(&out_dir).join("tylogger.res");
    let rc_source = Path::new("assets/resources/tylogger.rc");

    if !rc_source.exists() {
        return;
    }

    let rc_candidates = [
        r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22000.0\x64\rc.exe",
        r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\rc.exe",
        r"C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\rc.exe",
        "rc.exe",
    ];

    for candidate in &rc_candidates {
        let status = Command::new(candidate)
            .arg("/fo")
            .arg(&res_output)
            .arg(rc_source)
            .status();

        if let Ok(st) = status {
            if st.success() {
                println!("cargo:rustc-link-arg={}", res_output.display());
                break;
            }
        }
    }
}
