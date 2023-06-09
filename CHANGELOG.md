# Version 0.4.2 (2023-07-04)

Updating dependencies, specifically axoasset, to remove OpenSSL dependency.


# Version 0.4.1 (2023-05-23)

Just updating deps to get improvements to axoasset

# Version 0.4.0 (2023-05-19)

* the "root_dir" argument has been made the second argument in a ton of APIs, and renamed to "clamp_to_dir"
* the find_file API has been factored out and exposed as public
* Broken now includes a path to the manifest we found, to help with error messages and disambiguation

# Version 0.3.0 (2023-04-24)

* Added support for Cargo and NPM manifest keywords. For Cargo projects specifically, these will be squashed together
  with the categories field for now, since the dual design is very unique to crates.io specifically.

# Version 0.2.0 (2023-04-10)

This version reworks the design of the primary interface:

* get_project(s) is now called get_workspace(s) to be more precise
* get_workspaces no longer picks the "best" project for you, and instead returns results for all of them
* to help you make sense of those results, they are now wrapped in a WorkspaceSearch enum that can either be:
  * "Found": we found and parsed the workspace, here it is
  * "Missing": we found no evidence of a workspace (no Cargo.toml)
  * "Broken": we found a manifest but failed to make sense of it (parse error, missing/weird values, etc.)
* it now takes an optional "root" argument that specifies a root dir that we want to constrain the search to.
  most users can set this to None to just ignore the feature.
  * this isn't perfectly well-defined yet when a manifest is found under the root dir, but the root of the workspace
    is outside the root dir. this is fine for the intended purpose of clamping to a git checkout which presumably is
    completely self-contained as far as workspaces are concerned.
* there is now a CLI app version of axoproject with json output
* we now detect cstaticlibs and cdylibs in addition to binaries (in separate fields so if you don't care about them nothing has changed)

In addition the internals have been significantly reworked into separate libraries like axoasset and axocli so more of our tools can share logic.


# Version 0.1.0 (2023-03-27)

Initial release!
