//! Whether this computer's firewall lets others reach a host (cohesion
//! pass CW7.4). Only reads: it asks the platform's own tool and explains
//! what it found. The one change, an inbound allow rule for this program
//! and port on Windows, is made only when the person clicks for it, by an
//! elevated PowerShell that Windows asks them to approve; never at start.
//!
//! - Windows: Windows Defender Firewall's inbound rules for this program
//!   (`Get-NetFirewallApplicationFilter`), which is where Defender puts the
//!   Block rules it adds when its first prompt is dismissed.
//! - Linux: `firewalld` (Fedora, openSUSE), asked whether the zone that
//!   Tailscale's and the network's traffic arrives in opens the port, which
//!   it answers without root and counting port ranges (Fedora Workstation
//!   opens 1025-65535); or `ufw` (Ubuntu), whose rules need root to read.
//!   When the port is closed, the command that opens it is given apart from
//!   the text, for the wizard's Copy button.
//! - macOS: the Application Firewall's answer for this program.

use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use serde::Deserialize;

/// What the firewall says about hosting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// An enabled rule lets this program accept connections.
    Allowed,
    /// An enabled rule blocks this program: others can't reach it, and
    /// on Windows a Block rule wins over any Allow rule.
    Blocked { rules: Vec<String> },
    /// The firewall has no rule for it: Windows asks the first time it
    /// listens, and macOS may too.
    NoRule,
    /// No firewall is on, or none was found.
    Off,
    /// The firewall is on and doesn't open the port: others can't connect
    /// until `command`, run in a terminal, opens it.
    Closed {
        firewall: &'static str,
        command: String,
    },
    /// The firewall is on but won't say without administrator rights
    /// whether the port is open; `command` opens it if it isn't.
    MaybeClosed {
        firewall: &'static str,
        command: String,
    },
    /// The firewall couldn't be asked; the text says why.
    Unknown(String),
}

impl Verdict {
    /// One sentence for the setup wizard: what it found and what to do.
    pub fn explain(&self) -> String {
        match self {
            Verdict::Allowed => "The firewall lets Axcenity accept connections.".to_owned(),
            Verdict::Blocked { rules } => format!(
                "The firewall blocks Axcenity ({}), so others can't reach this computer. Click \
                 Allow to replace the block with a rule for Axcenity's port only; Windows asks \
                 you to approve it.",
                rules.join(", ")
            ),
            Verdict::NoRule => "The firewall has no rule for Axcenity yet. The first time you \
                 host, Windows asks: tick Private networks and click Allow, or click Allow \
                 here now."
                .to_owned(),
            Verdict::Off => "No firewall is blocking connections.".to_owned(),
            Verdict::Closed { firewall, .. } => format!(
                "{firewall} is on and doesn't let others reach Axcenity's port. Run this command \
                 in a terminal to open it, then click Check again:"
            ),
            Verdict::MaybeClosed { firewall, .. } => format!(
                "{firewall} is on, and only an administrator can see whether it lets others \
                 reach Axcenity's port. If others can't connect, run this command in a terminal:"
            ),
            Verdict::Unknown(why) => format!("Couldn't ask the firewall: {why}."),
        }
    }

    /// The command that opens the port, to show apart from the text so it
    /// can be copied whole.
    pub fn command(&self) -> Option<&str> {
        match self {
            Verdict::Closed { command, .. } | Verdict::MaybeClosed { command, .. } => Some(command),
            _ => None,
        }
    }

    /// Whether others may fail to reach this computer.
    pub fn is_problem(&self) -> bool {
        matches!(self, Verdict::Blocked { .. } | Verdict::Closed { .. })
    }
}

/// Asks this platform's firewall about `program` (the running editor or
/// server) listening on TCP `port`. Blocks for up to a few seconds: call
/// it off the UI thread.
pub fn check(program: &Path, port: u16) -> Verdict {
    if cfg!(windows) {
        match run(
            Path::new("powershell"),
            &[
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &windows_query(program),
            ],
        ) {
            Ok(json) => parse_windows(&json),
            Err(why) => Verdict::Unknown(why),
        }
    } else if cfg!(target_os = "macos") {
        let tool = Path::new("/usr/libexec/ApplicationFirewall/socketfilterfw");
        let state = run(tool, &["--getglobalstate"]);
        let app = run(tool, &["--getappblocked", &program.to_string_lossy()]);
        match (state, app) {
            (Ok(state), Ok(app)) => parse_macos(&state, &app),
            (Err(why), _) | (_, Err(why)) => Verdict::Unknown(why),
        }
    } else {
        check_linux(port)
    }
}

