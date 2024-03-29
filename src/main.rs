use anyhow::Result;
use clap::Parser;
use inquire::Confirm;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::{env::temp_dir, hash::Hash, path::PathBuf};
use xshell::Shell;

#[derive(Clone)]
enum Type {
    GitHub,
    GitLab,
}

impl clap::ValueEnum for Type {
    fn value_variants<'a>() -> &'a [Self] {
        &[Self::GitHub, Self::GitLab]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            Self::GitHub => Some(clap::builder::PossibleValue::new("github")),
            Self::GitLab => Some(clap::builder::PossibleValue::new("gitlab")),
        }
    }
}

#[derive(Parser)]
#[command()]
struct Args {
    #[arg(short, long)]
    url: String,
    #[arg(short, long)]
    name: String,
    /// Use HTTPS protocol, default is git protocol.
    #[arg(long)]
    #[clap(default_value_t = false)]
    https: bool,
    #[arg(name = "type", value_enum, short, long)]
    ty: Option<Type>,
}

struct Info {
    owner: String,
    repo: String,
    refs: String,
    path: String,
    domain: String,
}

fn hash<T>(t: T) -> u64
where
    T: Hash,
{
    let mut hasher = DefaultHasher::new();
    t.hash(&mut hasher);

    hasher.finish()
}

fn parse_url(url: &str, ty: Option<Type>) -> Info {
    let ty = match ty {
        Some(ty) => ty,
        None => {
            if url.starts_with("https://github.com/") {
                Type::GitHub
            } else if url.starts_with("https://gitlab.com/") {
                Type::GitLab
            } else {
                eprintln!(
                    "Unsupported URL schema(GitHub and GitLab like only), you can tell me the schema type by providing --type argument."
                );
                std::process::exit(1);
            }
        }
    };

    match ty {
        Type::GitHub => {
            let re = regex::Regex::new(r"https://(?P<domain>[^/]+)/(?P<owner>[^/]+)/(?P<repo>[^/]+)/tree/(?P<refs>[^/]+)/(?P<path>(.+))").unwrap();

            re.captures(url)
                .map(|caps| {
                    let owner = caps.name("owner").unwrap().as_str().into();
                    let repo = caps.name("repo").unwrap().as_str().into();
                    let refs = caps.name("refs").unwrap().as_str().into();
                    let path = caps.name("path").unwrap().as_str().into();
                    let domain = caps.name("domain").unwrap().as_str().into();

                    Info { owner, repo, refs, path, domain }
                })
                .expect("The URL doesn't match GitHub's schema")
        }
        Type::GitLab => {
            let re = regex::Regex::new( r"https://(?P<domain>[^/]+)/(?P<owner>[^/]+)/(?P<repo>[^/]+)/-/tree/(?P<refs>[^/]+)/(?P<path>(.+))").unwrap();

            re.captures(url)
                .map(|caps| {
                    let owner = caps.name("owner").unwrap().as_str().into();
                    let repo = caps.name("repo").unwrap().as_str().into();
                    let refs = caps.name("refs").unwrap().as_str().into();
                    let path = caps.name("path").unwrap().as_str().into();
                    let domain = caps.name("domain").unwrap().as_str().into();

                    Info { owner, repo, refs, path, domain }
                })
                .expect("The URL doesn't match GitLab's schema")
        }
    }
}

fn fetch_template(url: &str, ty: Option<Type>, use_https: bool) -> Result<(PathBuf, PathBuf)> {
    let Info { owner, repo, refs, path, domain } = parse_url(url, ty);

    let id = hash(format!("{domain}-{owner}-{repo}-{refs}"));
    let folder_name = format!("create-x-{id}");
    let clone_dir = temp_dir().join(folder_name);

    let sh = xshell::Shell::new()?;

    if clone_dir.exists() {
        fs_extra::dir::remove(&clone_dir)?;
    }
    sh.create_dir(&clone_dir)?;
    sh.change_dir(&clone_dir);
    xshell::cmd!(sh, "git init --quiet").quiet().ignore_stdout().run()?;

    if use_https {
        xshell::cmd!(sh, "git remote add origin https://{domain}/{owner}/{repo}.git")
    } else {
        xshell::cmd!(sh, "git remote add origin git@{domain}:{owner}/{repo}.git")
    }
    .quiet()
    .ignore_stdout()
    .run()?;

    xshell::cmd!(sh, "git config core.sparseCheckout true").quiet().ignore_stdout().run()?;
    sh.write_file(".git/info/sparse-checkout", &path)?;
    xshell::cmd!(sh, "git pull --quiet --depth=1 origin {refs}").quiet().ignore_stdout().run()?;

    let template_dir = path.split('/').fold(clone_dir.clone(), |x, y| x.join(y));
    Ok((clone_dir, template_dir))
}

fn main() -> Result<()> {
    let args = Args::parse();

    let cwd = std::env::current_dir()?;
    let dest_dir = cwd.join(&args.name);

    if dest_dir.exists() {
        if !Confirm::new("The folder already exists. Do you want to delete it and continue?")
            .with_default(false)
            .prompt()?
        {
            std::process::exit(1);
        }

        fs_extra::dir::remove(&dest_dir).unwrap();
    }

    let (clone_dir, template_dir) = fetch_template(&args.url, args.ty, args.https)?;

    // Copy template into target directory
    {
        let mut copy_options = fs_extra::dir::CopyOptions::new();
        copy_options.copy_inside = true;
        fs_extra::dir::move_dir(template_dir, &dest_dir, &copy_options).unwrap();

        let gitignore = dest_dir.join("_gitignore");
        if gitignore.exists() {
            let mut copy_options = fs_extra::file::CopyOptions::new();
            copy_options.skip_exist = true;
            fs_extra::file::move_file(gitignore, dest_dir.join(".gitignore"), &copy_options)
                .unwrap();
        }
        fs_extra::dir::remove(clone_dir)?;
    }

    // Run postscript
    {
        let ps_ps1 = dest_dir.join("_postscript_.ps1");
        let ps_sh = dest_dir.join("_postscript_.sh");

        #[cfg(windows)]
        {
            if ps_ps1.exists() {
                let shell = Shell::new()?;
                shell.change_dir(dest_dir);
                let pwsh = which::which("pwsh.exe").map(|_| "pwsh.exe").unwrap_or("powershell.exe");
                xshell::cmd!(shell, "{pwsh} -ExecutionPolicy Bypass --File {ps_ps1}")
                    .quiet()
                    .run()?;
            }
        }

        #[cfg(not(windows))]
        {
            if ps_sh.exists() {
                let shell = Shell::new()?;
                shell.change_dir(dest_dir);
                xshell::cmd!(shell, "chmod +x {ps_sh}").quiet().run()?;
                xshell::cmd!(shell, "{ps_sh}").quiet().run()?;
            }
        }

        // Delete postscripts
        if ps_ps1.exists() {
            fs_extra::file::remove(ps_ps1)?;
        }
        if ps_sh.exists() {
            fs_extra::file::remove(ps_sh)?;
        }
    }

    Ok(())
}
