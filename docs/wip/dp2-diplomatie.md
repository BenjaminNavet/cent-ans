# DP2 — Suites de la diplomatie (droit de passage, carte diplomatique, lisibilité des refus)

Branche : `worktree-agent-aeb4f4012dbd33ec4`. ADR : `docs/decisions/0031-droit-de-passage-et-carte-diplomatique.md`.

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
- [x] `passage.rs`, `stance.rs`, `treaty_explain.rs`, ledger `trespassers`, branchements (turn, casus belli).
- [x] Tests `sim-campaign/tests/dp2_passage.rs` (10), `dp2_explain.rs` (6 + 1 d'affichage),
  `ai/tests/dp2_passage_ai.rs` (1).
- [x] IA : `ai/src/grid.rs` retire de son graphe les places qu'elle ne veut pas violer.
- [x] Pont `godot-bridge/src/campaign_sim_dp2.rs` (`find_path_trespass`, `get_province_stances`,
  `get_faction_stance`, `get_trespass`, `explain_treaty`).
- [x] UI : `diplomatic_stances.gd` (palette), `diplomacy_controller.gd` (mode N), minicarte
  (bouton « Diplomatie », `campaign_minimap.gd`, `minimap_controller.gd`), chemin rouge
  (`army_movement_path.gd`, `army_movement_controller.gd`), panneau (raisons pondérées,
  contre-offre, fiche « droit de passage »), smoke.
- [x] ADR 0031 (dont le point 4 : accord commercial avec un rival, voulu).
- [ ] build.sh (compilation de godot-ffi très lente : machine chargée), pytest, import, smoke,
  captures, sonde d'équilibre.

## Prochaine étape
build.sh puis smoke ; captures `--stage=diplomacy_map` et `diplomacy_treaty` ; sonde
`balance_probe campaign 200 1-8` avec `passage.enabled` faux puis vrai.