/// firewalld when it runs, else ufw.
fn check_linux(port: u16) -> Verdict {
    let tool = Path::new("firewall-cmd");
    if let Ok(state) = run(tool, &["--state"])
        && state.trim() == "running"
    {
        // Tailscale's interface is in a zone of its own only if someone put
        // it there; otherwise its traffic, like the network's, meets the
        // default zone, which `--query-port` without `--zone` asks.
        let zone = run(tool, &["--get-zone-of-interface=tailscale0"])
            .ok()
            .map(|zone| zone.trim().to_owned())
            .filter(|zone| !zone.is_empty());
        let zone_arg = zone.as_ref().map(|zone| format!("--zone={zone}"));
        let mut query: Vec<String> = zone_arg.iter().cloned().collect();
        query.push(format!("--query-port={port}/tcp"));
        let query: Vec<&str> = query.iter().map(String::as_str).collect();
        let open = ask(tool, &query);
        let mut info: Vec<&str> = vec!["--info-zone"];
        let default_zone;
        if let Some(zone) = &zone {
            info = vec!["--info-zone", zone];
        } else {
            default_zone = run(tool, &["--get-default-zone"]).unwrap_or_default();
            info.push(default_zone.trim());
        }
        let info = run(tool, &info).unwrap_or_default();
        return parse_firewalld(open, &info, zone.as_deref(), port);
    }
    parse_ufw(run(Path::new("ufw"), &["status"]), port)
}

/// Reads firewalld's answer to `--query-port` (whether it exited well,
/// and what it printed) and the zone's `--info-zone`, whose `target:
/// ACCEPT` (the trusted zone) lets everything in.
pub fn parse_firewalld(
    query: Result<(bool, String), String>,
    info: &str,
    zone: Option<&str>,
    port: u16,
) -> Verdict {
    let accepts_all = info
        .lines()
        .any(|line| line.trim().eq_ignore_ascii_case("target: ACCEPT"));
    match query {
        Ok((_, answer)) if answer.trim() == "yes" => Verdict::Allowed,
        _ if accepts_all => Verdict::Allowed,
        Ok((_, answer)) if answer.trim() == "no" => Verdict::Closed {
            firewall: "firewalld",
            command: firewalld_command(zone, port),
        },
        Ok((_, answer)) => Verdict::Unknown(format!("firewalld answered \"{}\"", answer.trim())),
        Err(why) => Verdict::Unknown(why),
    }
}

/// Opens `port` for good: `--permanent` keeps it after a restart, and the
/// reload applies it now.
pub fn firewalld_command(zone: Option<&str>, port: u16) -> String {
    let zone = zone
        .map(|zone| format!(" --zone={zone}"))
        .unwrap_or_default();
    format!(
        "sudo firewall-cmd --permanent{zone} --add-port={port}/tcp && sudo firewall-cmd --reload"
    )
}

/// Reads `ufw status`: off, a rule allowing `port`, none, or no answer
/// without root.
pub fn parse_ufw(status: Result<String, String>, port: u16) -> Verdict {
    let command = format!("sudo ufw allow {port}/tcp");
    match status {
        Ok(status) if status.contains("Status: active") => {
            let allows = status.lines().any(|line| {
                let mut words = line.split_whitespace();
                let target = words.next().unwrap_or("");
                let target = target.strip_suffix("/tcp").unwrap_or(target);
                target == port.to_string() && line.contains("ALLOW")
            });
            if allows {
                Verdict::Allowed
            } else {
                Verdict::Closed {
                    firewall: "ufw",
                    command,
                }
            }
        }
        Ok(_) => Verdict::Off,
        Err(why) if why.contains("root") => Verdict::MaybeClosed {
            firewall: "ufw",
            command,
        },
        Err(_) => Verdict::Off,
    }
}

/// The PowerShell that lists this program's inbound rules as JSON, with
/// their state as words (not numbers, which differ between versions).
pub fn windows_query(program: &Path) -> String {
    format!(
        "Get-NetFirewallApplicationFilter -Program '{}' -ErrorAction SilentlyContinue | \
         Get-NetFirewallRule | Where-Object {{ $_.Direction -eq 'Inbound' }} | \
         Select-Object DisplayName, @{{n='Enabled';e={{$_.Enabled.ToString()}}}}, \
         @{{n='Action';e={{$_.Action.ToString()}}}} | ConvertTo-Json -Compress",
        quoted(program)
    )
}

/// The PowerShell run elevated by [`allow_on_windows`]: removes this
/// program's inbound Block rules and adds one Allow rule for TCP `port`
/// on private and domain networks, for this program only.
pub fn windows_allow_script(program: &Path, port: u16) -> String {
    let program = quoted(program);
    format!(
        "Get-NetFirewallApplicationFilter -Program '{program}' -ErrorAction SilentlyContinue | \
         Get-NetFirewallRule | Where-Object {{ $_.Direction -eq 'Inbound' -and $_.Action -eq \
         'Block' }} | Remove-NetFirewallRule; New-NetFirewallRule -DisplayName 'Axcenity \
         Collaborate (port {port})' -Direction Inbound -Program '{program}' -Protocol TCP \
         -LocalPort {port} -Action Allow -Profile Private,Domain"
    )
}

