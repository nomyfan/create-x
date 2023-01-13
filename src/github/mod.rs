use anyhow::Result;
use std::env::temp_dir;
use std::path::PathBuf;

pub(crate) fn fetch(owner: &str, refs: &str, repo: &str) -> Result<PathBuf> {
    let folder_name = format!("create-x-{owner}-{repo}-{refs}");
    let clone_dir = temp_dir().join(&folder_name);

    let sh = xshell::Shell::new()?;

    if clone_dir.exists() {
        sh.change_dir(clone_dir.as_path());
        xshell::cmd!(sh, "git pull").ignore_stdout().run()?;
    } else {
        sh.change_dir(temp_dir());
        xshell::cmd!(
            sh,
            "git clone --depth 1 --branch {refs} git@github.com:{owner}/{repo}.git {folder_name}"
        )
        .ignore_stdout()
        .run()?;
    }

    Ok(clone_dir)
}
