# DA2 — Portraits vivants (état de travail)

Branche : `worktree-agent-ade9b18af852c5bfe` (worktree `.claude/worktrees/agent-ade9b18af852c5bfe`,
contient `feat/da-direction-artistique`). La fusion est faite par l'orchestrateur DA, pas par
l'agent. ADR : `docs/decisions/0063-portraits-vivants.md` (0057-0060 laissés libres : trous vus
dans d'autres branches).

Dépenses : 0,27 $ (6 sondes validées, commit `3c99fe02`). Plafond du lot : 8 $.

## Fait
- Données et schéma : `data/portraits/archetypes.json` et `data/schemas/portrait_archetypes.schema.json`
  (122 archétypes, 25 variantes âgées, règles de rang, marques).
- Outil : `tools/cent_ans_tools/portrait_archetypes.py` et `cent-ans assets portrait-archetypes`
  (`--dry-run`, `--only`, `--limit`, `--no-aged`, `--no-archetypes`, `--envelope`).
  Tests : `tools/tests/test_portrait_archetypes.py`. pytest complet : 569 passés.
- Jeu :
  - `game/scripts/ui/living_portrait.gd` : résolution de l'image, FNV-1a, vieillissement,
    rang d'affichage, marques ;
  - `game/scripts/ui/portrait_frame.gd` : cadre par rang, couronne et mitre, écu, deuil,
    barreaux, blessure ;
  - `game/shaders/portrait_marks.gdshader` : grisaille, pâleur, ombre de captivité ;
  - branchement dans `PortraitLoader`, `court_panel`, `character_sheet` et `family_tree_view`
    (le sceau, la rançon et la bataille passent par `PortraitLoader`).
- Tests GDScript : `game/tests/da2_living_portrait_test.gd` (OK). `smoke.gd` passe.
- Captures : `docs/img/da2/avant-*.png` et `apres-*.png`, obtenues avec
  `game/tests/da2_screenshot.gd` (120 tours, printemps 1367).

## Reste à faire
- **Génération des 141 images restantes** (dry-run : environ 6,42 $). La commande a été
  refusée par le garde-fou « transactions réelles » de l'agent : il faut que le joueur
  l'autorise, ou qu'il la lance lui-même :
  `uv run --project tools cent-ans assets portrait-archetypes --envelope 7.6`
  (la commande est idempotente ; relancer après une erreur reprend là où elle s'est arrêtée,
  et chaque passe ajoute sa ligne dans `docs/budget.md`). Ensuite :
  `godot --headless --path game --import`, commit des `.jpg` et `.jpg.import`, puis nouvelles
  captures `--tag=apres`.
- Refaire les captures « après » une fois la banque complète : aujourd'hui, les replis
  habillent tous les hommes en chevalier anglais (sonde unique).
