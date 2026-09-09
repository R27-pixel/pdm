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
