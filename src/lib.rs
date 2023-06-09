//! Shared code for gathering up information about a workspace, used by various axo.dev tools
//! like cargo-dist and oranda.
//!
//! The main entry point is [`get_workspaces`][].

#![deny(missing_docs)]
#![allow(clippy::result_large_err)]

use std::fmt::Display;

use axoasset::{AxoassetError, LocalAsset};
use camino::{Utf8Path, Utf8PathBuf};
use errors::{AxoprojectError, Result};
use tracing::info;

pub mod errors;
#[cfg(feature = "npm-projects")]
pub mod javascript;
#[cfg(feature = "cargo-projects")]
pub mod rust;
#[cfg(test)]
mod tests;

/// Information about various kinds of workspaces
pub struct Workspaces {
    /// Info about the cargo/rust workspace
    #[cfg(feature = "cargo-projects")]
    pub rust: WorkspaceSearch,
    /// Info about the npm/js workspace
    #[cfg(feature = "npm-projects")]
    pub javascript: WorkspaceSearch,
}

impl Workspaces {
    #[cfg(test)]
    pub(crate) fn best(self) -> Option<WorkspaceInfo> {
        #![allow(clippy::vec_init_then_push)]

        let mut best_project = None;
        let mut max_depth = 0;
        let mut projects = vec![];

        // FIXME: should we provide feedback/logging here?
        #[cfg(feature = "cargo-projects")]
        projects.push(self.rust);

        #[cfg(feature = "npm-projects")]
        projects.push(self.javascript);

        // If we find multiple projects, prefer the one deeper in the file system
        // (the one closer to the start_dir).
        for project in projects {
            let WorkspaceSearch::Found(project) = project else {
                continue;
            };
            let depth = project.workspace_dir.ancestors().count();
            if depth > max_depth {
                best_project = Some(project);
                max_depth = depth;
            }
        }

        best_project
    }
}

/// Result of searching for a particular kind of workspace
pub enum WorkspaceSearch {
    /// We found it
    Found(WorkspaceInfo),
    /// We found something that looks like a workspace but there's something wrong with it
    Broken {
        /// Path to the closest manifest we found.
        ///
        /// Note that for workspaces we may have had a parsing error with a different file,
        /// but at least this is the file we found that made us discover that workspace!
        manifest_path: Utf8PathBuf,
        /// The error we encountered
        cause: AxoprojectError,
    },
    /// We found no hint of this kind of workspace
    Missing(AxoprojectError),
}

/// Kind of workspace
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorkspaceKind {
    /// cargo/rust workspace
    #[cfg(feature = "cargo-projects")]
    Rust,
    /// npm/js workspace
    #[cfg(feature = "npm-projects")]
    Javascript,
}

/// Info on the current workspace
///
/// This can either be a cargo workspace or an npm workspace, the concepts
/// are conflated to let users of axoproject handle things more uniformly.
pub struct WorkspaceInfo {
    /// The kinf of workspace this is (Rust or Javascript)
    pub kind: WorkspaceKind,
    /// The directory where build output will go (generally `target/`)
    pub target_dir: Utf8PathBuf,
    /// The root directory of the workspace (where the root Cargo.toml is)
    pub workspace_dir: Utf8PathBuf,
    /// Computed info about the packages.
    ///
    /// This notably includes finding readmes and licenses even if the user didn't
    /// specify their location -- something Cargo does but Guppy (and cargo-metadata) don't.
    pub package_info: Vec<PackageInfo>,
    /// Path to the root manifest of the workspace
    ///
    /// This can be either a Cargo.toml or package.json. In either case this manifest
    /// may or may not represent a "real" package. Both systems have some notion of
    /// "virtual" manifest which exists only to list the actual packages in the workspace.
    pub manifest_path: Utf8PathBuf,
    /// A consensus URL for the repo according to the packages in the workspace
    ///
    /// If there are multiple packages in the workspace that specify a repository
    /// but they disagree, this will be None.
    pub repository_url: Option<String>,
    /// If the workspace root has some auto-includeable files, here they are!
    ///
    /// This is currently what is use for top-level Announcement contents.
    pub root_auto_includes: AutoIncludes,
    /// Non-fatal issues that were encountered and should probably be reported
    pub warnings: Vec<AxoprojectError>,
    /// Raw cargo `[workspace.metadata]` table
    #[cfg(feature = "cargo-projects")]
    pub cargo_metadata_table: Option<serde_json::Value>,
    /// Any [profile.*] entries we found in the root Cargo.toml
    #[cfg(feature = "cargo-projects")]
    pub cargo_profiles: rust::CargoProfiles,
}

