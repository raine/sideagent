use anyhow::{Context, Result};
use std::fs;
use std::io::IsTerminal;
use std::path::Path;

const SKILL_ID: &str = "agent-offload";
const SKILL_CONTENT: &str = include_str!("../skills/agent-offload/SKILL.md");

pub fn run() -> Result<()> {
    let home = dirs::home_dir().context("could not find home directory")?;
    let skill_dir = home.join(".claude").join("skills").join(SKILL_ID);
    let dest = skill_dir.join("SKILL.md");
    let color = use_color();

    let up_to_date = fs::read(&dest).is_ok_and(|bytes| bytes == SKILL_CONTENT.as_bytes());

    if up_to_date {
        print_line("up-to-date", &dest, color, None, &home);
        return Ok(());
    }

    fs::create_dir_all(&skill_dir)
        .with_context(|| format!("could not create {}", skill_dir.display()))?;
    fs::write(&dest, SKILL_CONTENT.as_bytes())
        .with_context(|| format!("could not write {}", dest.display()))?;

    print_line("written", &dest, color, Some(32), &home);
    Ok(())
}

fn use_color() -> bool {
    std::io::stdout().is_terminal()
        && std::env::var("NO_COLOR")
            .map(|v| v.is_empty())
            .unwrap_or(true)
}

fn print_line(status: &str, path: &Path, color: bool, ansi_color: Option<u8>, home: &Path) {
    let display = shrink_path(path, home);
    if color {
        if let Some(code) = ansi_color {
            println!("\x1b[{code}m{status:<12}\x1b[0m {display}");
        } else {
            println!("\x1b[2m{status:<12}\x1b[0m {display}");
        }
    } else {
        println!("{status:<12} {display}");
    }
}

fn shrink_path(path: &Path, home: &Path) -> String {
    path.strip_prefix(home)
        .map(|rel| format!("~/{}", rel.display()))
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_shrink_path_under_home() {
        let home = PathBuf::from("/tmp/home");
        let path = home.join(".claude/skills/agent-offload/SKILL.md");
        assert_eq!(
            shrink_path(&path, &home),
            "~/.claude/skills/agent-offload/SKILL.md"
        );
    }

    #[test]
    fn test_shrink_path_outside_home() {
        let home = PathBuf::from("/tmp/home");
        let path = PathBuf::from("/var/tmp/SKILL.md");
        assert_eq!(shrink_path(&path, &home), "/var/tmp/SKILL.md");
    }
}
