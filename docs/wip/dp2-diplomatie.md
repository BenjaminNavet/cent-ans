# DP2 — Suites de la diplomatie (droit de passage, carte diplomatique, lisibilité des refus)

Branche : `worktree-agent-aeb4f4012dbd33ec4`. ADR : `docs/decisions/0031-droit-de-passage-et-carte-diplomatique.md` (à écrire).

## Plan
1. Droit de passage (`core/crates/sim-campaign/src/passage.rs`) : intrusion détectée en fin de saison
   (`turn.rs`, avant `resolve_diplomacy`), malus croissant `Violation de nos terres`, casus belli
   (`diplomacy.rs::casus_belli`), grâce pendant une trêve ; IA (`ai/src/grid.rs` : nœuds interdits) ;
   aperçu de chemin (`trespass_along`). Accès militaire = clause DP1 existante (`Article::MilitaryAccess`).
   `movement.rs` non touché.
2. Carte diplomatique (`stance.rs`) : allié / accord / neutre / tension / guerre / vassal ; mode N et
   bouton « Diplomatie » de la minicarte.
3. Explication détaillée des refus + contre-offre sur un seul point bloquant (`treaty_explain.rs`).
4. Accord commercial contre un rival : vérifier et documenter.

## État
- [x] Données `data/ai/diplomacy.json` § `passage` + schéma + `PassageRules`.
- [x] `passage.rs` (règles, détection, chemin, IA), `stance.rs`, ledger `trespassers`, branchements.
- [ ] Tests `sim-campaign/tests/dp2_passage.rs`.
- [ ] IA (grid.rs), pont, UI, explication, ADR, sondes.

## Prochaine étape
Tests déterministes du droit de passage, puis IA.
