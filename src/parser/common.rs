use std::path::PathBuf;

/// Strip surrounding quotes from a value
pub fn strip_quotes(value: &str) -> String {
    let value = value.trim();
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        value[1..value.len() - 1].to_string()
    } else {
        value.to_string()
    }
}

/// Expand ~ in source paths and strip quotes
pub fn expand_source_path(path: &str) -> Option<PathBuf> {
    let path = strip_quotes(path);

    if let Some(rest) = path.strip_prefix('~') {
        if let Some(home) = dirs::home_dir()
            && (rest.is_empty() || rest.starts_with('/'))
        {
            return Some(home.join(rest.trim_start_matches('/')));
        }
        return None; // Can't expand ~user
    }

    // Skip paths with unexpanded variables
    if path.contains('$') {
        return None;
    }

    Some(PathBuf::from(path))
}