/// Starts [`windows_allow_script`] in an elevated PowerShell, which
/// Windows asks the person to approve. Returns at once; check again after.
pub fn allow_on_windows(program: &Path, port: u16) -> Result<(), String> {
    if !cfg!(windows) {
        return Err("only Windows' firewall is changed from here".to_owned());
    }
    // `-EncodedCommand` carries the script whole, so no path needs
    // quoting twice.
    let elevate = format!(
        "Start-Process powershell -Verb RunAs -Wait -ArgumentList \
         '-NoProfile','-EncodedCommand','{}'",
        encoded_command(&windows_allow_script(program, port))
    );
    crate::tailscale::command(Path::new("powershell"))
        .args(["-NoProfile", "-Command", &elevate])
        .stdin(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("couldn't start PowerShell: {e}"))
}

/// `script` as PowerShell's `-EncodedCommand` wants it: UTF-16LE, then
/// base64.
pub fn encoded_command(script: &str) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// A path inside single quotes in PowerShell: a quote is doubled.
fn quoted(program: &Path) -> String {
    program.to_string_lossy().replace('\'', "''")
}

#[derive(Deserialize)]
struct WindowsRule {
    #[serde(rename = "DisplayName", default)]
    name: String,
    #[serde(rename = "Enabled", default)]
    enabled: String,
    #[serde(rename = "Action", default)]
    action: String,
}

/// Reads [`windows_query`]'s JSON: nothing, one rule, or a list.
pub fn parse_windows(json: &str) -> Verdict {
    let json = json.trim();
    if json.is_empty() {
        return Verdict::NoRule;
    }
    let rules: Vec<WindowsRule> = match serde_json::from_str::<Vec<WindowsRule>>(json) {
        Ok(rules) => rules,
        Err(_) => match serde_json::from_str::<WindowsRule>(json) {
            Ok(rule) => vec![rule],
            Err(error) => return Verdict::Unknown(format!("its answer wasn't readable ({error})")),
        },
    };
    let enabled: Vec<&WindowsRule> = rules.iter().filter(|r| r.enabled == "True").collect();
    let blocked: Vec<String> = enabled
        .iter()
        .filter(|r| r.action == "Block")
        .map(|r| r.name.clone())
        .collect();
    if !blocked.is_empty() {
        Verdict::Blocked { rules: blocked }
    } else if enabled.iter().any(|r| r.action == "Allow") {
        Verdict::Allowed
    } else {
        Verdict::NoRule
    }
}

/// Reads `socketfilterfw --getglobalstate` and `--getappblocked`.
pub fn parse_macos(state: &str, app: &str) -> Verdict {
    if state.contains("disabled") || state.contains("State = 0") {
        return Verdict::Off;
    }
    let app = app.to_lowercase();
    if app.contains("is blocked") || app.contains("are blocked") {
        Verdict::Blocked {
            rules: vec!["Application Firewall".to_owned()],
        }
    } else if app.contains("permitted") || app.contains("allowed") {
        Verdict::Allowed
    } else {
        Verdict::NoRule
    }
}

/// Runs `program` with `args` for at most five seconds and returns what
/// it printed, or why it failed.
fn run(program: &Path, args: &[&str]) -> Result<String, String> {
    match ask(program, args)? {
        (true, stdout) => Ok(stdout),
        (false, _) => Err(LAST_STDERR.with(|e| e.take())),
    }
}

std::thread_local! {
    /// What the last [`ask`] on this thread printed as errors.
    static LAST_STDERR: std::cell::Cell<String> = const { std::cell::Cell::new(String::new()) };
}

