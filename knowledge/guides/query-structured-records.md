---
type: Guide
audience: user
uses:
- /commands/init
- /commands/add
- /commands/ontology-show
- /commands/lint
- /commands/links
- /commands/backlinks
- /commands/search
- /commands/show
explains:
- /concepts/ontology-and-relationships
title: Query structured records and their relationships
description: Model a procedure's obligations, find its policy and team, and filter, expand, or aggregate matching documents.
sources:
- resource: crates/okf-core/src/query/selector.rs
  kind: git-path
- resource: crates/okf-core/src/graph/relationships.rs
  kind: git-path
---
# Query structured records and their relationships

Which team must review an account, and when? In the example below, a procedure names the
policy it implements and the team that owes the review. Nested records keep each action,
trigger, and deadline beside the reference they qualify.

## Create a small policy bundle

Start in an empty working directory. Create a bundle called `product` and run each command
from its parent directory.

```sh
okf init product
```

Add the policy the procedure implements.

```sh
okf add policies/access-control product --type Policy --title "Access control" --body "Review account access."
```

Add the team that owes the review.

```sh
okf add teams/platform product --type Team --title "Platform team" --body "Platform operations."
```

Save this ontology in the bundle root. The reference selectors identify target
concepts. The relationship declarations pair each target with the kind and attributes authored
in the **same list item**. The `evidence-file` property is plain text.

**File: `product/ontology.yaml`**

```yaml
# ontology.yaml
okf_ontology: "0.1"
field_types:
  RelationKind:
    base: enum
    values: [implements]
  Relation:
    base: object
    fields:
      kind: {type: RelationKind, required: true}
      target: {type: string, required: true}
      qualifier: {type: string}
  ClosedRelation:
    extends: Relation
    additional_properties: false
  Modality:
    base: enum
    values: [obliges, forbids, permits]
  RelativeDeadline:
    base: object
    additional_properties: false
    fields:
      within: {type: int, required: true, min: 1}
      unit: {type: enum, values: [hours], required: true}
      from: {type: enum, values: [account-created], required: true}
  Norm:
    base: object
    additional_properties: false
    fields:
      modality: {type: Modality, required: true}
      bearer: {type: string, required: true}
      action: {type: string, required: true}
      trigger: {type: string, required: true}
      discharge: {type: string, required: true}
      deadline: {type: RelativeDeadline}
concepts:
  Policy: {}
  Team: {}
  Procedure:
    fields:
      relations: {type: list, item: ClosedRelation, min: 1}
      norms: {type: list, item: Norm}
      tags: {type: list, item: string, unique_items: true}
      evidence-file: {type: string}
    references:
      policy-targets:
        selector: "relations[].target"
        target: Policy
        cardinality: 0..n
      obligation-bearers:
        selector: "norms[].bearer"
        target: Team
        cardinality: 0..n
    relationships:
      policy-relations:
        reference: policy-targets
        kind_selector: "relations[].kind"
        kinds:
          implements: {inverse: implemented-by}
        attributes:
          qualifier: "relations[].qualifier"
      obligations:
        reference: obligation-bearers
        kind_selector: "norms[].modality"
        kinds:
          obliges: {inverse: obligated-by}
          forbids: {inverse: prohibited-by}
          permits: {inverse: permitted-by}
        attributes:
          action: "norms[].action"
          trigger: "norms[].trigger"
          discharge: "norms[].discharge"
          deadline: "norms[].deadline"
```

Create the `procedures` directory and save the onboarding document. It names both targets
and gives each an authored role.

**File: `product/procedures/onboarding.md`**

```markdown
---
type: Procedure
title: Account onboarding
stable-id: PROC-017
description: Account creation and access review
tags: [security, production]
relations:
  - kind: implements
    target: /policies/access-control.md
    qualifier: production
norms:
  - modality: obliges
    bearer: /teams/platform.md
    action: review-account-access
    trigger: account-created
    discharge: access-reviewed
    deadline:
      within: 24
      unit: hours
      from: account-created
evidence-file: evidence/onboarding.csv
---
Create the account and arrange its access review.
```

Check the effective nested types declared for `Procedure`.

```sh
okf ontology show Procedure product
```

The output includes the nested deadline fields and both relationship rules. Lint the
actual values in the procedure.

```sh
okf lint product --json
```

For a schema error, correct the ontology. For a value finding, inspect the concrete field
path in the concept. For example, setting `norms[0].deadline.within` to zero violates the
declared minimum of one. The command may also report ordinary navigation findings for a
fresh bundle. Changing a shared field type affects every type that uses it.

## Read the policy and the obligation

Start with the procedure. The outgoing occurrences name the access-control policy and the
Platform team.

```sh
okf links procedures/onboarding product --details
```

Now start with the policy. Find procedures that implement it.

```sh
okf backlinks policies/access-control product --kind implements --related-type Procedure --details
```

The incoming occurrence identifies the onboarding procedure and its `production` qualifier.
To find the team’s 24-hour obligations, start from the team and filter the incoming edge.

```sh
okf backlinks teams/platform product --kind obliges --edge-filter 'deadline.within=24' --details
```

The result includes the required action, `account-created`
trigger, and deadline. Its inverse label, `obligated-by`, reads from the team's perspective.
The assertion belongs to the procedure. Operational evidence would be needed to establish
whether anyone actually completed the review.

## Filter primary records, then choose what to display

Find procedures that have a security or production tag and a deadline triggered by account
creation. Return their IDs and declared deadlines.

```sh
okf search product --type Procedure --facet-filter 'tags[] in ["security","production"]' --facet-filter 'norms[].deadline.from="account-created"' --project '$id' --project 'norms[].deadline' --json
```

The projection contains `/procedures/onboarding` and a deadline of 24 hours from
`account-created`. `[]` walks each item of a list. Separate filters combine with AND at the
document level. They can match different items in the same list. Use edge predicates when
attributes must come from the same obligation.

To read the named team's title alongside the procedure, expand the obligations rule.

```sh
okf search product --type Procedure --expand obligations --target-field title --json
```

The related concept is `/teams/platform`, titled “Platform team”. Expansion follows one
outgoing rule and retains primary hits when no target qualifies. To return the endpoint of
a longer chain, use a [saved query](save-and-run-queries.md).

To count procedures by tag, request a facet.

```sh
okf search product --type Procedure --facet 'tags[]' --limit 1 --json
```

Expect one document in the `security` bucket and one in `production`. The same procedure
has both tags. Counts cover the matching population before pagination. Check the query,
facet, and expansion summaries separately. For scan budgets, partial totals, and continuation,
see [Performance and bounded reads](bounded-reads.md). For CLI authoring and guarded
changes, see [Model and edit structured knowledge](model-your-knowledge.md).
