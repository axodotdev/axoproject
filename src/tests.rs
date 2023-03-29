use crate::WorkspaceKind;

#[cfg(feature = "cargo-projects")]
#[test]
fn test_self_detect() {
    let project = crate::get_project("./".into()).unwrap();
    assert_eq!(project.kind, WorkspaceKind::Rust);
    assert_eq!(project.package_info.len(), 1);

    let package = &project.package_info[0];
    assert_eq!(package.name, "axoproject");
    assert_eq!(package.binaries.len(), 1);

    let binary = &package.binaries[0];
    assert_eq!(binary, "axoproject");
}

#[cfg(feature = "cargo-projects")]
#[test]
fn test_cargo_new() {
    let project = crate::get_project("tests/projects/cargo-new/src/".into()).unwrap();
    assert_eq!(project.kind, WorkspaceKind::Rust);
    assert_eq!(project.package_info.len(), 1);

    let package = &project.package_info[0];
    assert_eq!(package.name, "cargo-new");
    assert_eq!(package.binaries.len(), 1);

    let binary = &package.binaries[0];
    assert_eq!(binary, "cargo-new");
}

#[cfg(feature = "cargo-projects")]
#[test]
fn test_cargo_virtual() {
    let project = crate::get_project("tests/projects/cargo-virtual/virtual/".into()).unwrap();
    assert_eq!(project.kind, WorkspaceKind::Rust);
    assert_eq!(project.package_info.len(), 3);

    {
        let package = &project.package_info[0];
        assert_eq!(package.name, "virtual");
        assert_eq!(&package.binaries[..], &["virtual"]);
    }

    {
        let package = &project.package_info[1];
        assert_eq!(package.name, "some-lib");
        assert!(package.binaries.is_empty());
    }

    {
        let package = &project.package_info[2];
        assert_eq!(package.name, "virtual-gui");
        assert_eq!(&package.binaries[..], &["virtual-gui"]);
    }
}

#[cfg(feature = "cargo-projects")]
#[test]
fn test_cargo_nonvirtual() {
    let project = crate::get_project("tests/projects/cargo-nonvirtual/".into()).unwrap();
    assert_eq!(project.kind, WorkspaceKind::Rust);
    assert_eq!(project.package_info.len(), 6);

    {
        let package = &project.package_info[0];
        assert_eq!(package.name, "some-cdylib");
        assert!(package.binaries.is_empty());
    }

    {
        let package = &project.package_info[1];
        assert_eq!(package.name, "some-lib");
        assert!(package.binaries.is_empty());
    }

    {
        let package = &project.package_info[2];
        assert_eq!(package.name, "some-other-lib");
        assert!(package.binaries.is_empty());
    }

    {
        let package = &project.package_info[3];
        assert_eq!(package.name, "some-staticlib");
        assert!(package.binaries.is_empty());
    }

    {
        let package = &project.package_info[4];
        assert_eq!(package.name, "test-bin");
        assert_eq!(&package.binaries[..], &["test-bin"]);
        assert!(!package.publish);
    }

    {
        let package = &project.package_info[5];
        assert_eq!(package.name, "nonvirtual");
        assert_eq!(&package.binaries[..], &["cargo-nonvirtual", "nonvirtual"]);
        assert!(package.publish);
    }
}

#[cfg(feature = "npm-projects")]
#[test]
fn test_npm_init_legacy() {
    let project = crate::get_project("tests/projects/npm-init-legacy".into()).unwrap();
    assert_eq!(project.kind, WorkspaceKind::Javascript);
    assert_eq!(project.package_info.len(), 1);

    let package = &project.package_info[0];
    assert_eq!(package.name, "npm-init-legacy");

    // NOTE: we provide a path for this binary that doesn't exist, so if we
    // get more rigorous this test will fail. That's fine, the test can be
    // updated. Oranda has similar tests.
    assert_eq!(package.binaries.len(), 1);
    let binary = &package.binaries[0];
    assert_eq!(binary, "npm-init-legacy");
}

#[cfg(feature = "npm-projects")]
#[test]
// NOTE: this test is currently busted pending upstream orogene fixes
#[ignore]
fn test_npm_create_react_app() {
    let project = crate::get_project("tests/projects/npm-create-react-app/src/".into()).unwrap();
    assert_eq!(project.kind, WorkspaceKind::Javascript);
    assert_eq!(project.package_info.len(), 1);

    let package = &project.package_info[0];
    assert_eq!(package.name, "npm-create-react-app");

    // NOTE: we provide paths that exist here, but they're not proper binaries, so if we
    // get more rigorous this test will fail. That's fine, the test can be
    // updated. Oranda has similar tests.
    assert_eq!(package.binaries.len(), 2);

    let binary = &package.binaries[0];
    assert_eq!(binary, "index.js");

    let binary = &package.binaries[1];
    assert_eq!(binary, "App.js");
}