/// Runs `program` with `args` for at most five seconds: whether it exited
/// well and what it printed. A question like `--query-port` answers "no"
/// with a failing exit.
fn ask(program: &Path, args: &[&str]) -> Result<(bool, String), String> {
    let name = program.display();
    let mut child = crate::tailscale::command(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("couldn't run {name}: {e}"))?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > Duration::from_secs(5) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("{name} didn't answer within 5 s"));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => return Err(format!("couldn't run {name}: {e}")),
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("couldn't read {name}'s answer: {e}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    LAST_STDERR.with(|e| e.set(stderr));
    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_defender_block_rule_is_found_and_wins_over_allow() {
        let json = r#"[{"DisplayName":"axcenity-collab.exe","Enabled":"True","Action":"Block"},
            {"DisplayName":"axcenity-collab.exe","Enabled":"True","Action":"Allow"}]"#;
        assert_eq!(
            parse_windows(json),
            Verdict::Blocked {
                rules: vec!["axcenity-collab.exe".to_owned()]
            }
        );
        assert!(parse_windows(json).is_problem());
    }

    #[test]
    fn one_allow_rule_no_rule_and_disabled_rules() {
        let one = r#"{"DisplayName":"Axcenity","Enabled":"True","Action":"Allow"}"#;
        assert_eq!(parse_windows(one), Verdict::Allowed);
        assert_eq!(parse_windows(""), Verdict::NoRule);
        let off = r#"{"DisplayName":"Old","Enabled":"False","Action":"Block"}"#;
        assert_eq!(parse_windows(off), Verdict::NoRule);
        assert!(matches!(parse_windows("{oops"), Verdict::Unknown(_)));
    }

    #[test]
    fn the_allow_script_is_scoped_to_the_program_and_port() {
        let program = Path::new(r"C:\Users\O'Neil\Axcenity\axcenity-editor.exe");
        let script = windows_allow_script(program, 47800);
        assert!(script.contains(r"-Program 'C:\Users\O''Neil\Axcenity\axcenity-editor.exe'"));
        assert!(script.contains("-LocalPort 47800"));
        assert!(script.contains("-Profile Private,Domain"));
        // Only this program's inbound Block rules are removed.
        assert!(script.contains("$_.Action -eq 'Block'"));
        assert!(script.contains("$_.Direction -eq 'Inbound'"));
        assert!(windows_query(program).contains("ConvertTo-Json"));
    }

    #[test]
    fn scripts_are_encoded_as_powershell_reads_them() {
        // `powershell -EncodedCommand` of "dir": UTF-16LE "d\0i\0r\0".
        assert_eq!(encoded_command("dir"), "ZABpAHIA");
        assert_eq!(encoded_command("ab"), "YQBiAA==");
    }

    #[test]
    fn macos_and_linux_answers() {
        assert_eq!(
            parse_macos("Firewall is disabled. (State = 0)", ""),
            Verdict::Off
        );
        assert!(
            parse_macos(
                "Firewall is enabled. (State = 1)",
                "The application /Applications/Axcenity.app is blocked from incoming connections"
            )
            .is_problem()
        );
        assert_eq!(
            parse_macos(
                "Firewall is enabled. (State = 1)",
                "The application is permitted"
            ),
            Verdict::Allowed
        );
    }

    #[test]
    fn firewalld_is_asked_about_the_port_and_its_zone() {
        // Fedora Workstation: the default zone opens 1025-65535, which
        // `--query-port` counts.
        assert_eq!(
            parse_firewalld(Ok((true, "yes\n".into())), "  target: default", None, 47800),
            Verdict::Allowed
        );
        // A zone that lets everything in.
        assert_eq!(
            parse_firewalld(
                Ok((false, "no\n".into())),
                "trusted\n  target: ACCEPT",
                None,
                47800
            ),
            Verdict::Allowed
        );
        let closed = parse_firewalld(Ok((false, "no\n".into())), "  target: default", None, 47800);
        assert!(closed.is_problem());
        assert_eq!(
            closed.command(),
            Some(
                "sudo firewall-cmd --permanent --add-port=47800/tcp && sudo firewall-cmd --reload"
            )
        );
        // The command names Tailscale's zone when it has one.
        let zoned = parse_firewalld(Ok((false, "no".into())), "", Some("home"), 47801);
        assert!(
            zoned
                .command()
                .unwrap()
                .contains("--zone=home --add-port=47801/tcp")
        );
        // The text doesn't carry the command: it's shown apart, to copy.
        assert!(!closed.explain().contains("sudo"));
        assert!(matches!(
            parse_firewalld(Err("couldn't run".into()), "", None, 47800),
            Verdict::Unknown(_)
        ));
    }

    #[test]
    fn ufw_rules_for_the_port() {
        assert_eq!(parse_ufw(Err("not found".into()), 47800), Verdict::Off);
        assert_eq!(
            parse_ufw(Ok("Status: inactive\n".into()), 47800),
            Verdict::Off
        );
        let allowed = "Status: active\n\nTo                         Action      From\n\
                       --                         ------      ----\n\
                       47800/tcp                  ALLOW       Anywhere\n";
        assert_eq!(parse_ufw(Ok(allowed.into()), 47800), Verdict::Allowed);
        let other = "Status: active\n22/tcp                     ALLOW       Anywhere\n";
        assert_eq!(
            parse_ufw(Ok(other.into()), 47800).command(),
            Some("sudo ufw allow 47800/tcp")
        );
        assert!(parse_ufw(Ok(other.into()), 47800).is_problem());
        let root = parse_ufw(
            Err("ERROR: You need to be root to run this script".into()),
            47800,
        );
        assert!(matches!(root, Verdict::MaybeClosed { .. }) && !root.is_problem());
    }
}
