---
description: Deep-scan code for bugs, vulnerabilities, and design issues; file issues unless told not to
argument-hint: "[path-or-module] [--no-issues]"
---
1. Read the project context and architecture docs
2. Map the surface area and hotspot files
3. Audit for security bugs, resource leaks, race conditions, and missing error handling
4. Classify findings as Critical, High, Medium, Low, or Informational
5. Create GitHub issues for Critical/High/Medium findings unless `--no-issues` is present
6. Produce a short audit report with verdict and test coverage gaps

Be specific. Do not invent problems to fill a quota.
