# Test plan gaps

- `utils.js`: The time-remaining table covers English through days only. Stretchly delegates duration wording, plural forms and longer units to `humanize-duration`; that library is not in the approved Rust crate list, and the complete formatter has not been ported. The bundled locale JSON validation does not verify localized duration wording.
