// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::{Context, Result};
use std::process::Command;

const SERVICE_NAME: &str = "p2poolv2";

pub struct P2PoolV2Service;

impl P2PoolV2Service {
    pub fn start() -> Result<()> {
        Self::run_systemctl("start")
    }

    pub fn stop() -> Result<()> {
        Self::run_systemctl("stop")
    }

    pub fn restart() -> Result<()> {
        Self::run_systemctl("restart")
    }

    pub fn is_running() -> Result<bool> {
        let output = Command::new("systemctl")
            .args(["--user", "is-active", SERVICE_NAME])
            .output()
            .context("failed to execute systemctl")?;

        Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "active")
    }

    fn run_systemctl(action: &str) -> Result<()> {
        let output = Command::new("systemctl")
            .args(["--user", action, SERVICE_NAME])
            .output()
            .with_context(|| format!("failed to execute systemctl {action}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "systemctl --user {action} {SERVICE_NAME} failed: {}",
                stderr.trim()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn fake_systemctl(fail_actions: bool, active: bool) -> (TempDir, Option<std::ffi::OsString>) {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("systemctl");
        fs::write(
            &script,
            format!(
                "#!/bin/sh\nif [ \"$2\" = \"is-active\" ]; then\n  {}\nelse\n  {}\nfi\n",
                if active {
                    "echo active; exit 0"
                } else {
                    "echo inactive; exit 3"
                },
                if fail_actions {
                    "echo permission denied >&2; exit 1"
                } else {
                    "exit 0"
                }
            ),
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var_os("PATH");
        let path = match old_path.as_ref() {
            Some(old_path) => format!("{}:{}", dir.path().display(), old_path.to_string_lossy()),
            None => dir.path().display().to_string(),
        };
        unsafe { std::env::set_var("PATH", path) };
        (dir, old_path)
    }

    fn restore_path(old_path: Option<std::ffi::OsString>) {
        unsafe {
            match old_path {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
        }
    }

    #[test]
    #[serial]
    fn service_actions_succeed_and_active_service_is_running() -> Result<()> {
        let (_dir, old_path) = fake_systemctl(false, true);

        let result = (|| {
            P2PoolV2Service::start()?;
            P2PoolV2Service::stop()?;
            P2PoolV2Service::restart()?;
            assert!(P2PoolV2Service::is_running()?);
            Ok::<_, anyhow::Error>(())
        })();

        restore_path(old_path);
        result
    }

    #[test]
    #[serial]
    fn service_actions_report_systemctl_failures() {
        let (_dir, old_path) = fake_systemctl(true, false);

        let start_error = P2PoolV2Service::start().unwrap_err().to_string();
        let stop_error = P2PoolV2Service::stop().unwrap_err().to_string();
        let restart_error = P2PoolV2Service::restart().unwrap_err().to_string();
        let running = P2PoolV2Service::is_running().unwrap();

        restore_path(old_path);

        assert!(start_error.contains("start p2poolv2 failed: permission denied"));
        assert!(stop_error.contains("stop p2poolv2 failed: permission denied"));
        assert!(restart_error.contains("restart p2poolv2 failed: permission denied"));
        assert!(!running);
    }

    #[test]
    #[ignore = "requires a user systemd session and an installed p2poolv2.service"]
    fn start_and_stop_service() -> Result<()> {
        assert!(!P2PoolV2Service::is_running()?);

        P2PoolV2Service::start()?;
        let running = P2PoolV2Service::is_running();
        let stop_result = P2PoolV2Service::stop();

        assert!(running?);
        stop_result?;
        assert!(!P2PoolV2Service::is_running()?);

        Ok(())
    }
}
