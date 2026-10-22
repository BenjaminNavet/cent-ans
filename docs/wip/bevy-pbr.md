# Lot pbr : fonctionnalités PBR et post-effets Bevy 0.20 optionnels

Branche : `worktree-agent-af54920576edee64b` (base remise à jour sur main par fast-forward : le worktree
démarrait avant la création de `client/`).

## État
- Squelette : `data/fx/postfx.json` + `data/schemas/postfx.schema.json`, clé `postfx` (liste) des
  préréglages de `render_quality.json`, features Cargo `pbr_*` et `experimental_pbr_pcss`.
- À faire : `client/src/settings/postfx.rs` (lecture, résolution, application aux caméras, section de dump
  `postfx`), 7 réglages `fx_*` dans `Settings` et le menu Affichage, tests, mesure au banc bataille.

## Conception
- Chaque effet : réglage joueur `auto` (suit le préréglage) / `off` / `on`. Préréglages low..ultra : liste
  `postfx` vide tant que rien n'est mesuré.
- Paramètres dans `data/fx/postfx.json`.

## Prochaine étape
Écrire postfx.rs, compiler, clippy, tests.
