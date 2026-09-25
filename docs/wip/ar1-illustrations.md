# WIP — AR1 : habillage illustré (chargements, vignettes d'événements, fins)

Branche : `worktree-agent-aaa4cf9bc2224eea2`.

## Principe

- Données : `data/ui/illustrations.json` (schéma `data/schemas/illustrations.schema.json`) :
  `loading.screens` (contexte battle/siege/naval/campaign, titre, citation de chroniqueur),
  `vignettes` (genres d'événements du cœur, ordre = priorité), `endings` (4 issues).
- Pipeline : `tools/cent_ans_tools/art_plates.py` ; enluminures du domaine public
  (Wikimedia Commons, cache local `tools/.cache/commons/`, ignoré par git), recadrage
  déclaré dans les données ; génération OpenRouter (gpt-5-image-mini, comme UR1) seulement
  pour les manques, jamais deux fois un fichier existant.
- Images : `game/assets/art/{loading,vignettes,endings}/*.jpg`.

## État

| Étape | État |
|---|---|
| Recherche domaine public (Commons : Froissart BnF fr. 2643, BL Royal 18 E I, Vigiles de Charles VII…) | fait |
| Données, schéma, outil, 27 planches Commons | fait |
| Génération des 4 manques (Avignon, famine, trésor, commerce) : 0,19 $ réels | fait |
| Intégration Godot (chargement bataille/siège/naval, vignettes, fins) | à faire |
| Tests pytest, CREDITS.md, captures | à faire |

## Prochaine étape

Essai de génération sur 1 image, puis le reste ; intégration Godot.
