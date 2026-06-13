use std::path::Path;

pub(crate) fn resolve_project_name(cwd: &Path) -> String {
    let path_str = cwd.to_string_lossy();

    if let Some(wt_pos) = path_str
        .find("__worktrees/")
        .or_else(|| path_str.find("/.worktrees/"))
    {
        let is_hidden = path_str[wt_pos..].starts_with("/.");
        let separator_len = if is_hidden {
            "/.worktrees/".len()
        } else {
            "__worktrees/".len()
        };

        let before = &path_str[..wt_pos];
        let main_project = Path::new(before)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        let after = &path_str[wt_pos + separator_len..];
        let worktree = after.split('/').next().unwrap_or("");

        if !main_project.is_empty() && !worktree.is_empty() {
            return format!("{main_project}/{worktree}");
        }
    }

    cwd.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_str.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_regular_project_name() {
        assert_eq!(
            resolve_project_name(Path::new("/tmp/sideagent")),
            "sideagent"
        );
    }

    #[test]
    fn resolves_hidden_worktree_project_name() {
        assert_eq!(
            resolve_project_name(Path::new("/tmp/sideagent/.worktrees/feature")),
            "sideagent/feature"
        );
    }

    #[test]
    fn resolves_suffixed_worktree_project_name() {
        assert_eq!(
            resolve_project_name(Path::new("/tmp/sideagent__worktrees/feature")),
            "sideagent/feature"
        );
    }
}
