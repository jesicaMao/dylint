use anyhow::{anyhow, Context, Result};
use dylint_internal::{
    clippy_utils::{clippy_utils_package_version, toolchain_channel},
    clone,
    git2::{Commit, ObjectType, Oid, Repository},
};
use if_chain::if_chain;
use std::{cell::RefCell, rc::Rc};
use tempfile::{tempdir, TempDir};

const RUST_CLIPPY_URL: &str = "https://github.com/rust-lang/rust-clippy";

#[derive(Debug, Eq, PartialEq)]
pub struct Rev {
    pub version: String,
    pub channel: String,
    pub oid: Oid,
}

pub struct Revs {
    repository: Rc<Repository>,
}

pub struct RevIter<'revs> {
    repository: Rc<Repository>,
    commit: Commit<'revs>,
    curr_rev: Option<Rev>,
}

impl Revs {
    pub fn new(quiet: bool) -> Result<Self> {
        let repository = clippy_repository(quiet)?;

        Ok(Self { repository })
    }

    #[allow(clippy::iter_not_returning_iterator)]
    pub fn iter(&self) -> Result<RevIter> {
        let path = self
            .repository
            .workdir()
            .ok_or_else(|| anyhow!("Could not get working directory"))?;
        let object = {
            let head = self.repository.head()?;
            let oid = head.target().ok_or_else(|| anyhow!("Could not get HEAD"))?;
            self.repository.find_object(oid, Some(ObjectType::Commit))?
        };
        let commit = object
            .as_commit()
            .ok_or_else(|| anyhow!("Object is not a commit"))?;
        let version = clippy_utils_package_version(path)?;
        let channel = toolchain_channel(path)?;
        let oid = commit.id();
        Ok(RevIter {
            repository: self.repository.clone(),
            commit: commit.clone(),
            curr_rev: Some(Rev {
                version,
                channel,
                oid,
            }),
        })
    }
}

impl Iterator for RevIter<'_> {
    type Item = Result<Rev>;

    // smoelius: I think it is okay to ignore the `non_local_effect_before_error_return` warning
    // here. If `self.commit` were not updated, the same commits would be traversed the next time
    // `next` was called.
    #[cfg_attr(dylint_lib = "general", allow(non_local_effect_before_error_return))]
    fn next(&mut self) -> Option<Self::Item> {
        (|| -> Result<Option<Rev>> {
            let mut prev_rev: Option<Rev> = None;
            loop {
                let curr_rev = if let Some(rev) = self.curr_rev.take() {
                    rev
                } else {
                    let path = self
                        .repository
                        .workdir()
                        .ok_or_else(|| anyhow!("Could not get working directory"))?;
                    // smoelius: Note that this is not `git log`'s default behavior. Rather, this
                    // behavior corresponds to:
                    //   git log --first-parent
                    let commit = if let Some(commit) = self.commit.parents().next() {
                        self.commit = commit.clone();
                        commit
                    } else {
                        return Ok(None);
                    };
                    self.repository
                        .checkout_tree(commit.as_object(), None)
                        .with_context(|| {
                            format!("`checkout_tree` failed for `{:?}`", commit.as_object())
                        })?;
                    self.repository
                        .set_head_detached(commit.id())
                        .with_context(|| {
                            format!("`set_head_detached` failed for `{}`", commit.id())
                        })?;
                    let version = clippy_utils_package_version(path)?;
                    let channel = toolchain_channel(path)?;
                    let oid = commit.id();
                    Rev {
                        version,
                        channel,
                        oid,
                    }
                };
                if_chain! {
                    if let Some(prev_rev) = prev_rev;
                    if prev_rev.version != curr_rev.version;
                    then {
                        self.curr_rev = Some(curr_rev);
                        return Ok(Some(prev_rev));
                    }
                }
                prev_rev = Some(curr_rev);
            }
        })()
        .transpose()
    }
}

// smoelius: `thread_local!` because `git2::Repository` cannot be shared between threads safely.
thread_local! {
    static TMPDIR_AND_REPOSITORY: RefCell<Option<(TempDir, Rc<Repository>)>> = const { RefCell::new(None) };
}

pub fn clippy_repository(quiet: bool) -> Result<Rc<Repository>> {
    TMPDIR_AND_REPOSITORY.with_borrow_mut(|cell| {
        if let Some((_, repository)) = cell {
            return Ok(repository.clone());
        }

        let tempdir = tempdir().with_context(|| "`tempdir` failed")?;

        let repository = clone(RUST_CLIPPY_URL, "master", tempdir.path(), quiet).map(Rc::new)?;

        cell.replace((tempdir, repository.clone()));

        Ok(repository)
    })
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod test {
    use super::*;
    use once_cell::sync::Lazy;

    static EXAMPLES: Lazy<[Rev; 6]> = Lazy::new(|| {
        [
            Rev {
                version: "0.1.65".to_owned(),
                channel: "nightly-2022-08-11".to_owned(),
                oid: Oid::from_str("2b2190cb5667cdd276a24ef8b9f3692209c54a89").unwrap(),
            },
            Rev {
                version: "0.1.64".to_owned(),
                channel: "nightly-2022-06-30".to_owned(),
                oid: Oid::from_str("0cb0f7636851f9fcc57085cf80197a2ef6db098f").unwrap(),
            },
            // smoelius: 0.1.62 and 0.1.63 omitted (for no particular reason).
            Rev {
                version: "0.1.61".to_owned(),
                channel: "nightly-2022-02-24".to_owned(),
                oid: Oid::from_str("7b2896a8fc9f0b275692677ee6d2d66a7cbde16a").unwrap(),
            },
            Rev {
                version: "0.1.60".to_owned(),
                channel: "nightly-2022-01-13".to_owned(),
                oid: Oid::from_str("97a5daa65908e59744e2bc625b14849352231c75").unwrap(),
            },
            Rev {
                version: "0.1.59".to_owned(),
                channel: "nightly-2021-12-02".to_owned(),
                oid: Oid::from_str("392b0c5c25ddbd36e4dc480afcf70ed01dce352d").unwrap(),
            },
            Rev {
                version: "0.1.58".to_owned(),
                channel: "nightly-2021-10-21".to_owned(),
                oid: Oid::from_str("91496c2ac6abf6454c413bb23e8becf6b6dc20ea").unwrap(),
            },
        ]
    });

    #[test]
    fn examples() {
        for example in &*EXAMPLES {
            let revs = Revs::new(false).unwrap();
            let mut iter = revs.iter().unwrap();
            let rev = iter
                .find(|rev| {
                    rev.as_ref()
                        .map_or(true, |rev| rev.version == example.version)
                })
                .unwrap()
                .unwrap();
            assert_eq!(rev, *example);
        }
    }
}
