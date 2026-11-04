#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use std::io::Read;
    use std::path::PathBuf;
    use std::process::{Command, Stdio};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    let raw_args: Vec<String> = std::env::args().skip(1).collect();
    let mut forward_args = Vec::new();
    let mut parent_pid: Option<i32> = None;

    let mut i = 0;
    while i < raw_args.len() {
        if raw_args[i] == "--parent-pid" {
            if let Some(pid_str) = raw_args.get(i + 1) {
                parent_pid = pid_str.parse::<i32>().ok();
                i += 2;
                continue;
            }
        } else if let Some(pid_str) = raw_args[i].strip_prefix("--parent-pid=") {
            parent_pid = pid_str.parse::<i32>().ok();
            i += 1;
            continue;
        }
        forward_args.push(raw_args[i].clone());
        i += 1;
    }

    // Pre-verify parent PID via kqueue EVFILT_PROC / NOTE_EXIT if provided
    let kq_fd = if let Some(ppid) = parent_pid {
        unsafe {
            let kq = libc::kqueue();
            if kq >= 0 {
                let changelist = libc::kevent {
                    ident: ppid as usize,
                    filter: libc::EVFILT_PROC,
                    flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_ONESHOT,
                    fflags: libc::NOTE_EXIT,
                    data: 0,
                    udata: std::ptr::null_mut(),
                };
                let rc = libc::kevent(
                    kq,
                    &changelist,
                    1,
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                );
                if rc < 0 {
                    let err = *libc::__error();
                    libc::close(kq);
                    if err == libc::ESRCH {
                        // Parent already exited before registration; abort without launching sing-box
                        std::process::exit(0);
                    }
                    None
                } else {
                    Some(kq)
                }
            } else {
                None
            }
        }
    } else {
        None
    };

    // Resolve sing-box binary path: inspect current_exe parent first, fallback to helper dir
    let singbox_path = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.join("sing-box")))
        .filter(|p| p.exists())
        .unwrap_or_else(|| PathBuf::from("/Library/Application Support/X-Link/helpers/sing-box"));

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
            eprintln!("xlink-darwin-supervisor: failed to spawn sing-box: {}", e);
            std::process::exit(1);
        }
    };

    let child_pid = child.id() as i32;
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Watch stdin EOF (primary heartbeat trigger across sudo boundary)
    {
        let stop_flag_stdin = Arc::clone(&stop_flag);
        std::thread::spawn(move || {
            let mut stdin = std::io::stdin();
            let mut buf = [0u8; 64];
            loop {
                match stdin.read(&mut buf) {
                    Ok(0) | Err(_) => {
                        stop_flag_stdin.store(true, Ordering::SeqCst);
                        unsafe {
                            libc::kill(child_pid, libc::SIGTERM);
                        }
                        break;
                    }
                    Ok(_) => {}
                }
            }
        });
    }

    // Watch kqueue EVFILT_PROC NOTE_EXIT on parent PID
    if let Some(kq) = kq_fd {
        let stop_flag_kq = Arc::clone(&stop_flag);
        std::thread::spawn(move || {
            let mut event: libc::kevent = unsafe { std::mem::zeroed() };
            let rc = unsafe {
                libc::kevent(
                    kq,
                    std::ptr::null(),
                    0,
                    &mut event,
                    1,
                    std::ptr::null(),
                )
            };
            unsafe {
                libc::close(kq);
            }
            if rc > 0 {
                stop_flag_kq.store(true, Ordering::SeqCst);
                unsafe {
                    libc::kill(child_pid, libc::SIGTERM);
                }
            }
        });
    }

    // Poll child completion and enforce 3s SIGKILL escalation after stop_flag triggers
    let mut kill_deadline: Option<Instant> = None;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                std::process::exit(status.code().unwrap_or(0));
            }
            Ok(None) => {
                if stop_flag.load(Ordering::SeqCst) {
                    let deadline =
                        *kill_deadline.get_or_insert_with(|| Instant::now() + Duration::from_secs(3));
                    if Instant::now() >= deadline {
                        unsafe {
                            libc::kill(child_pid, libc::SIGKILL);
                        }
                        let _ = child.wait();
                        std::process::exit(0);
                    }
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                std::process::exit(1);
            }
        }
    }
}
