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
- [x] IB1 **fusionné** (ff `main`, df96f3b8)
- [x] IB3 **fusionné** (6197ed33) : Alt fige/chaîne/remplace, grâce, Échap ; version détaillée en attente de `RichTooltip.spec_for` (IB1) ; cas « souris sur bulle » vérifié à la main seulement (headless)
- [x] IB4 **fusionné** (f679f72f) : liens `ib:`, bulles riches et de règle, placement latéral, fil d'Ariane, réduction ; défauts IB1 corrigés. Ouverts : alignement sur la ligne du mot-clé à juger à l'œil ; ancêtre rouverte ne re-place pas ses descendantes ; ressources/traits/compétences sans `*_spec` (spec simple)
- [x] IB5 **fusionné** (3e26c0ac) : `preview.rs` (avant → après sur copie d'état), `requirements` [{id, met}] au pont ; jauges à équilibre libellées « équilibre a → b » (`equilibrium_effects` du style). Non couverts : piété, garnison, effets de bataille, croissance, unités
- [x] IB2 **fusionné** (7ac2f0d4) : 135 littéraux → `attach_plain`/`plain_tooltip_host.gd`, tables → `tooltips.json`, bogue de repli de `RichTooltip.texts()` corrigé. Reste : `RichLabel` (`pre_battle_dialog.gd`) rend encore à plat ; découpage titre/corps à relire
- [ ] IB6 intégration faite, planche faite ; **jugement du joueur en attente**

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
- 28/09 : dylib de `main` à jour (déjà reconstruite par une autre session). Smoke : seul échec « 3 faction cards, got 10 » (FE, étranger à IB). IB1 et IB3 lancés. **Prochaine étape : fusionner IB1 et IB3 dans `integration/ib`, puis lancer IB4 et IB5.**
- 28/09 : IB3 fusionné dans `main` par ff (6197ed33) via `../gp-ib-merge` (`integration/ib`) ; smoke (seul échec FE connu), `ib_chain_test`, `p2c_ui_test` verts. Worktree IB3 supprimé. En cours : IB1.
- 28/09 : IB1 rendu (89f2425b) et fusionné dans `integration/ib` ; tests verts. Contrôle visuel OK (planche `docs/img/ib/apres/`). Défauts pour IB4 : chiffre vedette sans icône (« Tir »), texte d'ambiance ni italique ni atténué. Écarts IB1 : largeur en unités d'interface (échelle déjà appliquée par `content_scale_factor`), `lower_is_better` ajouté au style, `before`/`after` lus dans `live["before_after"]` = {effet: [avant, après]} (format à fournir par IB5), prérequis ✓ seulement si `available`, sinon • (pas de donnée par prérequis au core → IB5). `main` a bougé (FE5, core) : dylib reconstruite dans `gp-ib-merge` (cible `core/target-ib`, à supprimer à la fin d'IB).
- 28/09 : IB1 dans `main` (df96f3b8, tous tests verts dont smoke). Dylib de `main` rafraîchie depuis `gp-ib-merge`. IB4 et IB5 lancés. **Prochaine étape : fusionner IB4/IB5, puis IB2 (mech).**
- 28/09 : IB4 dans `main` (f679f72f), tous verts (smoke, ib_chain, ib_layout, p2c, po_ui, hud_components, pytest). IB2 lancé (mech). En cours : IB5, IB2. **Prochaine étape : fusionner IB5 et IB2, puis IB6 (planche, jugement du joueur, nettoyage `core/target-ib` et `../gp-ib-merge`).**
- 28/09 : IB5 dans `main` (3e26c0ac) ; cargo test sim-campaign + godot-bridge et 6 tests Godot verts. En cours : IB2.
- 28/09 : IB2 dans `main` (7ac2f0d4) ; batterie complète verte (smoke, ib_plain/chain/layout, p2c, po_ui, hud_components, pytest 7). Worktree IB2 verrouillé par le processus de l'agent : à supprimer plus tard (`git worktree remove --force` + `git branch -d feat/ib2-migrate worktree-agent-a226bf6f2c9fcce90`). **Prochaine étape : IB6 (planche après régénérée, jugement du joueur), puis nettoyage `core/target-ib`, `../gp-ib-merge`, branche `integration/ib`.**
- 28/09 : planche `docs/img/ib/apres/` régénérée sur `main` (IB1-IB5 + IB2). Contrôle visuel du bâtiment complet : sections, liens, « équilibre a → b », ✗ prérequis et ⚠ lisibles. Retouches mineures possibles : vedette « +5 Piété » sans icône ; décimales brutes (« 2.1 → 0 ») à arrondir. **En attente : jugement du joueur (IB6).**
- 29/09 : nettoyage fait par la session RS : worktrees `../gp-ib-merge` et IB2 supprimés, branches `integration/ib`, `feat/ib2-migrate` supprimées (toutes dans main), `core/target-ib` supprimé. Reste : jugement du joueur (IB6).
