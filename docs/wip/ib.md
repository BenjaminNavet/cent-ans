# IB — Infobulles en sections et chaînes à Alt (orchestration) — fichier de reprise

Demande du joueur (28/09) : comparer une infobulle de BG3 à celles du jeu et faire mieux ; ouvrir
plusieurs infobulles en chaîne en maintenant une touche. Décisions : **Alt**, détail complet dans
les bulles verrouillées ; « vas-y ».

- Spec : `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md` (validée, **source de vérité**, lots § 4)
- ADR : `docs/decisions/0109-infobulles-sections-et-chaine-alt.md`. Budget : 0 $.

## Reprendre

Lire ce fichier, `git worktree list`, `git branch --list 'feat/ib*'`. Chaque lot a son
`docs/wip/ib<N>.md`. Fusion : worktree `../gp-ib-merge` (branche `integration/ib`, depuis `main`),
puis ff-only vers `main`. Autres chantiers actifs le 28/09 : FE (vague 2), TW2 (T3-T5), RS (C, K, L)
→ 2 agents IB à la fois ; IB2 (touche ~120 `tooltip_text` dans beaucoup de fichiers) en dernier,
après les fusions FE/TW2 en cours si possible.

## Liste

- [x] IB0 squelette (orchestrateur, dans `main`) : ADR 0109, `data/ui/tooltip_style.json` +
  `tooltips.json` + schémas + `tools/tests/test_tooltip_schemas.py`, action `tooltip_explore` (Alt),
  `TooltipView` (repli sur `make_panel`), `ib_layout_test.gd` / `ib_chain_test.gd` désactivés
- [ ] IB1 rendu en sections (dev) — branche `feat/ib1-layout`
- [ ] IB3 chaîne à Alt (dev) — branche `feat/ib3-chain`
- [ ] IB4 liens `ib:`, bulles riches, placement latéral, fil d'Ariane (dev) — après IB1 + IB3
- [ ] IB5 avant → après (`before`/`after` au pont) (dev) — après IB1
- [ ] IB2 autres constructeurs, ~120 infobulles brutes, tables vers `data/` (mech) — après IB1
- [ ] IB6 intégration, planche avant/après (`docs/img/ib/`), jugement du joueur

## Tronc commun des briefs

> Tu travailles sur le jeu Cent Ans (lire `CLAUDE.md`). Chantier IB : spec
> `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`, ADR 0109, fichier `docs/wip/ib.md`.
> Ton lot est « IB<N> » du § 4 de la spec. Branche `feat/ib<N>-<objet>` depuis `main`, dans ton
> worktree. Squelette d'abord, commits `wip:` au plus toutes les 15 min avec des chemins explicites,
> `docs/wip/ib<N>.md` (état, prochaine étape) à chaque commit. Pas de `git stash`. Ne touche jamais
> `main`, ne fusionne rien. Aucun outil de capture ni d'éditeur ; tu n'ouvres pas les images.
> Après une copie de dylib : `godot --headless --path game --import`. Tests d'écran : appeler
> `Settings.use_test_file()` + `_apply_ui_scale()` avant le premier écran. Présentation pure :
> aucune règle de jeu dans `game/`. Avant de rendre la main : fusionne `main` dans ta branche, lance
> `smoke.gd`, tes tests IB, `po_ui_test.gd`, `p2c_ui_test.gd`, puis réponds en 10 lignes au plus :
> fait, reste, commit final, tests.

| Lot | Fichiers | À ajouter |
|---|---|---|
| IB1 | `rich_tooltip.gd`, `tooltip_view.gd`, `data/ui/tooltip_style.json`, `ib_layout_test.gd`, `game/tests/ib_shot.gd` | Commencer par `ib_shot.gd` et la planche **avant** `docs/img/ib/avant/` (infobulle d'unité en recrutement et en armée, bâtiment, technique, jauge de province) sur l'état de `main`. Puis spec § 2.1-2.3 pour unités, bâtiments, techniques ; `make_panel(bbcode)` reste en repli pour les autres. Version courte / complète via `detailed`. Tailles `UiType`. Activer `ib_layout_test` (§ 5). Ne pas toucher `codex_bubbles.gd` (IB3). |
| IB3 | `codex_bubbles.gd`, `ib_chain_test.gd`, `shortcut_sheet.gd` | Spec § 3.1-3.2 : Alt (`tooltip_explore`) fige l'infobulle native visible (`RichTooltip.visible_panel` / contrôle survolé) en bulle verrouillée à la même place ; Alt maintenu → filles à `chain.hover_delay_s`, verrouillées, remplacement de branche ; relâché → grâce ; T inchangé. Délais et `max_bubbles` lus dans `tooltip_style.json`. Bulle verrouillée = `detailed` (appel à `TooltipView.build(spec, true)` quand la bulle vient d'une spec, sinon BBCode actuel). Ne pas toucher `rich_tooltip.gd` (IB1). Activer `ib_chain_test` (§ 5). |

## Journal

- 28/09 : spec (f65e12c6), décisions du joueur (Alt, détail verrouillé), IB0 écrit. P2c déjà
  fusionné par RS. `main` : dylib périmée après la fusion FE (`feudal.json` : `power_ratio` inconnu)
  → `core/build.sh` relancé par IB0 avant le smoke.
