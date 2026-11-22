# Fictional worked examples

All roles and inputs below are invented. These examples illustrate report structure.

## Defect

A writer needs literal whitespace preserved. A synthetic fixture stores the string
`a  b`. Reading it back yields `a b`. The workaround is to encode the value manually.
Acceptance is an exact round trip of the invented string.

## Feature request

A reader builds an outline from two queries. A writer publishes between them. Both queries
succeed, but the reader has no indication that the inputs changed. Today the reader repeats
both queries. A successful workflow would detect the intervening publication before the
outline is accepted. A revision token is one possible interface.

## Guidance gap

An editor previews a move across two fictional collections. The preview says that no
references were found. The editor has to consult the manual to learn which collections were
examined. Acceptance is a preview that identifies the scope and explains how to widen it.

## Workflow lesson

A curator needs a consistent inventory. Reading one inventory and selecting entries locally
avoids a second query while a collection is changing. Document this practice for inventories
that fit the caller's memory budget. An example should show when a fresh inventory is required.
