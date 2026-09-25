# DA3 — Marqueurs de carte (langage unique)

Lot DA3 de `docs/wip/da-direction-artistique.md`, bible `docs/design/2026-09-25-bible-da.md` § 7-8.
Branche `worktree-agent-a4046e3d4756f0516` (fusionnée avec `feat/da-direction-artistique` et main).
Plafond du lot : 1,5 $ (clé OpenRouter personnelle, section DA de `docs/budget.md`). ADR 0060.

## Constat (captures `docs/img/da3/*_avant.png`)
- `SettlementLayer` : un `MultiMesh` d'icônes (`settlement_icon.gdshader`) à formes SDF
  (écu crénelé, disque crénelé = « rosace/soleil », carré, disque à croix, point) remplies de la
  **couleur de faction** du contrôleur (rose Angleterre, jaune Flandre, bleu France…), au seul
  palier moyen.
- Palier Europe (parchemin, CM2) : `ParchmentOverlay` dessine une vignette à l'encre par cité
  (toutes), drapeau de couleur pour les capitales : autre langage.
- Légende (UX1) : `LegendSample.draw_settlement` redessine les formes SDF.

## Plan
1. Données : `data/map/settlement_markers.json` + schéma (pictogrammes, rang, tailles, paliers).
2. Outil `tools/cent_ans_tools/map_markers.py` (+ CLI `assets map-markers`, `--dry-run/--only`) :
   génération gpt-5-image-mini d'un pictogramme par entrée, détourage, liseré, atlas 4×4 de 128 px.
3. Shader : atlas + écu composé (atlas d'écus construit à l'exécution depuis
   `PortraitLoader.heraldry_texture`), visibilité par instance (`visible_until`) : pas de CPU par image.
4. `SettlementMarkers` (GDScript) : rang, pictogramme, visibilité ; `SettlementLayer` s'en sert ;
   parchemin sans vignettes de villes ; légende mise à jour.
5. Mesures i/s avant/après (`--fps-probe`), captures après, ADR, budget.

## État
- [x] Captures avant (europe/province/comté) ; squelette données + schéma.
- [ ] Outil + sonde 3 images
- [ ] Génération complète + atlas
- [ ] Shader + GDScript
- [ ] Perf, captures après, ADR, budget

## Prochaine étape
Écrire `map_markers.py` et ses tests, puis la sonde (3 images).
