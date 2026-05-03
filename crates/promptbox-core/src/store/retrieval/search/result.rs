use super::super::super::{
    text::{project_name_from_cwd, provider_label, short_session_id, snippet},
    types::PromptSearchResultItem,
};

pub(super) fn search_result(
    provider: &str,
    session_id: &str,
    cwd: Option<&str>,
    title: &str,
    match_kind: &str,
    match_label: &str,
    snippet_source: &str,
    is_low_info: bool,
    sent_at: Option<String>,
    updated_at: &str,
) -> PromptSearchResultItem {
    PromptSearchResultItem {
        provider: provider.to_string(),
        provider_label: provider_label(provider).to_string(),
        session_id: session_id.to_string(),
        short_session_id: short_session_id(session_id),
        title: title.to_string(),
        project_name: cwd
            .map(project_name_from_cwd)
            .unwrap_or_else(|| "未知项目".to_string()),
        match_kind: match_kind.to_string(),
        match_label: match_label.to_string(),
        snippet: snippet(snippet_source),
        is_low_info,
        sent_at,
        updated_at: updated_at.to_string(),
    }
}
