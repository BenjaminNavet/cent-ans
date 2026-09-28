# FE — féodalité et petites factions (orchestration)

Spec : `docs/superpowers/specs/2026-09-28-feodalite-design.md`. Plan : `docs/superpowers/plans/2026-09-28-feodalite.md`.
ADR : 0098. Worktree orchestrateur : `../game_project-fe` sur `feat/fe`.

## État
- 2026-09-28 : spec validée (b651fda4), plan écrit (31f10532).
- F0 **fait**, dans `main` (1dcd6821) ; tests Rust, pytest, smoke verts. Détail F0 : modèle `FeudalTitle` / `TitleId` / `FeudalRules` (data-model), invariants
  `title_check.rs`, `data/rules/feudal.json` (constantes de `diplomacy.rs` retirées), migration
  `feudal_migrate.py` appliquée (45 titres, `overlord`/`holder` retirés), `CampaignState::feudal`
  (détentions), `feudal.rs` avec **déductions déjà implémentées** (`liege_of`, `province_lieges`,
  `direct_vassals`, `title_vassals`, `feudal_tree`) pour que F1-F3 partent de la même base ; stubs
  pour escalade, félonie, commise, objectifs. Sauvegarde v7 (`PreFeudalSave`). Tests `feudal_*.rs`.

## Écarts au plan
- F0 fait dans un worktree (`feat/fe`) puis ff vers `main`, pas directement dans `main` (TW2 en parallèle).
- Déductions faites en F0 (et non F1) : F1 se concentre sur le branchement de `diplomacy.rs` et la loyauté.
- `playable` par défaut à `true` : reporté à F4/F6 (change le menu de choix de faction).
- Princes d'Empire (Autriche, Bohême, Brabant, Gueldre, Hainaut, Waldstätten, Vérone) : vassaux
  *de jure* de l'Empire par la migration, mais sans `suzerain` de faction ; F1 aligne (ADR 0098).

## Prochaine étape
- Vague 1 lancée le 2026-09-28 (4 agents, worktrees isolés) : F1 `feat/fe1-deductions`, F2 `feat/fe2-escalade`,
  F3 `feat/fe3-titres` (cent-ans-dev), F4a `feat/fe4a-france` (cent-ans-mech). Notes `docs/wip/fe<N>-*.md` dans leurs branches.
- À leur retour : fusion dans un worktree dédié (`feat/fe`), conflits attendus dans `feudal.rs`, puis ff `main`.

## Points ouverts
- `STATE_VERSION` 7 : si TW2 fusionne avant un bump de version, reprendre sa valeur + 1.
