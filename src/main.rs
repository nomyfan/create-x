mod github;

use anyhow::Result;
use clap::Parser;

lazy_static::lazy_static! {
    static ref GITHUB_URL_RE: regex::Regex = regex::Regex::new(
        r"https://github.com/(?P<owner>[^/]+)/(?P<repo>[^/]+)/tree/(?P<refs>[^/]+)/(?P<path>(.+))",
    ).unwrap();
}

#[derive(Parser, Debug)]
#[command()]
struct Args {
    #[arg(short, long)]
    url: String,
    #[arg(short, long)]
    name: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let caps = GITHUB_URL_RE.captures(&args.url).expect("Invalid github url");

    let owner = caps.name("owner").unwrap().as_str();
    let repo = caps.name("repo").unwrap().as_str();
    let refs = caps.name("refs").unwrap().as_str();
    let path = caps.name("path").unwrap().as_str();

    let repo_root_dir = github::fetch(owner, refs, repo)?;

    // Copy template into target directory
    {
        let template_dir = path.split('/').into_iter().fold(repo_root_dir, |x, y| x.join(y));

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
