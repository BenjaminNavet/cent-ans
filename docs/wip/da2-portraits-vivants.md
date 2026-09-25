# DA2 — Portraits vivants (état de travail)

Branche : worktree `agent-ade9b18af852c5bfe` (fusion par l'orchestrateur DA, pas par l'agent).
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
- [ ] Sonde 3 images, validation visuelle.
- [ ] Génération complète.
- [ ] GDScript : résolveur, cadre, branchement (cour, arbre, fiche, sceau, rançon…).
- [ ] Tests (pytest, GDScript), captures avant/après `docs/img/da2/`, ADR.

## Prochaine étape
Sonde de 3 images puis GDScript.