impl WorkspaceInfo {
    /// Get a package
    pub fn package(&self, idx: PackageIdx) -> &PackageInfo {
        &self.package_info[idx.0]
    }
    /// Get a mutable package
    pub fn package_mut(&mut self, idx: PackageIdx) -> &PackageInfo {
        &mut self.package_info[idx.0]
    }
    /// Iterate over packages
    pub fn packages(&self) -> impl Iterator<Item = (PackageIdx, &PackageInfo)> {
        self.package_info
            .iter()
            .enumerate()
            .map(|(i, k)| (PackageIdx(i), k))
    }
}

/// Computed info about a package
///
/// This notably includes finding readmes and licenses even if the user didn't
/// specify their location -- something Cargo does but Guppy (and cargo-metadata) don't.
#[derive(Debug)]
pub struct PackageInfo {
    /// Path to the manifest for this package
    pub manifest_path: Utf8PathBuf,
    /// Path to the root dir for this package
    pub package_root: Utf8PathBuf,
    /// Name of the package
    ///
    /// This can actually be missing for JS packages, but in that case it's basically
    /// the same thing as a "virtual manifest" in Cargo. PackageInfo is only for concrete
    /// packages so we don't need to allow for that.
    pub name: String,
    /// Version of the package
    ///
    /// Both cargo and npm use SemVer but they disagree slightly on what that means:
    ///
    /// * cargo: <https://crates.io/crates/semver>
    /// * npm: <https://crates.io/crates/node-semver>
    ///
    /// Cargo requires this field at all times, npm only requires it to publish.
    /// Probably we could get away with making it non-optional but allowing this
    /// theoretically lets npm users "kick the tires" even when they're not ready
    /// to publish.
    pub version: Option<Version>,
    /// A brief description of the package
    pub description: Option<String>,
    /// Authors of the package (may be empty)
    pub authors: Vec<String>,
    /// The license the package is provided under
    pub license: Option<String>,
    /// False if they set publish=false, true otherwise
    ///
    /// Currently always true for npm packages.
    pub publish: bool,
    /// Package keywords AND/OR categories.
    ///
    /// Specifically, Cargo has both the notion
    /// of a "package keyword" (free-form text) and a "package category" (one of circa 70
    /// predefined categories accepted by crates.io). We don't really care about validating
    /// these, though, and just squash them together with the keywords.
    pub keywords: Option<Vec<String>>,
    /// URL to the repository for this package
    ///
    /// This URL can be used by various CI/Installer helpers. In the future we
    /// might also use it for auto-detecting "hey you're using github, here's the
    /// recommended github setup".
    ///
    /// i.e. `cargo dist init --installer=shell` uses this as the base URL for fetching from
    /// a Github Release™️.
    pub repository_url: Option<String>,
    /// URL to the homepage for this package.
    ///
    /// Currently this isn't terribly important or useful?
    pub homepage_url: Option<String>,
    /// URL to the documentation for this package.
    ///
    /// This will default to docs.rs if not specified, which is the default crates.io behaviour.
    ///
    /// Currently this isn't terribly important or useful?
    pub documentation_url: Option<String>,
    /// Path to the README file for this package.
    ///
    /// If the user specifies where this is, we'll respect it. Otherwise we'll try to find
    /// this in the workspace using AutoIncludes.
    pub readme_file: Option<Utf8PathBuf>,
    /// Paths to the LICENSE files for this package.
    ///
    /// By default these should be copied into a zip containing this package's binary.
    ///
    /// If the user specifies where this is, we'll respect it. Otherwise we'll try to find
    /// this in the workspace using AutoIncludes.
    ///
    /// Cargo only lets you specify one such path, but that's because its license-path
    /// key primarily exists as an escape hatch for someone's whacky-wild custom license.
    /// Ultimately Cargo's license-path is inadequate for Normal Licenses because it
    /// can't handle the standard pattern of dual licensing MIT/Apache and having two
    /// license files. AutoIncludes properly handles dual licensing.
    pub license_files: Vec<Utf8PathBuf>,
    /// Paths to the CHANGELOG or RELEASES file for this package
    ///
    /// By default this should be copied into a zip containing this package's binary.
    ///
    /// We will *try* to parse this
    pub changelog_file: Option<Utf8PathBuf>,
    /// Names of binaries this package defines
    ///
    /// For Cargo this is currently properly computed in all its complexity.
    /// For JS I *think* this is computed in its full complexity but Tests Needed
    /// and also there's so many ways to define things who can ever be sure.
    pub binaries: Vec<String>,
    /// Names of C-style staticlibs (.a) this library defines.
    ///
    /// For Cargo this is currently properly computed in all its complexity.
    /// For JS we don't compute this at all.
    pub cstaticlibs: Vec<String>,
    /// Names of C-style dylibs (.dll, .so, ...) this package defines
    ///
    /// For Cargo this is currently properly computed in all its complexity.
    /// For JS we don't compute this at all.
    pub cdylibs: Vec<String>,
    /// Raw cargo `[package.metadata]` table
    #[cfg(feature = "cargo-projects")]
    pub cargo_metadata_table: Option<serde_json::Value>,
    /// A unique id used by Cargo to refer to the package
    #[cfg(feature = "cargo-projects")]
    pub cargo_package_id: Option<guppy::PackageId>,
}

