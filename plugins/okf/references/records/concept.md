# concept

```json
{
  "discriminator": "no kind field is added; unprojected concept metadata",
  "frontmatter_conflicts": "optional object preserving authored values whose names collide with computed fields, including authored id",
  "id": "computed suffix-free bundle-relative concept path",
  "identity_projection": "--project '$id' selects computed identity; --project id selects authored frontmatter id",
  "trust_tier": "unverified|machine-confirmed|human-reviewed",
  "verification_current": "boolean|null; latest valid verification compared with generated.at (legacy timestamp fallback); null when either timestamp is unavailable"
}
```
