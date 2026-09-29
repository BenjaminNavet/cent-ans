# NT5 — N6 plafond d'unités + N7 engins de siège construits

Branche `feat/nt5-cap-engines` (worktree agent). État : **terminé**, à fusionner. Spec : `docs/superpowers/specs/2026-09-29-nt-nuit-tww3-design.md` (ligne NT5).
ADR : `docs/decisions/0128-plafond-et-engins-de-siege.md`.

## Conception
- N6 : `data/rules/armies.json` (`max_units` 20, schéma `army_rules.schema.json`), `GameData::army_rules`.
  Remplace `encounters.json` `max_army_units` (supprimé). Bornés : `CreateArmy`, `MergeArmies`
  (`OrderError::ArmyFull`), mercenaires (`blocked` « armée complète »), rencontres `join`.
  Le recrutement lève en garnison (pas dans l'armée) : c'est la formation d'armée qui est bornée.
  IA : `CreateArmy` découpée par paquets de 20, fusion seulement si la somme tient, mercenaires
  bornés par la place libre.
- N7 : `data/rules/siege_engines.json` (schéma `siege_engine_rules.schema.json`). `SiegeState.engine_work`
  cumule chaque tour de siège `hommes / men_per_work_point` (min `min_work_per_turn`, × vitesse de
  siège) ; engins dans l'ordre (échelles 4, bélier 16, beffroi 40). `assault_blocker` : murailles
  debout et aucun engin prêt → assaut refusé (`AssaultError::NoEngine`), l'IA attend. Beffroi prêt →
  `walls_stand` faux. Bataille : `SiegeSetup.engines` (`SiegeEngineSetup {ram, ladders, towers}`),
  `None` = ancien comportement ; sans échelles, pas d'escalade hors tour accostée.
- Bridge : `get_assault_odds` + `engines`, `blocker` ; `army_unit_cap()`.

## État
- [x] Données + schémas, data-model, sim-battle, sim-campaign, IA, bridge (compile)
- [x] Tests Rust NT5 (`sim-campaign/tests/nt5_cap_engines.rs` 8, `ai/tests/nt5_cap_engines_ai.rs` 3), pytest schémas
- [x] Tests d'assaut existants (m8, m8_battle, q5) : échelles posées avant l'ordre (`ladders_ready`) ;
  sièges de démo (`stage_siege_at`) : échelles + bélier comme avant
- [x] UI : siege_controller (engins, bouton grisé + infobulle), settlement/province panel (Former une armée > 20)
- [x] `core/build.sh`, test Godot `nt5_cap_engines_test.gd` OK, `tw2_t3_mercenaries_test` OK
- [x] Garde-fous bataille avant/après : `ep7_historical` 6/6, `ep9b_duel` 8/8, `ai` (dont `ai_beats_a_passive_ai_at_equal_forces`) 2/2, identiques
- [x] `cargo test --workspace` (no-fail-fast), clippy, pytest 1274, smoke OK
- [x] Sonde `century_probe` après (part de guerre FR–EN) : dans la cible, aucun ajustement de données

## Chiffres d'équilibre
Avant (main f36689196, `century_probe 464 1-6`, normale) : guerre FR-EN moy. 67,6 % [61-73], 6/6 dans
55-75 % ; prises 949-1474 (moy. 1188) ; sièges réussis 34-52 %.
Après (même sonde) : guerre FR-EN moy. 66,9 % [64-71], 6/6 dans 55-75 % ; prises moy. 1116 ;
sièges réussis 37-43 % ; sièges engagés 446 → 452 ; révoltes 24,5 → 18,3 par partie.
Effet de bord : batailles FR/EN par décennie 77,5 → 144,6 (armées plafonnées à 20 : plus d'osts
distincts, donc plus de rencontres). Garde-fous bataille inchangés (voir État).

## Points ouverts
- `cv3_ai_stances::the_ai_never_gives_a_stance_order_the_core_refuses` : plus d'embuscade en 60 tours
  pour les graines 1-4 après NT5 ; graine 5 (aucun ordre refusé dans tous les cas).
- Batailles FR/EN ×1,9 par décennie (N6) : à surveiller en partie pilote.
- Le bélier n'a pas d'effet propre en résolution automatique (seul le beffroi ôte les murailles).

## Prochaine étape
Fusion dans `integration/nt` par le coordinateur.
