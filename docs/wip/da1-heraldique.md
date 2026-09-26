# DA1 — Armoiries des maisons et héraldique des figurines

Branche : `worktree-agent-addd87848de848c53` (worktree `.claude/worktrees/agent-addd87848de848c53`),
basée sur main + merge de `feat/da-direction-artistique` (bible DA). Bible :
`docs/design/2026-09-25-bible-da.md`. ADR : `docs/decisions/0064-armoiries-des-maisons.md`.

## État
- [x] Données `data/heraldry/houses.json` (51 maisons, 4 badges) + schéma + tests pytest
  (`tools/tests/test_heraldry_houses.py`).
- [x] `heraldry.py` grammaire v2 (maisons) ; écus de faction identiques à l'octet (vérifié) ;
  `build_houses()` → `game/assets/heraldry/houses/*.png` ; CLI `cent-ans assets heraldry`.
  Planche : `docs/img/da1/planche_maisons.jpg`.
- [x] `HouseArms` (`game/scripts/ui/house_arms.gd`), `PortraitLoader.house_heraldry_texture` ;
  fiche, cour, arbre, sceau du général, HUD et avant-bataille (maison posée dans
  `setup[side].general.house` par `battle_scene.gd`).
- [x] Shader skinné : atlas `Texture2DArray`, seigneur + 3 bannerets (30 %), projection
  poitrine/dos (boîte `BattleSkinned.chest_box`), croix du commun ; rigide : armes du
  seigneur pour les nobles ; `--no-da1` pour A/B.
- [x] Captures `docs/img/da1/` (avant = `--no-da1` dans `res://tests/da1_arms_shot.gd`).
- [x] Mesure perf (`res://tests/da1_perf.gd`, A/B dans un seul processus contre le shader de
  main) : +0,22 % / -0,17 % / +0,03 % → négligeable. Smoke vert, pytest 567 passés.
- Lot terminé, en attente de fusion par l'orchestrateur DA.

## Notes
- Le buste des figurines regarde -Z en pose de repos (demi-tour cuit dans les os) : d'où le
  retournement de `uv.x` quand `v_rest_n.z > 0`.
- Bench bataille (`--benchmark`) trop bruité sur la machine partagée (35-90 i/s d'une passe
  à l'autre) : mesure A/B faite dans un seul processus.
- Suites possibles : étendards EP5 aux armes de la maison du général ; bible § 10 ligne 2 à
  marquer faite par l'orchestrateur.
