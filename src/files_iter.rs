use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use globset::{Glob, GlobSet, GlobSetBuilder};
use indexmap::IndexSet;
use log::debug;
use nom::{
    Finish, IResult, Parser,
    bytes::complete::{escaped_transform, is_not},
    character::complete::one_of,
};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
pub struct RootIteratorPackage {
    globset: GlobSet,
    pattern_paths: Vec<(PathBuf, bool)>,
    root_paths: Vec<PathBuf>,
}

pub fn root_iterator_package<IR, IP, R, P>(
    roots: IR,
    patterns: IP,
) -> anyhow::Result<RootIteratorPackage>
where
    IR: IntoIterator<Item = R>,
    R: AsRef<Path>,
    IP: IntoIterator<Item = P>,
    P: AsRef<str>,
{
    let iter = patterns.into_iter().filter_map(|p| {
        let pattern = p.as_ref();

        // TODO: if there is no greatest common root, still need to yeild the pattern.
        // If the path is empty, also skip it, but yeild the pattern.
        // Might want to check if the pattern is an empty string.
        let (root_path, skippable) = greatest_root_path(pattern).ok()?;
        let glob = Glob::new(pattern).ok()?;

        Some((root_path, glob, skippable))
    });

    let mut globset_builder = GlobSetBuilder::new();
    let mut pattern_paths = IndexSet::new();

    // TOOD: Check if path exists before adding it to the collection.
    // For each pattern, we want to do one of the following:
    // - have M * N patterns, where M is the number of roots, and N is the number of patterns.
    //   - Append each root to each pattern for Windows machines.
    // - if a pattern starts with a `RootDir` or a `Prefix` + `RootDir` component, be smarter on how we
    //  check if we've visited a folder.
    for (path, glob, skippable) in iter {
        pattern_paths.insert((path, skippable));
        globset_builder.add(glob);
    }

    let globset = globset_builder.build()?;
    let package = RootIteratorPackage {
        globset,
        pattern_paths: pattern_paths.into_iter().collect(),
        root_paths: roots.into_iter().map(|p| p.as_ref().to_owned()).collect(),
    };

    Ok(package)
}

#[fauxgen::generator(yield = DirEntry)]
pub fn root_iterator(package: RootIteratorPackage) {
    let mut skip_paths = HashSet::new();

    // Iterate over the root paths parsed from the glob patterns
    for (path, skippable) in package.pattern_paths {
        if skippable {
            skip_paths.insert(path.clone());
        }

        let iter = WalkDir::new(path)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| package.globset.is_match(entry.path()));

        for entry in iter {
            r#yield!(entry);
        }
    }

    // Iterate over root paths where patterns _don't_ match the
    // previously matched root patterns, and skip over any
    // directory that we know was wildcard matched already.
    for path in package.root_paths {
        let iter = WalkDir::new(path)
            .into_iter()
            .filter_entry(|entry| {
                let path = entry.path();

                #[cfg(any(target_os = "macos", target_os = "ios"))]
                {
                    if path == Path::new("/dev") {
                        return false;
                    }
                }

                #[cfg(any(target_os = "linux", target_os = "android"))]
                {
                    if path == Path::new("/dev")
                        || path == Path::new("/proc")
                        || path == Path::new("/sys")
                    {
                        return false;
                    }
                }

                !skip_paths.contains(path)
            })
            .filter_map(Result::ok)
            .filter(|entry| !package.globset.is_match(entry.path()));

        for entry in iter {
            r#yield!(entry);
        }
    }
}

fn greatest_root_path(input: &str) -> anyhow::Result<(PathBuf, bool)> {
    let (remaining, path) = root_parser(input)
        .finish()
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let skippable = matches!(remaining, "*" | "**/*");

    // Strip trailing slash
    let path_ref = if path.ends_with('/') {
        &path[..path.len() - 1]
    } else {
        &path
    };

    debug!("Remaining: `{remaining}`");

    Ok((path_ref.into(), skippable))
}

fn root_parser(input: &str) -> IResult<&str, String> {
    escaped_transform(is_not("\\*![]{}"), '\\', one_of("*![]{}")).parse(input)
}
