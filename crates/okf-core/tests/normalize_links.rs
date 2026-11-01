use okf_core::mutate::{normalize_links::normalize_relative_links, transaction};
use std::{fs, path::Path};

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn preview_then_apply_preserves_yaml_and_markdown_presentation() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "a/b.md", "---\ntype: Note\n---\n# Target\n");
    write(root.path(), "a/picture.png", "opaque");
    let before = "---\r\ntype: Note\r\n# Keep YAML comments and field order.\r\nsources: [{resource: b.md}]\r\nverified: [{at: '2026-01-01T00:00:00Z', by: 'human:reviewer'}]\r\n---\r\nSee [**label**](b.md?view=1#heading \"title\") and [ref][B].\r\n\r\n[B]: <b.md#anchor> 'reference title'\r\n\r\n![image](picture.png)\r\n\r\n`[code](b.md)`\r\n\r\n```md\r\n[fenced](b.md)\r\n```\r\n";
    write(root.path(), "a/a.md", before);
    let preview = normalize_relative_links(root.path(), false).unwrap();
    assert!(!preview.applied);
    assert_eq!(preview.changes.len(), 1);
    assert_eq!(preview.links.len(), 3);
    assert_eq!(
        fs::read_to_string(root.path().join("a/a.md")).unwrap(),
        before
    );
    let applied = normalize_relative_links(root.path(), true).unwrap();
    assert!(applied.applied);
    let after = fs::read_to_string(root.path().join("a/a.md")).unwrap();
    let expected = before
        .replace("](b.md?view=1#heading", "](/a/b.md?view=1#heading")
        .replace("[B]: <b.md#anchor>", "[B]: </a/b.md#anchor>")
        .replace("](picture.png)", "](/a/picture.png)");
    assert_eq!(after, expected);
    assert!(normalize_relative_links(root.path(), false)
        .unwrap()
        .changes
        .is_empty());
}

#[test]
fn explicit_normalization_chooses_current_target_even_with_legacy_target_present() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "b.md", "---\ntype: Note\n---\nlegacy\n");
    write(root.path(), "a/b.md", "---\ntype: Note\n---\ncurrent\n");
    write(
        root.path(),
        "a/a.md",
        "---\ntype: Note\n---\n[b](b.md) [missing](missing.md) [explicit](./b.md)\n",
    );
    normalize_relative_links(root.path(), true).unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join("a/a.md")).unwrap(),
        "---\ntype: Note\n---\n[b](/a/b.md) [missing](missing.md) [explicit](./b.md)\n"
    );
}

#[test]
fn nested_labels_titles_and_suffix_free_concepts_keep_their_meaning() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "a/b.md", "---\ntype: Note\n---\n");
    write(root.path(), "a/b/child.md", "---\ntype: Note\n---\n");
    let before = "---\ntype: Note\n---\n[`text](b.md)` and label](b.md \"title ](b.md)\")\n[paren](b.md (title)) [id](b#anchor)\n";
    write(root.path(), "a/a.md", before);
    normalize_relative_links(root.path(), true).unwrap();
    let after = fs::read_to_string(root.path().join("a/a.md")).unwrap();
    assert!(
        after.contains("[`text](b.md)` and label](/a/b.md \"title ](b.md)\")"),
        "{after}"
    );
    assert!(
        after.contains("[paren](/a/b.md (title)) [id](/a/b.md#anchor)"),
        "{after}"
    );
}

#[test]
fn pending_publication_blocks_preview_and_apply() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "a/a.md", "---\ntype: Note\n---\n[b](b.md)\n");
    write(root.path(), "a/b.md", "---\ntype: Note\n---\n");
    fs::create_dir(root.path().join(transaction::JOURNAL)).unwrap();
    assert!(normalize_relative_links(root.path(), false).is_err());
    assert!(normalize_relative_links(root.path(), true).is_err());
}

#[test]
fn linked_image_with_same_destination_is_normalized_in_one_pass() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "a/b.md", "---\ntype: Note\n---\n");
    let before = "---\ntype: Note\n---\n[![x](b.md \"image\")](b.md \"outer\")\n";
    write(root.path(), "a/a.md", before);
    let result = normalize_relative_links(root.path(), true).unwrap();
    assert_eq!(result.links.len(), 2);
    assert_eq!(
        fs::read_to_string(root.path().join("a/a.md")).unwrap(),
        "---\ntype: Note\n---\n[![x](/a/b.md \"image\")](/a/b.md \"outer\")\n"
    );
    assert!(normalize_relative_links(root.path(), false)
        .unwrap()
        .changes
        .is_empty());
}

#[test]
fn nonconformant_changed_document_aborts_the_whole_batch() {
    let root = tempfile::tempdir().unwrap();
    let valid = "---\ntype: Note\n---\n[b](b.md)\n";
    write(root.path(), "a/a.md", valid);
    write(root.path(), "a/b.md", "---\ntype: Note\n---\n");
    write(
        root.path(),
        "a/z.md",
        "---\ntitle: missing type\n---\n[b](b.md)\n",
    );
    assert!(normalize_relative_links(root.path(), true).is_err());
    assert_eq!(
        fs::read_to_string(root.path().join("a/a.md")).unwrap(),
        valid
    );
}

#[cfg(unix)]
#[test]
fn links_escaping_through_symlinks_are_never_normalized() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    write(external.path(), "b.md", "outside");
    write(
        root.path(),
        "a/a.md",
        "---\ntype: Note\n---\n[b](escape/b.md)\n",
    );
    std::os::unix::fs::symlink(external.path(), root.path().join("a/escape")).unwrap();
    assert!(normalize_relative_links(root.path(), true)
        .unwrap()
        .changes
        .is_empty());
}
