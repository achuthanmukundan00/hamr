---
name: receiving-code-review
description: Use when receiving code review feedback, before implementing suggestions, especially if feedback seems unclear or technically questionable - requires technical rigor and verification, not performative agreement or blind implementation
---

# Receive Review

Technical correctness over social comfort. Verify before implementing. No performative agreement.

## Flow

1. Read all feedback.
2. Understand each item; restate or ask.
3. Verify against codebase.
4. Evaluate: correct here, compatible, not YAGNI violation?
5. Respond with technical ack or reasoned pushback.
6. Implement one item at a time; test each.

If any item unclear, stop and clarify before partial implementation; items may interact.

## Sources

Human partner: trusted, but ask if scope unclear. Skip praise; act.

External reviewer: skeptical but careful. Check:
- correct for this codebase?
- breaks existing behavior/platform?
- reason current code exists?
- full context known?
- conflicts with human decisions?

If cannot verify, say what evidence is missing and ask whether to investigate/proceed.

## YAGNI

If reviewer asks for "proper" unused feature, grep usage. If unused, ask to remove/skip. If used, implement properly.

## Push Back When

Suggestion is wrong, breaks compat, violates YAGNI, ignores legacy constraint, or conflicts with architecture. Use code/tests, not defensiveness. Involve human for architecture.

## Forbidden

- "You're absolutely right"
- "Great point"
- "Thanks..."
- "Let me implement" before verification
- Top-level GitHub PR reply to inline comment; reply in thread (`.../pulls/{pr}/comments/{id}/replies`).

Correct feedback response: `Fixed: <specific change>.` If your pushback was wrong: `You were right; I checked <evidence>. Fixing.`
