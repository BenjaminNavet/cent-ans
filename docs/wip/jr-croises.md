# JR — Croisés (orchestration)

Spec `docs/superpowers/specs/2026-10-02-jr-croises-jerusalem-design.md`, ADR 0165.
Worktree d'intégration `../gp-jr` (`feat/jr`). Lancé le 2026-10-02, autonomie totale
(enchaîner les lots, relire, fusionner dans main sans jalon de validation). Budget 0 $.

## Lots
- JR1 règle (cent-ans-dev, `../gp-jr`, `feat/jr`) : tout le Rust + `data/rules/crusade.json` et son
  schéma. Squelette commité d'abord ; fusionne `feat/jr-data` pour les tests d'intégration.
- JR2 données (cent-ans-mech, `../gp-jr-data`, `feat/jr-data`) : tout `data/` hors `crusade.json` —
  faction, chef, Limassol, flotte, revendications, relations réciproques, objectifs, arêtes
  maritimes, codex, armes et bannières, carte de sélection. Pas de build cargo.
- JR3 pont + section Godot (cent-ans-dev) : section « Ferveur », `EventKind::Crusade` côté Godot,
  sélecteur de faction sur carte, test headless.
- JR4 IA + équilibre (cent-ans-dev) : sonde 5 graines × 50 tours, débarquement de l'IA.
- JR5 relecture, capture de contrôle, fusion dans main.

Ordre : (JR1 ∥ JR2) → (JR3 ∥ JR4) → JR5.

## Contrat d'identifiants
`fac_crusaders`, `chr_pierre_de_la_palud`, base `set_limassol`, capitale `prov_cyprus`,
revendications `prov_jerusalem`, `prov_gaza`, `prov_safad`.

## État
- JR1 fait (règle, pont, IA de prêche ; cargo 1362 ok). JR2 + JR2b faits (données, arêtes forcées).
- JR3 fait (section Ferveur, repère HUD, sélecteur « Sans terre », évènements ; smoke ok).
  Capture jugée : lisible ; défauts renvoyés : déficit de départ −1 076/saison, « tour(s) »,
  armes illisibles (JR3b en cours).
- JR4 en cours (IA outre-mer, équilibrage, seuils dans la vue).

## Prochaine étape
- JR5 : relecture du diff, tests complets, nouvelle capture, fusion dans main (worktree dédié, ff-only).

## Points ouverts
- Usages de la capitale pour une faction sans cité (liste dans le rapport d'exploration, spec § 5.1).
- `fe_ui_test` « map picker framed on the playable lands » échoue selon JR3 depuis OM (pas JR) : à vérifier sur main.