/// An id for a [`PackageInfo`][] entry in a [`WorkspaceInfo`][].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageIdx(pub usize);

/// A Version abstracted over project type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    /// cargo version
    #[cfg(feature = "cargo-projects")]
    Cargo(semver::Version),
    /// npm version
    #[cfg(feature = "npm-projects")]
    Npm(node_semver::Version),
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(feature = "cargo-projects")]
            Version::Cargo(v) => v.fmt(f),
            #[cfg(feature = "npm-projects")]
            Version::Npm(v) => v.fmt(f),
        }
    }
}

impl Version {
    /// Assume it's a cargo Version
    #[cfg(feature = "cargo-projects")]
    pub fn cargo(&self) -> &semver::Version {
        #[allow(irrefutable_let_patterns)]
        if let Version::Cargo(v) = self {
            v
        } else {
            panic!("Version wasn't in the cargo format")
        }
    }

    /// Assume it's an npm Version
    #[cfg(feature = "npm-projects")]
    pub fn npm(&self) -> &node_semver::Version {
        #[allow(irrefutable_let_patterns)]
        if let Version::Npm(v) = self {
            v
        } else {
            panic!("Version wasn't in the npm format")
        }
    }
}

/// Various files we might want to auto-include
#[derive(Debug, Clone)]
pub struct AutoIncludes {
    /// README
    pub readme: Option<Utf8PathBuf>,
    /// LICENSE/UNLICENSE
    pub licenses: Vec<Utf8PathBuf>,
    /// CHANGELOG/RELEASES
    pub changelog: Option<Utf8PathBuf>,
}

