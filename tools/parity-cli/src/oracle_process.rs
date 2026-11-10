//! Starting the oracle runtime: one child process with a cleared
//! environment plus an allow-list, a closed stdin, both output pipes read
//! while it runs, and a time limit. Used by `oracle-verify`'s runtime
//! probe and by every `oracle-run` launch.

use std::ffi::OsString;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// What a child did.
#[derive(Debug)]
pub struct Finished {
    /// Exit code; `None` when the process ended without one (signal) or
    /// was killed at the time limit.
    pub code: Option<i32>,
    /// The time limit was reached and the process was killed.
    pub timed_out: bool,
    /// Standard output, lossy UTF-8.
    pub stdout: String,
    /// Standard error, lossy UTF-8.
    pub stderr: String,
    /// Wall time until exit or kill.
    pub elapsed: Duration,
}

/// Runs `program` with `args` in `dir`. The environment is cleared and
/// holds only `env` and, on Windows, `SystemRoot` (the CRT and DLL loading
/// need it). Stdin is closed, so a child that reads it gets end of file at
/// once. `Err` only when the process cannot be started.
pub fn run(
    program: &Path,
    args: &[OsString],
    dir: &Path,
    env: &[(String, OsString)],
    limit: Duration,
) -> Result<Finished, String> {
    let mut command = Command::new(program);
    command
        .env_clear()
        .envs(env.iter().map(|(k, v)| (k, v)))
        .current_dir(dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if cfg!(windows)
        && let Some(root) = std::env::var_os("SystemRoot")
    {
        command.env("SystemRoot", root);
    }
    // File name only: messages end up in reports, which must not carry
    // machine paths.
    let name = program
        .file_name()
        .map_or_else(|| "the program".into(), |n| n.to_string_lossy());
    let start = Instant::now();
    let mut child = command
        .spawn()
        .map_err(|e| format!("cannot start {name}: {}", e.kind()))?;
    // Both pipes are drained on their own threads while we wait, so a
    // child that writes a lot cannot block on a full pipe.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let (code, timed_out) = loop {
        match child.try_wait() {
            Ok(Some(status)) => break (status.code(), false),
            Ok(None) if start.elapsed() > limit => {
                // Best effort: the child may have exited meanwhile.
                let _ = child.kill();
                let _ = child.wait();
                break (None, true);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("waiting for {name}: {}", e.kind()));
            }
        }
    };
    let elapsed = start.elapsed();
    Ok(Finished {
        code,
        timed_out,
        stdout: stdout.join().unwrap_or_default(),
        stderr: stderr.join().unwrap_or_default(),
        elapsed,
    })
}

/// `LUA_CPATH` for the runtime in `bin`: its own directory only.
pub fn cpath(bin: &Path) -> OsString {
    let suffix = if cfg!(windows) { "dll" } else { "so" };
    bin.join(format!("?.{suffix}")).into_os_string()
}

/// Reads a child's pipe to the end on a thread (lossy UTF-8).
fn drain(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<String> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            // A read error keeps what was read so far.
            let _ = pipe.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

#[cfg(test)]
mod tests {
    //! The test binary itself is the child: [`child`] does nothing in a
    //! normal run and acts on `LOOM_TEST_CHILD` when this module starts it.

    use super::*;

    const CHILD_VAR: &str = "LOOM_TEST_CHILD";

    fn start(mode: &str, limit: Duration, extra: &[(&str, &str)]) -> Finished {
        let exe = std::env::current_exe().unwrap();
        let args: Vec<OsString> = ["--exact", "oracle_process::tests::child", "--nocapture"]
            .iter()
            .map(OsString::from)
            .collect();
        let mut env = vec![(CHILD_VAR.to_owned(), OsString::from(mode))];
        env.extend(
            extra
                .iter()
                .map(|(k, v)| ((*k).to_owned(), OsString::from(v))),
        );
        run(&exe, &args, &std::env::temp_dir(), &env, limit).unwrap()
    }

    #[test]
    fn child() {
        let Ok(mode) = std::env::var(CHILD_VAR) else {
            return;
        };
        match mode.as_str() {
            "sleep" => std::thread::sleep(Duration::from_secs(30)),
            "flood" => {
                // Far more than a pipe buffer on both pipes.
                let line = "x".repeat(1023);
                for _ in 0..2048 {
                    println!("{line}");
                    eprintln!("{line}");
                }
            }
            "env" => {
                let mut names: Vec<String> = std::env::vars().map(|(k, _)| k).collect();
                names.sort();
                println!("ENV:{}", names.join(","));
            }
            "stdin" => {
                let mut text = String::new();
                let read = std::io::stdin().read_to_string(&mut text).unwrap();
                println!("STDIN:{read}");
            }
            "exit3" => std::process::exit(3),
            _ => {}
        }
    }

    #[test]
    fn run_should_kill_child_at_time_limit() {
        let finished = start("sleep", Duration::from_millis(500), &[]);
        assert!(finished.timed_out);
        assert_eq!(finished.code, None);
        assert!(finished.elapsed < Duration::from_secs(20));
    }

    #[test]
    fn run_should_read_both_pipes_of_a_child_that_writes_a_lot() {
        let finished = start("flood", Duration::from_secs(60), &[]);
        assert!(!finished.timed_out);
        assert_eq!(finished.code, Some(0));
        assert!(finished.stdout.len() > 2_000_000);
        assert!(finished.stderr.len() > 2_000_000);
    }

    #[test]
    fn run_should_pass_only_the_allow_list() {
        let finished = start("env", Duration::from_secs(60), &[("LUA_PATH", "x")]);
        let line = finished
            .stdout
            .lines()
            .find_map(|l| l.strip_prefix("ENV:"))
            .unwrap();
        let mut expected = vec![CHILD_VAR, "LUA_PATH"];
        if cfg!(windows) {
            expected.push("SystemRoot");
        }
        expected.sort_unstable();
        assert_eq!(line, expected.join(","));
    }

    #[test]
    fn run_should_close_stdin() {
        let finished = start("stdin", Duration::from_secs(60), &[]);
        assert!(finished.stdout.contains("STDIN:0"), "{finished:?}");
    }

    #[test]
    fn run_should_report_exit_code() {
        let finished = start("exit3", Duration::from_secs(60), &[]);
        assert_eq!(finished.code, Some(3));
    }

    #[test]
    fn run_should_fail_for_missing_program() {
        let err = run(
            Path::new("does-not-exist-loom"),
            &[],
            &std::env::temp_dir(),
            &[],
            Duration::from_secs(1),
        )
        .unwrap_err();
        assert!(err.starts_with("cannot start"), "{err}");
    }
}
