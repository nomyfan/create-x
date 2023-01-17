use anyhow::Result;
use clap::Parser;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::{env::temp_dir, hash::Hash, path::PathBuf};

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

fn parse_url<'a>(url: &'a str, ty: Option<Type>) -> Info {
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

fn fetch_template<'a>(url: &'a str, ty: Option<Type>) -> Result<PathBuf> {
    let Info { owner, repo, refs, path, domain } = parse_url(url, ty);

    let id = hash(format!("{domain}-{owner}-{repo}-{refs}"));
    let folder_name = format!("create-x-{id}");
    let clone_dir = temp_dir().join(&folder_name);

    let sh = xshell::Shell::new()?;

    if clone_dir.exists() {
        sh.change_dir(clone_dir.as_path());
        xshell::cmd!(sh, "git pull").ignore_stdout().run()?;
    } else {
        sh.change_dir(temp_dir());
        xshell::cmd!(
            sh,
            "git clone --depth 1 --branch {refs} git@{domain}:{owner}/{repo}.git {folder_name}"
        )
        .ignore_stdout()
        .run()?;
    }

    let template_dir = path.split('/').into_iter().fold(clone_dir, |x, y| x.join(y));
    Ok(template_dir)
}

fn main() -> Result<()> {
    let args = Args::parse();

    let template_dir = fetch_template(&args.url, args.ty)?;

    // Copy template into target directory
    {
        let mut copy_options = fs_extra::dir::CopyOptions::new();
        let dest_dir = std::path::Path::new(&args.name);
        copy_options.copy_inside = true;
        fs_extra::dir::copy(&template_dir, &dest_dir, &copy_options).unwrap();

        let gitignore = dest_dir.join("_gitignore");
        if gitignore.exists() {
            let mut copy_options = fs_extra::file::CopyOptions::new();
            copy_options.skip_exist = true;
            fs_extra::file::move_file(gitignore, dest_dir.join(".gitignore"), &copy_options)
                .unwrap();
        }
    }

    Ok(())
}
