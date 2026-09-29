# Plan : OM — carte Oural–Méditerranée

Spec : `docs/superpowers/specs/2026-09-28-oural-mediterranee-design.md`. ADR 0115, 0116.
Intégration : worktree `../game_project-om`, branche `feat/om`. Chaque lot : branche `feat/om-<lot>`
issue de `feat/om`, commits `wip:` au plus toutes les 15 min, note `docs/wip/om-<lot>.md`.

## OM0 — Squelette (orchestrateur)
Spec, ADR, plan, énumérations de schéma (steppe, desert, arid), religions, noms des mers.

## Vague 1
- **OM1 Monde rectangulaire** (`cent-ans-dev`) : taille du monde lue dans `map.json`, plus aucune
  constante 4096 (Rust relief-lod, vegetation, pont, navgrid ; GDScript ; shaders ; caméra ; tests),
  décalage d'origine de la pyramide, refus des anciennes sauvegardes. Identique à l'existant sur la
  carte actuelle.
- **OM2 Chaîne géo** (`cent-ans-dev`) : nouvelle emprise, téléchargements (ETOPO, KK10, Natural
  Earth), génération de tous les artefacts, migration +1280 y des données en pixels, générateur de
  provinces (terres sans graine proche hors provinces), fichiers ≤ 50 Mo.
- **OM3 Terrains et climats** (`cent-ans-dev`) : `steppe`, `desert`, `arid`, `steppe` dans le cœur
  (mouvement, ravitaillement, attrition, bataille), revue des religions ajoutées.
- **D1-D3** (`cent-ans-mech`) : registres Nord-Baltique, Europe centrale et orientale, Rus' et Horde.
  Données seulement : pas de `cargo`, pas de géo ; `pytest` sur les schémas et invariants.

## Vague 2
- **D4-D6** : Balkans-Byzance, Anatolie-Levant-Égypte, Maghreb.
- Intégration : fusion OM1+OM2+OM3, puis données D1-D6 (résolveur JSON à trois voies pour les listes
  partagées), une seule régénération géo, `cargo test`, `pytest`, `smoke.gd`, mesure mémoire et
  temps de tour.

## Vague 3
- Blasons et bannières générés, fiches front-end, portraits (plafond 10 $), équilibre (sonde
  `century_probe`), captures de contrôle (3 max).
