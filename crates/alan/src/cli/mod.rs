pub mod ask;
pub mod chat;
pub mod daemon;
pub mod init;
pub mod migrate;
pub mod workspace;

pub(crate) fn load_agent_config_with_notice() -> anyhow::Result<alan_runtime::Config> {
    let loaded = alan_runtime::Config::load_with_metadata()?;
    if let Some(notice) = loaded.legacy_notice() {
        eprintln!("Warning: {notice}");
    }
    Ok(loaded.into_config())
}
