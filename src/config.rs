// SPDX-FileCopyrightText: 2024 PDM Authors
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use anyhow::Result;
use config::{Config, File};

pub struct ApiConfig {
    pub base_url: String,
    pub auth_user: Option<String>,
    pub auth_pass: Option<String>,
}

pub fn load_api_config() -> Result<ApiConfig> {
    let settings = Config::builder()
        .add_source(File::with_name("config/config"))
        .build()?;

    let host: String = settings.get("api.host")?;
    let port: u16 = settings.get("api.port")?;
    let auth_user: Option<String> = settings.get("api.auth_user").ok();
    let auth_pass: Option<String> = settings.get("api.auth_pass").ok();

    Ok(ApiConfig {
        base_url: format!("http://{}:{}", host, port),
        auth_user,
        auth_pass,
    })
}
