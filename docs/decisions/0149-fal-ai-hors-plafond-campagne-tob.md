# 0149 — fal.ai hors plafond v1 pour le chantier TB (campagne façon Thrones of Britannia)

Date : 2026-10-02. Statut : **remplacée par l'ADR 0152 le 02/10 (chantier sans fal.ai)** ; était acceptée (accord du joueur le 02/10, enveloppe de 25 $ validée avec le plan).
Plan : `docs/design/2026-10-02-campagne-tob.md`.

## Contexte
Le plafond cloud de la v1 est de 50 $ (CLAUDE.md, `docs/budget.md`). Le chantier TB (carte de
campagne façon Thrones of Britannia) a besoin d'images et de maquettes générées : planche cible,
bâtiments hors les murs à 3 niveaux, textures de falaises, ornements. Le 02/10, le joueur a accepté
de dépasser le plafond, mais **seulement sur fal.ai**.

## Décision
- Les dépenses fal.ai du chantier TB ne comptent pas dans le plafond de 50 $. Elles ont leur
  propre section dans `docs/budget.md`, avec une **enveloppe de 25 $** (estimation ≈ 12-16 $,
  plan § 4). Au-delà, on redemande au joueur.
- Les autres services payants (OpenRouter, ElevenLabs, etc.) restent sous le plafond v1 et ses
  règles.
- Chaque appel est consigné ligne par ligne (prix catalogue, reprises comprises), comme dans les
  sections GA3 et HB.

## Conséquences
- Les lots TB0, TB3, TB5 et TB7 peuvent lancer des générations fal.ai sans nouvelle confirmation,
  tant que l'enveloppe tient.
- Le conteneur cloud du 02/10 n'atteint pas fal.ai : ces générations se font en session locale ou
  dans un environnement qui autorise ce domaine.
