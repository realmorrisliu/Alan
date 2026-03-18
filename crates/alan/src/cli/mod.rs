pub mod ask;
pub mod chat;
pub mod daemon;
pub mod init;
pub mod migrate;
pub mod workspace;

pub(crate) fn load_agent_config_with_notice() -> anyhow::Result<alan_runtime::Config> {
    Ok(alan_runtime::Config::load_with_metadata()?.into_config())
}
