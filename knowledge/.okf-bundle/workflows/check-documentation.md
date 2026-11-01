# Check the documentation bundle

From the selected bundle root, run okf bundle validate ., okf validate . --profile bundle,
and okf lint . --fail-on any. Report each result separately from source verification.
When this bundle is in the OKF source repository and repository checks are requested,
run cargo xtask docs --check, cargo xtask skills, and python3 scripts/check-docs.py from
the repository root. If those tools or files are unavailable, report that limitation.
This check workflow does not authorize generated-file updates or knowledge verification.
