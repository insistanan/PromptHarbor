use super::super::types::{PromptSearchResultItem, PromptSearchResults};
use rusqlite::Connection;

mod drafts;
mod prompt_events;
mod result;
mod sessions;

use self::drafts::search_drafts;
use self::prompt_events::search_prompt_events;
use self::sessions::search_sessions;

pub(in crate::store) fn search_prompts(
    connection: &Connection,
    query: &str,
    include_low_info: bool,
) -> Result<PromptSearchResults, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(PromptSearchResults {
            query: String::new(),
            items: Vec::new(),
        });
    }

    let pattern = format!("%{query}%");
    let mut items = Vec::new();
    items.extend(search_sessions(
        connection,
        query,
        &pattern,
        include_low_info,
    )?);
    items.extend(search_prompt_events(connection, &pattern, include_low_info)?);
    items.extend(search_drafts(connection, &pattern)?);
    sort_search_results(&mut items);
    items.truncate(80);

    Ok(PromptSearchResults {
        query: query.to_string(),
        items,
    })
}

fn sort_search_results(items: &mut [PromptSearchResultItem]) {
    items.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.sent_at.cmp(&left.sent_at))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
}
