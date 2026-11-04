#[cfg(not(target_os = "linux"))]
fn main() {}

#[cfg(target_os = "linux")]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    use std::path::PathBuf;
    use std::process::Stdio;
    use std::time::Duration;
    use tokio::io::AsyncReadExt;
    use tokio::process::Command;
    use tokio::signal::unix::{signal, SignalKind};

    // Bind parent death signal before spawning capable sing-box child
    unsafe {
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
    }

    let forward_args: Vec<String> = std::env::args().skip(1).collect();

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut singbox_path = exe_dir.join("sing-box");
    if !singbox_path.exists() {
        if let Ok(entries) = std::fs::read_dir(&exe_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with("sing-box") && !name.contains("supervisor") {
                    singbox_path = entry.path();
                    break;
                }
            }
        }
    }

    let mut sigterm = signal(SignalKind::terminate()).expect("failed to register SIGTERM handler");
    let mut sigint = signal(SignalKind::interrupt()).expect("failed to register SIGINT handler");
    let mut sigchld = signal(SignalKind::child()).expect("failed to register SIGCHLD handler");

    let mut child = match Command::new(&singbox_path)
        .args(&forward_args)
        .env("ENABLE_DEPRECATED_GEOSITE", "true")
        .env("ENABLE_DEPRECATED_GEOIP", "true")
        .env("ENABLE_DEPRECATED_LEGACY_DNS_FAKEIP_OPTIONS", "true")
        .env("ENABLE_DEPRECATED_LEGACY_DNS_SERVERS", "true")
        .env("ENABLE_DEPRECATED_OUTBOUND_DNS_RULE_ITEM", "true")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("xlink-linux-supervisor: failed to spawn sing-box: {}", e);
            std::process::exit(1);
        }
    };

    let child_pid = child.id().unwrap_or(0) as i32;
    let mut stdin = tokio::io::stdin();
    let mut stdin_buf = [0u8; 64];

    let terminate_child = |pid: i32| {
        if pid > 0 {
            unsafe {
                libc::kill(pid, libc::SIGTERM);
            }
        }
    };

    let reap_zombies = || unsafe {
        let mut status: libc::c_int = 0;
        while libc::waitpid(-1, &mut status, libc::WNOHANG) > 0 {}
    };

    loop {
        tokio::select! {
            status_res = child.wait() => {
                reap_zombies();
                let code = status_res.ok().and_then(|s| s.code()).unwrap_or(0);
                std::process::exit(code);
            }
            read_res = stdin.read(&mut stdin_buf) => {
                if matches!(read_res, Ok(0) | Err(_)) {
                    terminate_child(child_pid);
                    break;
                }
            }
            _ = sigterm.recv() => {
                terminate_child(child_pid);
                break;
            }
            _ = sigint.recv() => {
                terminate_child(child_pid);
                break;
            }
            _ = sigchld.recv() => {
                reap_zombies();
            }
        }
    }

    // Wait up to 3 seconds for sing-box to exit gracefully before escalating to SIGKILL
    match tokio::time::timeout(Duration::from_secs(3), child.wait()).await {
        Ok(Ok(status)) => {
            reap_zombies();
            std::process::exit(status.code().unwrap_or(0));
        }
        _ => {
            if child_pid > 0 {
                unsafe {
                    libc::kill(child_pid, libc::SIGKILL);
                }
            }
            let _ = child.wait().await;
            reap_zombies();
            std::process::exit(0);
        }
    }
}
