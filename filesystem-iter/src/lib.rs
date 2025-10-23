use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use anyhow::Ok;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use indexmap::IndexSet;
use walkdir::{DirEntry, WalkDir};

pub mod file_offline;
pub mod parse_mounts;

#[derive(Debug)]
pub struct RootIteratorPackage {
    globset: GlobSet,
    pattern_paths: Vec<(PathBuf, bool)>,
    root_paths: Vec<PathBuf>,
}

// Ok past self, why do this root iterator package stuff?
// I should just have a single parser for *all* paths
// - might have to be multi-stage
// - first add them all to a big hashset to dedupe raw patterns
// - construct a hashmap of all processed patterns that have gone through
//      `root_parser` of type HashMap<Option(&Path), HashSet<&str>> (maybe `root_parser` changes
//            and only returns the remainder)
//      - the `None` case is for all loose patterns that have no root
//      - the set of `str` is all the pattern remainders
// - if an entry's set contains `**` or `**/*`, discard all other remainders.
//
// Basically just go ham on pattern set minimization.

// ... ok I'm trying to solve two problems here:
//      - get a set of root paths from all the patterns (including paths that resolve to files, aka
//          ones that have no special glob characters)
//          - for said root paths, I want to know if everything in that root can be matched or not
//              as an optimization for match/not-match logic when I go back and iterate over `/`
//      - glob pattern minimization
//          - if certain patterns are more generous in their matching, aka two patterns that share
//              the same root, but one ends with `**` earlier in its path, discard all others.
pub fn parse_patterns<IP, P>(patterns: IP)
where
    P: AsRef<str>,
    IP: IntoIterator<Item = P>,
{
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
        let pattern = p.as_ref().trim();
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .build()
            .ok()?;
        let (root_path, match_all_recurse) = root_parser(pattern)?;

        Some((root_path.into(), glob, match_all_recurse))
    });

    let mut globset_builder = GlobSetBuilder::new();
    let mut pattern_paths = IndexSet::new();

    // TOOD: Check if path exists before adding it to the collection.
    // For each pattern, we want to do one of the following:
    // - have M * N patterns, where M is the number of roots, and N is the number of patterns.
    //   - Append each root to each pattern for Windows machines.
    // - if a pattern starts with a `RootDir` or a `Prefix` + `RootDir` component, be smarter on how we
    //  check if we've visited a folder.
    for (path, glob, match_all_recurse) in iter {
        pattern_paths.insert((path, match_all_recurse));
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
    for (path, recursive_match_all) in package.pattern_paths {
        if recursive_match_all {
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

struct RootParserData<'a> {
    root: &'a Path,
    recursive_match_all: bool,
    remainder: &'a str,
}

fn root_parser(input: &str) -> Option<(&Path, bool)> {
    // Early return on some trivial patterns
    match input {
        "" => return None,
        "**" | "**/*" | "/**" | "/**/*" => return Some((Path::new("/"), true)),
        _ => {}
    }

    // If you're not a trivial pattern, and are missing a starting slash,
    // pattern has no parseable root.
    if !input.starts_with("/") {
        return None;
    }

    // Start the meat of the pattern processing
    let mut index = 0;
    let mut last_char = None;
    let mut last_separator = None;

    for char in input.chars() {
        match char {
            '/' => last_separator = Some(index),
            '*' | '[' | ']' | '{' | '}' | '?' | '!' if last_char != Some('\\') => break,
            _ => {}
        }

        last_char = Some(char);
        index += 1;
    }

    if index == input.len() {
        Some((Path::new(input), is_recursive_match_all(input)))
    } else if let Some(last) = last_separator {
        let end = last + 1;

        Some((
            Path::new(&input[..end]),
            is_recursive_match_all(&input[end..]),
        ))
    } else {
        None
    }
}

fn is_recursive_match_all(input: &str) -> bool {
    matches!(input, "**" | "**/*")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_parser() {
        let input = "/foo/bar/*/test";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/"), false)), actual);

        let input = "/foo/bar/**/test";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/"), false)), actual);

        // SHOULD THIS FAIL? Technically there is no greatest
        // root, as the whole thing has a single match,
        // **HOWEVER** it matches a very specific file with
        // a fully qualified path
        let input = "/foo/bar/test";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/test"), false)), actual);

        // SHOULD THIS FAIL? Technically there is no greatest
        // root, as the whole thing has a single match...
        let input = "hello";
        let actual = root_parser(input);
        assert_eq!(None, actual);

        let input = "ohhell*there";
        let actual = root_parser(input);
        assert_eq!(None, actual);

        let input = "/foo/bar/*/test/**/*";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/"), false)), actual);

        let input = "/foo/bar/**";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/"), true)), actual);

        let input = "/foo/bar/*";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/"), false)), actual);

        let input = "/foo/bar/**/*";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/"), true)), actual);

        let input = "/foo/bar/\\*";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/*"), false)), actual);

        let input = "/foo/bar/\\*/hello";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/foo/bar/*/hello"), false)), actual);

        let input = "**/*";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/"), true)), actual);

        let input = "**";
        let actual = root_parser(input);
        assert_eq!(Some((Path::new("/"), true)), actual);

        let input = "";
        let actual = root_parser(input);
        assert_eq!(None, actual);
    }
}
