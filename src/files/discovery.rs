use crate::platform::{ConfigFile, Platform};
use crate::trace::Context;

/// Discover all config files for the given context
///
/// Returns files in the order they should be processed (which matters for
/// correctly tracking variable modifications).
pub fn discover_files(platform: Platform, context: Context) -> Vec<ConfigFile> {
    let files = platform.config_files(context);

    // Filter to only files that exist and are readable
    files.into_iter().filter(|f| f.path.exists()).collect()
}
