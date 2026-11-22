//! The terminal window on Windows.
//!
//! Release builds of the editor, the player and the updater are Windows GUI
//! programs (`windows_subsystem = "windows"`), so opening one from the Start
//! menu or the desktop doesn't open a terminal beside it. A GUI program has
//! no console of its own, though, so `axcenity-editor --version` typed into
//! a terminal would print nothing. [`attach_parent_console`] fixes that: when
//! the program was started from a terminal and its output isn't already
//! going to a pipe or file, it writes to that terminal.

/// On Windows, sends standard output and error to the terminal that started
/// this program, if there is one and they don't already go somewhere. Does
/// nothing on other systems, in debug builds (which keep their own console)
/// or when started from Explorer.
pub fn attach_parent_console() {
    #[cfg(windows)]
    windows::attach_parent_console();
}

/// Starts a command without a terminal window. A GUI program that starts a
/// console tool (`cargo`, `reg`, `tasklist`) would otherwise flash one up.
/// Does nothing on other systems.
pub fn hide_window(command: &mut std::process::Command) -> &mut std::process::Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

/// Tells a fatal error on standard error, and on Windows, when there's no
/// terminal to see it (a release build opened from Explorer), in an error
/// dialog box as well. `title` is the dialog's title.
pub fn tell_error(title: &str, message: &str) {
    eprintln!("{message}");
    #[cfg(windows)]
    windows::error_box_without_console(title, message);
    #[cfg(not(windows))]
    let _ = title;
}

#[cfg(windows)]
#[allow(unsafe_code)]
mod windows {
    // A few kernel32 and user32 calls, declared here instead of adding a
    // Windows bindings crate for them.
    const ATTACH_PARENT_PROCESS: u32 = u32::MAX;
    const STD_OUTPUT_HANDLE: u32 = -11i32 as u32;
    const STD_ERROR_HANDLE: u32 = -12i32 as u32;
    const INVALID_HANDLE_VALUE: isize = -1;
    const MB_OK_ICONERROR: u32 = 0x10;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn GetStdHandle(std_handle: u32) -> isize;
        fn GetConsoleWindow() -> isize;
    }

    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(window: isize, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }

    pub fn error_box_without_console(title: &str, message: &str) {
        let wide = |text: &str| text.encode_utf16().chain([0]).collect::<Vec<u16>>();
        let (title, message) = (wide(title), wide(message));
        // SAFETY: the strings are NUL-terminated UTF-16 that outlive the
        // call, which blocks until the box is closed and keeps no pointer.
        unsafe {
            let err = GetStdHandle(STD_ERROR_HANDLE);
            let redirected = err != 0 && err != INVALID_HANDLE_VALUE;
            if redirected || GetConsoleWindow() != 0 {
                return;
            }
            MessageBoxW(0, message.as_ptr(), title.as_ptr(), MB_OK_ICONERROR);
        }
    }

    pub fn attach_parent_console() {
        // SAFETY: both calls take plain integers and only read or set this
        // process's standard handles; neither keeps a pointer.
        unsafe {
            let out = GetStdHandle(STD_OUTPUT_HANDLE);
            // Already a console, a pipe or a file (the updater reading
            // `--version --json`, or a debug build): leave it.
            if out != 0 && out != INVALID_HANDLE_VALUE {
                return;
            }
            // Fails when there's no parent console (started from Explorer),
            // which is fine.
            AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}
