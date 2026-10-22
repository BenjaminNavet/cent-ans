# tx-m6-parjure

Branche `tx/m6-parjure`, worktree `../gp-tx-m6-parjure`, ADR 0721.

## État
- Fait : table `events[]` + `truce_window` (data, schéma, data-model), `perjury.rs` (record_event, preview, fenêtre de trêve), branché dans `war.rs` (trêve récente/ancienne, pacte, lien juré), `ransom.rs` (exécution d'un captif promis sur parole), aperçu de guerre `game-api/diplomacy.rs`, garde IA (`ai.rs`). Tests `tx_m6_*` verts.
- Reste : fmt, clippy, `cargo test --workspace`, wr_ai_diplo, ADR conséquences.
- Non fait : UI client du coût de parjure à l'exécution (champs `perjury_*` dans `ExecutionPreview`, pas dans `ExecutionView`), `treaty_explain.rs` (traités, pas guerres).
