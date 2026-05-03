mod backup;
mod command;
mod tree;

pub(crate) use backup::backup_config_file;
pub(crate) use command::hook_command;
pub(crate) use tree::{
    ensure_user_prompt_submit_hook, has_promptbox_hook, has_stale_promptbox_hook,
    prune_empty_hooks_root, remove_promptbox_hooks,
};
