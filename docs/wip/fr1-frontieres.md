# FR1 — Frontières de faction lumineuses (façon Total War)

Branche : `feat/fr1-faction-borders`. ADR : `docs/decisions/0070-frontieres-de-faction.md`.
**État : terminé, prêt à fusionner** (main fusionnée dans la branche le 2026-09-26).

## Approche

Crochet dans le fragment du terrain : `game/shaders/faction_borders.gdshaderinc`
(`fr1_borders(p, uv, footprint, albedo, emission)`), inclus par `terrain.gdshader` et
`terrain_parchment.gdshader` (2 lignes chacun : include + appel, marquées « FR1 »). Uniformes
`fr1_*` posés sur le matériau partagé du terrain par `game/scripts/map/faction_borders.gd`
(`FactionBorders`), branché dans `campaign_map.gd` (setup, `refresh_all`, `_process`).
Réglages : `data/map/faction_borders.json` (+ schéma, pytest `tools/tests/test_faction_borders_schema.py`).

Premier essai abandonné : passe `next_pass` du matériau du terrain (shader séparé rejouant
`qt_vertex`) — drapé parfait mais +1,1 à +4,1 ms GPU (tout le relief redessiné).

## Mesures (1080p, Haute, `--rendering-driver vulkan`, A/B entrelacé 6×30 images, médianes)

| Vue | Distance | Avec | Sans | Écart |
|---|---|---|---|---|
| Europe | 700 | 12,72 ms | 12,44 ms | +0,28 ms |
| Comté | 60 | 24,76 ms | 24,78 ms | ≈ 0 |
| Vallée (~2 km) | 2,8 | 16,64 ms | 16,63 ms | ≈ 0 |
| Parchemin | 1 500 | 8,56 ms | 8,30 ms | +0,26 ms |

Bruit de mesure ±0,3 ms (runs précédents : Europe +0,34 / +0,52, comté +0,17 / +0,18).
Commande : `godot --path game --rendering-driver vulkan --script res://tests/fr1_shot.gd -- <dossier> --quality=high`
(`--shots-only` : captures sans banc).

## Captures

`docs/audit/captures/fr1/` : `fr1-europe`, `fr1-comte`, `fr1-vallee`, `fr1-parchemin` (+ `-sans`),
`fr1-occupation` (Guyenne et Gascogne occupées par la France, hachures), `fr1-mode-religion`
(encre neutre).

## Points ouverts

- Légende de la carte (UX1, `data/ui/map_legend.json`) : pas encore d'entrée « frontière de
  royaume / occupation » (échantillon à ajouter côté `legend_sample.gd`).
- Frontières maritimes / lacustres : non (le champ de distance ne couvre que les terres).
- Le trait noir de royaume du terrain (`realm_border_*`) est gardé sous le trait coloré ; le liseré
  sombre intérieur (`realm_band_alpha`) est coupé par FR1.
