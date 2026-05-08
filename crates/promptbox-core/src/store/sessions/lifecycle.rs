use super::super::{
    attachments, drafts,
    text::{session_db_id, short_session_title, title_from_prompt},
    types::{ArchiveSessionOutcome, DeleteSessionOutcome},
};
use crate::current_captured_at;
use rusqlite::{params, Connection};
use std::{fs, path::Path};

pub(in crate::store) fn archive_session(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    force: bool,
) -> Result<ArchiveSessionOutcome, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let status = connection
        .query_row(
            "select status from sessions where id = ?1",
            params![session_db_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| format!("读取 Agent 会话状态失败：{error}"))?;

    if status == "archived" {
        return Ok(ArchiveSessionOutcome {
            archived: true,
            requires_confirmation: false,
            message: "Agent 会话已经是历史状态".to_string(),
        });
    }

    if status != "active" && status != "maybe_closed" {
        return Err(format!("不支持归档当前状态的 Agent 会话：{status}"));
    }

    let has_non_empty_draft = drafts::session_has_non_empty_draft(connection, session_db_id)?;
    if has_non_empty_draft && !force {
        return Ok(ArchiveSessionOutcome {
            archived: false,
            requires_confirmation: true,
            message: "该 Agent 会话有非空当前草稿，归档前需要确认".to_string(),
        });
    }

    let now = current_captured_at();
    connection
        .execute(
            r#"
            update sessions
            set status = 'archived',
                archived_at = ?1,
                updated_at = ?1
            where id = ?2
            "#,
            params![now, session_db_id],
        )
        .map_err(|error| format!("归档 Agent 会话失败：{error}"))?;

    Ok(ArchiveSessionOutcome {
        archived: true,
        requires_confirmation: false,
        message: "Agent 会话已归档为历史".to_string(),
    })
}

pub(in crate::store) fn delete_session(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    attachment_root: &Path,
) -> Result<DeleteSessionOutcome, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let attachment_files = attachments::session_attachment_files(connection, session_db_id)?;
    let prompt_events_deleted = count_session_rows(connection, "prompt_events", session_db_id)?;
    let drafts_deleted = count_session_rows(connection, "draft_items", session_db_id)?
        + count_session_rows(connection, "drafts", session_db_id)?;
    let attachments_deleted = attachment_files.len();

    let mut files_deleted = 0_usize;
    for file_path in &attachment_files {
        if remove_prompt_attachment_file(file_path, attachment_root)? {
            files_deleted += 1;
        }
    }

    connection
        .execute(
            r#"
            delete from prompt_event_attachments
            where prompt_event_id in (
              select id from prompt_events where session_db_id = ?1
            )
            "#,
            params![session_db_id],
        )
        .map_err(|error| format!("删除 prompt 图片附件记录失败：{error}"))?;
    connection
        .execute(
            "delete from prompt_events where session_db_id = ?1",
            params![session_db_id],
        )
        .map_err(|error| format!("删除已发送 prompt 记录失败：{error}"))?;
    connection
        .execute(
            "delete from draft_items where session_db_id = ?1",
            params![session_db_id],
        )
        .map_err(|error| format!("删除草稿列表失败：{error}"))?;
    connection
        .execute(
            "delete from drafts where session_db_id = ?1",
            params![session_db_id],
        )
        .map_err(|error| format!("删除兼容草稿记录失败：{error}"))?;
    connection
        .execute(
            "delete from raw_hook_events where provider = ?1 and session_id = ?2",
            params![provider, session_id],
        )
        .map_err(|error| format!("删除 raw hook 记录失败：{error}"))?;
    connection
        .execute("delete from sessions where id = ?1", params![session_db_id])
        .map_err(|error| format!("删除 Agent 会话失败：{error}"))?;

    Ok(DeleteSessionOutcome {
        deleted: true,
        provider: provider.to_string(),
        session_id: session_id.to_string(),
        prompt_events_deleted,
        drafts_deleted,
        attachments_deleted,
        files_deleted,
        message: "已删除 PromptHarbor 本地会话记录；不会删除 Claude Code 或 Codex CLI 原始会话文件"
            .to_string(),
    })
}

fn remove_prompt_attachment_file(file_path: &Path, attachment_root: &Path) -> Result<bool, String> {
    if !file_path.exists() {
        return Ok(false);
    }

    let canonical_root = attachment_root.canonicalize().map_err(|error| {
        format!(
            "解析 PromptHarbor 附件目录失败：{}：{error}",
            attachment_root.display()
        )
    })?;
    let canonical_file = file_path.canonicalize().map_err(|error| {
        format!(
            "解析 PromptHarbor 附件文件失败：{}：{error}",
            file_path.display()
        )
    })?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err(format!(
            "拒绝删除附件目录外的文件：{}",
            canonical_file.display()
        ));
    }

    fs::remove_file(&canonical_file).map_err(|error| {
        format!(
            "删除 PromptHarbor 附件文件失败：{}：{error}",
            canonical_file.display()
        )
    })?;
    Ok(true)
}

pub(in crate::store) fn set_session_note(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    note: &str,
) -> Result<(), String> {
    let now = current_captured_at();
    let trimmed = note.trim();

    if trimmed.is_empty() {
        let (db_session_id, first_prompt): (String, Option<String>) = connection
            .query_row(
                "select session_id, first_prompt from sessions where provider = ?1 and session_id = ?2",
                params![provider, session_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .map_err(|error| format!("读取 Agent 会话失败：{error}"))?;

        let (fallback_title, fallback_source) = match first_prompt.as_deref() {
            Some(prompt) if !prompt.trim().is_empty() => {
                (title_from_prompt(prompt, &db_session_id), "first_non_low_info_prompt")
            }
            _ => (short_session_title(&db_session_id), "session_id"),
        };

        connection
            .execute(
                "update sessions set title = ?1, title_source = ?2, updated_at = ?3 where provider = ?4 and session_id = ?5",
                params![fallback_title, fallback_source, now, provider, session_id],
            )
            .map_err(|error| format!("清除会话备注失败：{error}"))?;
    } else {
        connection
            .execute(
                "update sessions set title = ?1, title_source = 'manual', updated_at = ?2 where provider = ?3 and session_id = ?4",
                params![trimmed, now, provider, session_id],
            )
            .map_err(|error| format!("更新会话备注失败：{error}"))?;
    }

    Ok(())
}

fn count_session_rows(
    connection: &Connection,
    table: &str,
    session_db_id: i64,
) -> Result<usize, String> {
    let sql = format!("select count(*) from {table} where session_db_id = ?1");
    connection
        .query_row(&sql, params![session_db_id], |row| row.get::<_, i64>(0))
        .map(|count| count as usize)
        .map_err(|error| format!("读取会话数据计数失败：{table}：{error}"))
}
