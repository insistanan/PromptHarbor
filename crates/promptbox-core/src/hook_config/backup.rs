use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn backup_config_file(path: &Path, product_label: &str) -> Result<PathBuf, String> {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let backup_path = path.with_file_name(format!("{file_name}.promptbox.{timestamp}.bak"));
    fs::copy(path, &backup_path).map_err(|error| {
        format!(
            "备份 {product_label} 配置失败：{} -> {}：{error}",
            path.display(),
            backup_path.display()
        )
    })?;
    Ok(backup_path)
}
