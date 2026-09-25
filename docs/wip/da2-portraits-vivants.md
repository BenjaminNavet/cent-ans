# DA2 — Portraits vivants (état de travail)

Branche : `worktree-agent-ade9b18af852c5bfe` (worktree `.claude/worktrees/agent-ade9b18af852c5bfe`, contient la fusion de `feat/da-direction-artistique`) ; fusion par l'orchestrateur DA, pas par l'agent.

**EN PAUSE (demande du joueur, 25/09).** Coût dépensé : **0,00 $** (aucune requête payante envoyée ; seul le `--dry-run` a tourné).
Plafond : 8 $ (section « Direction artistique (25/09) » de `docs/budget.md`).

## Conception
- Données : `data/portraits/archetypes.json` (+ schéma `data/schemas/portrait_archetypes.schema.json`) :
  tranches d'âge (enfant ≤ 15, jeune ≤ 29, adulte ≤ 49, âgé), 4 aires d'habit (France/Bourgogne,
  Angleterre/Écosse, Ibérie, Italie/Empire), 6 rangs (enfant, souverain, grand noble, chevalier,
  prélat, bourgeois), 4 visages par sexe, cases générées (122 images), replis, règles de rang
  d'affichage, 25 variantes âgées, traits des marques.
- Outil : `tools/cent_ans_tools/portrait_archetypes.py` + `cent-ans assets portrait-archetypes`
  (`--dry-run`, `--only`, `--no-aged`, `--no-archetypes`). Réutilise `portraits.STYLE`,
  `portraits.generate` (enveloppe, plafond, une ligne de budget). Variantes âgées : portrait
  existant envoyé en référence (`openrouter.request_image(images=...)`).
- Jeu : `game/scripts/ui/living_portrait.gd` (résolution du portrait : fixe, variante âgée,
  archétype par hachage FNV-1a de l'id) et `portrait_frame.gd` (cadre par rang, couronne/mitre,
  pâleur, blessure, deuil, captivité, grisaille des morts).

## État
- [x] Données + schéma, outil Python, commande CLI, dry-run (147 images ≈ 6,69 $).
- [x] Captures « avant » : `docs/img/da2/avant-cour.png`, `avant-arbre.png` (script
  `game/tests/da2_screenshot.gd`, 120 tours = printemps 1367, ~15 s ; avec affichage :
  `godot --resolution 1600x900 --path game --script res://tests/da2_screenshot.gd -- --tag=avant`).
  On y voit les personnages nés en jeu réduits à l'écu de France.
- [ ] Sonde 3 images (`--only <clé>` ×3, p. ex. `noble_male_young_france_0`,
  `knight_male_adult_england_1`, `chr_edward_iii_old`), validation visuelle.
- [ ] Génération complète.
- [ ] GDScript : résolveur, cadre, branchement (cour, arbre, fiche, sceau, rançon…).
- [ ] Tests (pytest, GDScript), captures avant/après `docs/img/da2/`, ADR.

## Prochaine étape
1. Reprendre (après levée de la pause) : `uv run --project tools cent-ans assets portrait-archetypes --dry-run`
   puis la sonde de 3 images ; lire les JPEG ; si style conforme, génération complète
   (`--envelope 7.6`) ; vérifier la ligne ajoutée à `docs/budget.md` (section DA).
2. GDScript : `living_portrait.gd` (rang d'affichage : dirigeant/conjoint via
   `get_family_tree(id,0,0)` {ruler, heir}, rôle de données via `SimFacade.store.get_character`,
   mots-clés du titre, armée → chevalier ; tranche par âge ; hachage FNV-1a de l'id → visage ;
   replis rang/tranche) puis `portrait_frame.gd` (marques) ; brancher dans
   `PortraitLoader.portrait_texture/overlay_portrait` pour couvrir cour, arbre, fiche, sceau,
   rançon, bataille.
3. Tests pytest (schéma, 122 archétypes, prompts), test GDScript, captures « après », ADR
   (prochain numéro libre après 0055, à revérifier dans main).
