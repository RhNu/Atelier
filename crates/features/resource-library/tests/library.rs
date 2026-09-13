use atelier_resource_library::{
    LibraryFolder, LibraryFolderId, LibraryNamespace, LibraryResource, LibraryResourceId,
    LibraryTree, ResourceIdentifier, ResourceLibraryErrorKind, ResourceName, ResourcePath,
};

#[test]
fn identifiers_and_paths_are_unicode_nfc_and_root_relative() {
    let identifier = ResourceIdentifier::parse("背景-e\u{301}").unwrap();
    assert_eq!(identifier.as_str(), "背景-é");
    assert_eq!(
        ResourcePath::parse("人物/女性/hero-1").unwrap().as_str(),
        "人物/女性/hero-1"
    );
    for invalid in ["", "1hero", "with space", "a/b", ".", "a."] {
        assert!(ResourceIdentifier::parse(invalid).is_err(), "{invalid}");
    }
    for invalid in ["", "/hero", "hero/", "hero//main"] {
        assert!(ResourcePath::parse(invalid).is_err(), "{invalid}");
    }
    assert_eq!(
        ResourceIdentifier::from_legacy("人物 / 女性", "item").as_str(),
        "人物_女性"
    );
    assert_eq!(
        ResourceIdentifier::from_legacy("42 cats", "item").as_str(),
        "_42_cats"
    );
}

#[test]
fn tree_enforces_namespace_parent_and_sibling_rules() {
    let mut tree = LibraryTree::default();
    let people = folder(LibraryNamespace::PromptChunk, None, "people");
    tree.insert_folder(people.clone()).unwrap();
    let hero = resource(
        LibraryNamespace::PromptChunk,
        Some(people.id.clone()),
        "hero",
    );
    tree.insert_resource(hero.clone()).unwrap();
    assert_eq!(
        tree.resource_path(&hero.id).unwrap().as_str(),
        "people/hero"
    );

    let duplicate_folder = folder(LibraryNamespace::PromptChunk, Some(people.id), "hero");
    assert_eq!(
        tree.insert_folder(duplicate_folder).unwrap_err().kind(),
        ResourceLibraryErrorKind::Conflict
    );

    tree.insert_resource(resource(LibraryNamespace::Vibe, None, "hero"))
        .unwrap();
}

#[test]
fn moving_a_folder_repaths_descendants_and_rejects_cycles() {
    let mut tree = LibraryTree::default();
    let people = folder(LibraryNamespace::PromptChunk, None, "people");
    let women = folder(
        LibraryNamespace::PromptChunk,
        Some(people.id.clone()),
        "women",
    );
    let archive = folder(LibraryNamespace::PromptChunk, None, "archive");
    tree.insert_folder(people.clone()).unwrap();
    tree.insert_folder(women.clone()).unwrap();
    tree.insert_folder(archive.clone()).unwrap();
    let hero = resource(
        LibraryNamespace::PromptChunk,
        Some(women.id.clone()),
        "hero",
    );
    tree.insert_resource(hero.clone()).unwrap();

    tree.move_folder(&people.id, Some(archive.id.clone()), 20)
        .unwrap();
    assert_eq!(
        tree.resource_path(&hero.id).unwrap().as_str(),
        "archive/people/women/hero"
    );
    assert_eq!(
        tree.move_folder(&archive.id, Some(women.id), 30)
            .unwrap_err()
            .kind(),
        ResourceLibraryErrorKind::Cycle
    );
}

fn folder(
    namespace: LibraryNamespace,
    parent_id: Option<LibraryFolderId>,
    identifier: &str,
) -> LibraryFolder {
    LibraryFolder::new(
        LibraryFolderId::allocate(),
        namespace,
        parent_id,
        ResourceIdentifier::parse(identifier).unwrap(),
        identifier,
        10,
    )
    .unwrap()
}

fn resource(
    namespace: LibraryNamespace,
    folder_id: Option<LibraryFolderId>,
    identifier: &str,
) -> LibraryResource {
    LibraryResource {
        id: LibraryResourceId::allocate(),
        namespace,
        folder_id,
        name: ResourceName::new(
            ResourceIdentifier::parse(identifier).unwrap(),
            identifier,
            ["search alias"],
        )
        .unwrap(),
        created_at_ms: 10,
        updated_at_ms: 10,
    }
}
