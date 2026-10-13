//! Records the build date and time like the C++ `__DATE__ " at " __TIME__` ("Oct 10 2026 at 12:34:56"),
//! shown on the console banner as "Compiled on ...".

fn main() {
    let stamp = std::process::Command::new("date")
        .arg("+%b %e %Y at %H:%M:%S")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown date".to_string());
    println!("cargo:rustc-env=UPLINK_BUILD_DATE={stamp}");
}
