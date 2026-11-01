use okf_core::{
    model::{
        concept::{Concept, ConceptId},
        frontmatter::Frontmatter,
    },
    ontology::{
        field_types::{check_concept, resolve_field, resolve_type_name},
        load::parse_ontology,
    },
    query::metadata::Condition,
};

fn concept(yaml: &str) -> Concept {
    Concept {
        id: ConceptId::from_relative("note"),
        frontmatter: Frontmatter::from_map(serde_yaml::from_str(yaml).unwrap()),
        body: String::new(),
    }
}

#[test]
fn complement_includes_absent_only_when_requested() {
    let expression = Condition::parse("status != \"implemented\" OR status absent").unwrap();
    assert_eq!(
        expression.source,
        "status != \"implemented\" OR status absent"
    );
    for (metadata, expected) in [
        ("status: proposed", true),
        ("status: implemented", false),
        ("type: Note", true),
        ("status: null", false),
    ] {
        assert_eq!(
            expression.matches(&concept(metadata)).unwrap(),
            expected,
            "{metadata}"
        );
    }
    assert!(Condition::parse("status absent OR status is null")
        .unwrap()
        .matches(&concept("status: null"))
        .unwrap());
    assert!(!Condition::parse("status not in [\"implemented\"]")
        .unwrap()
        .matches(&concept("type: Note"))
        .unwrap());
}

#[test]
fn presence_is_distinct_from_null_and_empty_containers() {
    let note =
        concept("nothing: null\nempty: []\nobject: {}\nnested: [{value: null}, {value: yes}]\n");
    for filter in [
        "nothing exists",
        "nothing is null",
        "missing absent",
        "empty exists",
        "empty[] exists",
        "empty[] is not null",
        "object is not null",
        "nested[].value is null",
        "nested[].value is not null",
        "nested[].unknown absent",
    ] {
        assert!(
            Condition::parse(filter).unwrap().matches(&note).unwrap(),
            "{filter}"
        );
    }
    for filter in [
        "nothing absent",
        "nothing is not null",
        "missing is null",
        "missing is not null",
        "empty is null",
    ] {
        assert!(
            !Condition::parse(filter).unwrap().matches(&note).unwrap(),
            "{filter}"
        );
    }
}

#[test]
fn disjunction_respects_quoted_json_and_validates_all_branches() {
    let note = concept("a OR b: 'x OR y'\nstatus: 5\n");
    assert!(
        Condition::parse("[\"a OR b\"] in [\"x OR y\"] OR missing exists")
            .unwrap()
            .matches(&note)
            .unwrap()
    );
    assert!(
        Condition::parse("missing absent OR status != \"implemented\"")
            .unwrap()
            .matches(&note)
            .is_err()
    );
    for filter in [
        "",
        " OR ",
        "status exists OR ",
        "status exists OR OR missing absent",
        "status == 1",
        "status != null",
        "status is absent",
        "status exists OR nope",
        "status in []",
        "status = [1]",
        "status exists or missing absent",
    ] {
        assert!(Condition::parse(filter).is_err(), "{filter}");
    }
    assert!(Condition::parse(&vec!["x absent"; 65].join(" OR ")).is_err());
}

#[test]
fn field_documentation_survives_inheritance_roundtrip_and_lint() {
    let yaml = r#"
okf_ontology: '0.1'
field_types:
  Parent: {base: string, description: Parent meaning, min: 1}
  Inherited: {extends: Parent}
  Child: {extends: Parent, description: Child meaning}
  Record:
    base: object
    fields:
      name: {type: Child, description: Nested meaning}
concepts:
  Note:
    fields:
      inherited: {type: Inherited}
      child: {type: Child}
      own: {type: Child, description: Field meaning}
      record: {type: Record}
"#;
    let ontology = parse_ontology(yaml).unwrap();
    let roundtrip = serde_yaml::to_string(&ontology).unwrap();
    assert_eq!(parse_ontology(&roundtrip).unwrap(), ontology);
    let fields = &ontology.concepts["Note"].fields;
    for (field, expected) in [
        ("inherited", "Parent meaning"),
        ("child", "Child meaning"),
        ("own", "Field meaning"),
    ] {
        let resolved = resolve_field(&ontology, &fields[field]).unwrap();
        assert_eq!(resolved.ty.description.as_deref(), Some(expected));
        assert!(!resolved.ty.constraints.contains_key("description"));
        assert!(serde_json::to_value(&resolved).unwrap()["ty"]["description"] == expected);
    }
    let record = resolve_type_name(&ontology, "Record", &mut Vec::new()).unwrap();
    assert_eq!(
        record.fields["name"].ty.description.as_deref(),
        Some("Nested meaning")
    );
    let metadata =
        concept("type: Note\ninherited: foo\nchild: bar\nown: baz\nrecord: {name: example}");
    assert!(check_concept(&ontology, &metadata.frontmatter, |_| None).is_empty());
    let invalid = yaml.replace("description: Field meaning", "unknown_constraint: ignored");
    assert!(!check_concept(
        &parse_ontology(&invalid).unwrap(),
        &metadata.frontmatter,
        |_| None
    )
    .is_empty());
}
