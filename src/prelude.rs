pub use anyhow::{bail, Context};
pub use async_ssh2_tokio::client::{Client, CommandExecutedResult};
pub use color_eyre::owo_colors::OwoColorize;
pub use serde::{Deserialize, Serialize};
pub use serde_aux::prelude::*;
pub use std::collections::BTreeMap as Map;
pub use tracing::*;

pub enum Os {
    Ubuntu,
    Debian,
    Unsupported,
}

pub async fn osinfo(client: &Client) -> Os {
    match silent(&client, "uname -a").await {
        Ok(out) => {
            if out.output.contains("Ubuntu") {
                Os::Ubuntu
            } else if out.output.contains("Debian") {
                Os::Debian
            } else {
                Os::Unsupported
            }
        }
        Err(_) => Os::Unsupported,
    }
}

pub async fn which(client: &Client, cmd: &str) -> anyhow::Result<String> {
    match silent(&client, &cmd).await {
        Ok(out) => {
            if out.exit_status == 0 {
                return Ok(out.output.trim().to_string());
            } else {
                bail!(out.output.trim().replace("bash: line 1: ", "").to_string());
            }
        }
        Err(_) => bail!("not installed"),
    }
}

pub async fn some_output(client: &Client, cmd: &str) -> bool {
    match silent(&client, &cmd).await {
        Ok(out) => {
            if out.exit_status == 0 {
                !out.output.trim().is_empty()
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

pub async fn file_exists(client: &Client, filename: &str) -> bool {
    let cmd = format!("ls -1 {}", filename);
    match silent(&client, &cmd).await {
        Ok(out) => out.exit_status == 0,
        Err(_) => false,
    }
}

/// run and fail on any exit_status that is not 0
#[instrument(skip(client), level = "debug")]
pub async fn run(client: &Client, cmd: &str) -> anyhow::Result<CommandExecutedResult> {
    let exec_result: CommandExecutedResult = client.execute(&cmd).await?;
    if exec_result.exit_status == 0 {
        debug!("{} {:?}", cmd, exec_result);
        Ok(exec_result)
    } else {
        warn!("{} {:?}", cmd, exec_result);
        Err(anyhow::Error::msg(exec_result.output))
    }
}

/// run and ingore the possible erro
#[instrument(skip(client), level = "debug")]
pub async fn silent(client: &Client, cmd: &str) -> anyhow::Result<CommandExecutedResult> {
    let exec_result: CommandExecutedResult = client.execute(&cmd).await?;
    debug!("{} {:?}", cmd, exec_result);
    Ok(exec_result)
}

#[derive(Serialize)]
pub enum Status {
    Installed {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        success: Vec<String>,
    },
    NotInstalled {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        success: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        fail: Vec<String>,
    },
}
impl std::fmt::Debug for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Installed { success } => {
                let mut out = f.debug_struct("Installed");
                if success.len() > 0 {
                    out.field("success", success);
                }
                out.finish()
            }
            Status::NotInstalled { success, fail } => {
                let mut out = f.debug_struct("NotInstalled");
                if success.len() > 0 {
                    out.field("success", success);
                }
                if fail.len() > 0 {
                    out.field("fail", fail);
                }
                out.finish()
            }
        }
    }
}

impl Status {
    pub fn new(success: Vec<String>, failure: Vec<String>) -> Self {
        if failure.is_empty() {
            Status::Installed { success }
        } else {
            Status::NotInstalled {
                success,
                fail: failure,
            }
        }
    }

    pub fn print(&self, alias: &str) {
        let out = format!("{:?}", self);
        match &self {
            Status::Installed { .. } => {
                println!("+ {}: {}", alias.green(), format!("{}", out).green());
            }
            Status::NotInstalled { .. } => {
                println!("+ {}: {}", alias.red(), format!("{}", out).red());
            }
        }
    }
}
