pub fn spawn_program(program: &str, args: &[&str]) {
    match std::process::Command::new(program)
        .args(args)
        // Don't pass WAYLAND_DEBUG on to children, the added noise makes
        // debugging the window manager itself impractical.
        .env_remove("WAYLAND_DEBUG")
        // .stdin(std::process::Stdio::null())
        // .stdout(std::process::Stdio::inherit())
        // .stderr(std::process::Stdio::inherit())
        .spawn()
    {
        // Ok(_) => println!("planrwm: spawned {program}"),
        Ok(_) => {}
        Err(e) => eprintln!("planarwm: Failed to spawn {program}: {e}"),
    }
}

// TODO: check if this shell closes if wm closes
pub fn spawn_shell(command: &str) {
    match std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .env_remove("WAYLAND_DEBUG")
        .spawn()
    {
        Ok(_) => {}
        Err(e) => eprintln!("planarwm: Failed to run shell command '{command}': {e}"),
    }
}