/// Tries to find information about the workspace at start_dir, walking up
/// ancestors as necessary until we reach root_dir (or run out of ancestors).
///
/// Behaviour is unspecified if only part of the workspace is nested in root_dir.
///
/// In the future setting root_dir may cause the output's paths to be relative
/// to that directory, but for now they're always absolute. The cli does this
/// relativizing, but not the library.
///
/// This can be either a cargo project or an npm project. Support for each
/// one is behind feature flags:
///
/// * cargo-projects
/// * npm-projects
///
/// Concepts of both will largely be conflated, the only distinction will be
/// the top level [`WorkspaceKind`][].
pub fn get_workspaces(start_dir: &Utf8Path, clamp_to_dir: Option<&Utf8Path>) -> Workspaces {
    Workspaces {
        #[cfg(feature = "cargo-projects")]
        rust: rust::get_workspace(start_dir, clamp_to_dir),
        #[cfg(feature = "npm-projects")]
        javascript: javascript::get_workspace(start_dir, clamp_to_dir),
    }
}

/// Find auto-includeable files in a dir
///
/// This includes:
///
/// * reamde: `README*`
/// * license: `LICENSE*` and `UNLICENSE*`
/// * changelog: `CHANGELOG*` and `RELEASES*`
///
/// This doesn't look at parent/child dirs, and doesn't factor in user provided paths.
/// Handle those details by using [`merge_auto_includes`][] to merge the results into a [`PackageInfo`].
pub fn find_auto_includes(dir: &Utf8Path) -> Result<AutoIncludes> {
    find_auto_includes_inner(dir).map_err(|details| AxoprojectError::AutoIncludeSearch {
        dir: dir.to_owned(),
        details,
    })
}

fn find_auto_includes_inner(dir: &Utf8Path) -> std::result::Result<AutoIncludes, std::io::Error> {
    // Is there a better way to get the path to sniff?
    // Should we spider more than just package_root and workspace_root?
    // Should we more carefully prevent grabbing LICENSES from both dirs?
    // Should we not spider the workspace root for README since Cargo has a proper field for this?
    // Should we check for a "readme=..." on the workspace root Cargo.toml?

    let mut includes = AutoIncludes {
        readme: None,
        licenses: vec![],
        changelog: None,
    };

    // Iterate over files in the dir
    let entries = dir.read_dir_utf8()?;

    for entry in entries {
        // Make sure it's a file
        //
        // I think this *may* mishandle symlinks, Rust's docs have some notes that
        // the only reliable way to check if something is a file is to try to Open it,
        // but honestly I don't super care about someone symlinking a README???
        let entry = entry?;
        let meta = entry.file_type()?;
        if !meta.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        if file_name.starts_with("README") {
            // Found a readme! It doesn't really make sense to have multiple of these,
            // so we just need to pick one (probably will never be stressed...)
            if includes.readme.is_none() {
                let path = entry.path().to_owned();
                info!("Found README at {}", path);
                includes.readme = Some(path);
            } else {
                info!("Ignoring duplicate candidate README at {}", entry.path());
            }
        } else if file_name.starts_with("LICENSE") || file_name.starts_with("UNLICENSE") {
            // Found a license! Dual licensing means we will often have multiple of these,
            // so we should grab every one we can find!
            let path = entry.path().to_owned();
            info!("Found LICENSE at {}", path);
            includes.licenses.push(path);
        } else if file_name.starts_with("CHANGELOG") || file_name.starts_with("RELEASES") {
            // Found a changelog! It doesn't really make sense to have multiple of these,
            // so we just need to pick one? Might one day become untrue if we work out
            // how to do changelogs for independently versioned/released monorepos.
            if includes.changelog.is_none() {
                let path = entry.path().to_owned();
                info!("Found CHANGELOG at {}", path);
                includes.changelog = Some(path);
            } else {
                info!("Ignoring duplicate candidate CHANGELOG at {}", entry.path());
            }
        }
    }

    Ok(includes)
}

