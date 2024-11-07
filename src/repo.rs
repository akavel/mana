use anyhow::{bail, Result};
use fn_error_context::context;
use git2::Repository as GitRepo;

use std::path::Path;

pub struct Repo {
    repo: GitRepo,
}

impl Repo {
    // open repo and verify it has no pending operation
    #[context("opening git repository {dir:?}")]
    pub fn open(dir: &Path) -> Result<Repo> {
        let repo = GitRepo::open(dir)?;
        if repo.state() != git2::RepositoryState::Clean {
            bail!(
                "repository has pending unfinished operation {:?}",
                repo.state()
            );
        }
        Ok(Self { repo })
    }

    pub fn index(&self) -> Result<git2::Index, git2::Error> {
        self.repo.index()
    }

    #[context("checking statuses in git repository")]
    pub fn statuses_are_empty(&self, ignores: impl IntoIterator<Item: AsRef<str>>) -> Result<bool> {
        let mut stat_opt = git2::StatusOptions::new();
        stat_opt.include_untracked(true);
        for ign in ignores {
            stat_opt.pathspec("/".to_owned() + ign.as_ref());
        }
        let stat = self.repo.statuses(Some(&mut stat_opt))?;
        Ok(stat.is_empty())
    }

    pub fn all_pending(&self) -> Result<git2::Statuses<'_>, git2::Error> {
        let mut stat_opt = git2::StatusOptions::new();
        stat_opt.include_untracked(true);
        stat_opt.recurse_untracked_dirs(true);
        self.repo.statuses(Some(&mut stat_opt))
    }

    // TODO: convert to iterator form
    pub fn walk_paths_pre_order<C>(&self, mut callback: C) -> Result<(), git2::Error>
    where
        C: FnMut(String) -> git2::TreeWalkResult,
    {
        let head = self.repo.head()?;
        let head_tree = head.peel_to_tree()?;
        head_tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
            if entry.kind() != Some(git2::ObjectType::Blob) {
                return git2::TreeWalkResult::Ok;
            }
            let name = entry.name().unwrap();
            callback(root.to_string() + name)
        })
    }
}
