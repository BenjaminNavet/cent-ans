# Migration vers Bevy (branche `bevy-migration`, worktree `../gp-bevy`)

Mandat (14/10 au soir) : nuit en autonomie complète, largeur maximale avec de nombreux agents Sonnet
contrôlés par Opus, **ni fusion dans `main` ni push**. Verdict : ADR 0347 ; architecture : ADR 0357.

## État
- [ ] V0 squelette : `client/` (plugins vides enregistrés, états, surface agent, chargement
      sim), `core/crates/game-api` (vide + test).
- [ ] V1 (parallèle) : ui/kit, campaign/terrain+camera, campaign/provinces, campaign/water,
      campaign/markers, battle/scene, audio, game-api (vues HUD/province/faction).
- [ ] V2 : menus (accueil, faction, réglages, sauvegardes), HUD campagne + fin de tour,
      fiche de province, armées (sélection, ordres, chemin), diplomatie, technologies,
      bataille (ordres, HUD, avant-bataille, résultat), codex, végétation/forêts.
- [ ] V3 : le reste (vie de la carte, sièges, naval auto-résolu, conseil, féodalité, chronique…).

## Règles des lots
- Un lot = un dossier de `client/src/` (ou un module de `game-api`) ; ne toucher ni `main.rs` ni
  les dossiers des autres.
- Branche `bm/<lot>` depuis `bevy-migration`, worktree `../gp-bm-<lot>`, `client/target` cloné
  (`cp -c -R`, APFS) depuis `../gp-bevy/client/target`.
- Référence : le GDScript équivalent dans `game/scripts/…` (même dépôt).
- Vérification : `cargo clippy --features dev -- -D warnings`, `cargo test`, passe
  `--dump-state` ; pas de capture pour les agents Sonnet.

## Journal
- 15/10 ~1 h 45 : banc n° 4 valide, verdict Bevy (ADR 0347). Branche créée depuis `proto-bevy`
  (qui contient `main` du 15/10, `ef2bbfaae`).
