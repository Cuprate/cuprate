---
name: 🔍 Tracking Issue
about: Create an issue that tracks an effort or PR(s)
title: 'Tracking Issue for ...'
labels: ["C-tracking-issue"]
assignees: ''

---

<!--
Note: Please search to see if an issue already exists for this tracking issue.

About tracking issues:

Tracking issues are used to record the overall progress of implementation.
They are also used as hubs connecting to other relevant issues, e.g., bugs or open design questions.
A tracking issue is however not meant for large scale discussion, questions, or bug reports about a feature.
Instead, open a dedicated issue for the specific matter.
-->

## What
Describe the effort being tracked.

## Why
Describe why this effort is needed.

## Where
Describe where changes will be made.

## How
If possible, describe how the effort could be implemented.

Include a summarized version of the public API, e.g.
just the function signatures without their doc comments or implementation.

```rust
// core::magic

pub struct Magic;

impl Magic {
    pub fn magic(self);
}
```

## Steps
Describe the steps required to bring this effort to completion.

For larger features, more steps might be involved.
If the feature is changed later, please add those PRs here as well.

- [ ] Initial implementation: #...
- [ ] Other code: #...
- [ ] Finalization PR: #...
