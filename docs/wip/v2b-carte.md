# V2b — Finitions de la carte de campagne semi-réaliste (branche `visual-v2b`)

Depuis `visual` (V1-V4 fusionnés). Captures de travail dans le scratchpad, finales dans
`docs/img/visuel/v2b_*.png`.

## Plan
1. Parcellaire commun terrain/haies : grille biaisée déformée (même fonction en GDScript et en shader),
   enclos bordés de haies, lanières de cultures dans chaque enclos, prés, jachères, vignes.
2. Haies plus fines, plus claires, interrompues, arbres de haie.
3. Occupation du sol par province (vigne, sécheresse) : texture basse résolution floutée.
4. Palette chaude, atmosphère moins bleue ; contrôle Angleterre/Écosse, Midi/Espagne, Alpes.
5. Fondus de zoom, étiquettes (city_markers.gd), banc de performance.

## État
- Démarré : lecture, captures de base (bocage : quadrillage de haies sombres, parcelles Voronoï à facettes,
  prairie bleutée par le brouillard).

- Étapes 1-3 codées : `game/scripts/map/vegetation_fields.gd` (parcellaire partagé + occupation du
  sol par province), `terrain.gdshader` (`field_at` : enclos, lanières, prés, vignes, chemins, pied de
  haie), `vegetation_tile_job.gd` (haies sur les mêmes bords, fines, trouées, arbres de haie),
  `vegetation_mask.gd` (`hedge_at` lit le canal B de l'occupation du sol). Capture `n1` : bien plus
  naturel ; la Normandie orientale (plaine) a trop peu de haies, le bocage est trop vert/bleu.

## Prochaine étape
- Vérifier le bocage (canal B dilaté), la palette (atmosphère moins bleue), les fondus, les étiquettes.
