# 0256 — Chaîne de migration des sauvegardes et erreurs de test attendues

Statut : accepté (2026-10-09, lot RX `robust`)

## Contexte
`CampaignState::load_json` refusait toute `state_version` différente de `STATE_VERSION` : chaque
changement de format perdait les parties en cours, sans chemin de reprise. Par ailleurs le smoke
imprime des `ERROR:` volontaires (fixtures sans relief ni données de simulation) indiscernables
d'une panne réelle dans un journal.

## Décision
1. `sim-campaign/src/save.rs` porte une chaîne de migrations JSON : `OLDEST_MIGRATABLE_VERSION`
   (9 aujourd'hui) et `MIGRATIONS[i]`, étape `v(OLDEST+i) -> v(OLDEST+i+1)` sur l'arbre
   `serde_json::Value`. `load_json` applique les étapes puis désérialise en version courante.
   Version < `OLDEST_MIGRATABLE_VERSION` : `OlderSave` ; version > `STATE_VERSION` :
   `VersionMismatch` (inchangés). Un test garantit `MIGRATIONS.len() == STATE_VERSION - OLDEST`
   : **tout incrément de `STATE_VERSION` doit ajouter son étape** (vide si le format ne change
   pas). Abandonner une ancienne version = relever `OLDEST_MIGRATABLE_VERSION` et retirer ses
   étapes, délibérément.
2. `ExpectedErrors` (`game/scripts/debug/expected_errors.gd`) : drapeau `Engine` « erreurs
   attendues ». Les sites concernés (`TerrainBuilder._fail_relief`, `CampaignMap`, pont
   `CampaignSim`) émettent alors un avertissement préfixé `[attendu]` au lieu d'une erreur. Le smoke
   l'active autour du seul chargement de la carte sur fixtures.

## Conséquences
Les parties v9 restent chargeables au prochain format ; un `grep '^ERROR'` d'un journal de smoke ne
signale plus que des pannes. Le drapeau est global : à limiter à `begin()`/`end()` autour d'un repli
connu.