/// Merge AutoIncluded files into PackageInfo, preferring already existing values
/// over the AutoIncludes. The expected way to use this is:
///
/// 1. Compute PackageInfo from a manifest, populate fields with user-provided paths
/// 2. Compute AutoIncludes for the package's root dir, merge them in
/// 3. Compute AutoIncludes for the workspace's root dir, merge them in
///
/// This naturally cascades results.
pub fn merge_auto_includes(info: &mut PackageInfo, auto_includes: &AutoIncludes) {
    if info.readme_file.is_none() {
        info.readme_file = auto_includes.readme.clone();
    }
    if info.changelog_file.is_none() {
        info.changelog_file = auto_includes.changelog.clone();
    }
    // Note that even though we allow for multiple licenses, it's supremely wonky
    // to source them from multiple locations, so if any source provides a license
    // we will ignore all the other ones.
    if info.license_files.is_empty() {
        info.license_files = auto_includes.licenses.clone();
    }
}

/// Find a file with the given name, starting at the given dir and walking up to ancestor dirs,
/// optionally clamped to a given ancestor dir
pub fn find_file(
    name: &str,
    start_dir: &Utf8Path,
    clamp_to_dir: Option<&Utf8Path>,
) -> Result<Utf8PathBuf> {
    let manifest = LocalAsset::search_ancestors(start_dir, name)?;

    if let Some(root_dir) = clamp_to_dir {
        let root_dir = if root_dir.is_relative() {
            let current_dir = LocalAsset::current_dir()?;
            current_dir.join(root_dir)
        } else {
            root_dir.to_owned()
        };

        let improperly_nested = pathdiff::diff_utf8_paths(&manifest, root_dir)
            .map(|p| p.starts_with(".."))
            .unwrap_or(true);

        if improperly_nested {
            return Err(AxoassetError::SearchFailed {
                start_dir: start_dir.to_owned(),
                desired_filename: name.to_owned(),
            })?;
        }
    }

    Ok(manifest)
}

/*

cargo-dist utils we might want to factor out here, haven't thought about it yet


/// Load a changelog to a string
fn try_load_changelog(changelog_path: &Utf8Path) -> Result<String> {
    let file = File::open(changelog_path)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to open changelog at {changelog_path}"))?;
    let mut data = BufReader::new(file);
    let mut changelog_str = String::new();
    data.read_to_string(&mut changelog_str)
        .into_diagnostic()
        .wrap_err_with(|| format!("failed to read changelog at {changelog_path}"))?;
    Ok(changelog_str)
}

/// Tries to find a changelog entry with the exact version given and returns its title and notes.
fn try_extract_changelog_exact(
    changelogs: &parse_changelog::Changelog,
    version: &Version,
) -> Option<(String, String)> {
    let version_string = format!("{}", version);

    changelogs.get(&*version_string).map(|release_notes| {
        (
            release_notes.title.to_string(),
            release_notes.notes.to_string(),
        )
    })
}

/// Tries to find a changelog entry that matches the given version's normalized form. That is, just
/// the `major.minor.patch` part. If successful, the entry's title is modified to include the
/// version's prerelease part before it is returned together with the notes.
///
/// Noop if the given version is already normalized.
fn try_extract_changelog_normalized(
    changelogs: &parse_changelog::Changelog,
    version: &Version,
) -> Option<(String, String)> {
    if version.pre.is_empty() {
        return None;
    }

    let version_normalized = Version::new(version.major, version.minor, version.patch);
    let version_normalized_string = format!("{}", version_normalized);

    let release_notes = changelogs.get(&*version_normalized_string)?;

    // title looks something like '<prefix><version><freeform>'
    // prefix could be 'v' or 'Version ' for example
    let (prefix_and_version, freeform) = release_notes.title.split_at(
        release_notes
            .title
            .find(&*version_normalized_string)
            .unwrap() // impossible that this version string is not present in the header
            + version_normalized_string.len(),
    );

    // insert prerelease suffix into the title
    let title = format!(
        "{}-{} {}",
        prefix_and_version.trim(),
        version.pre,
        freeform.trim()
    );

    Some((title.trim().to_string(), release_notes.notes.to_string()))
}
 */
