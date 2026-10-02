# TB — carte de campagne façon Thrones of Britannia (orchestration)

Demande du joueur (02/10) : rendre le jeu « beaucoup plus joli », référence principale **Thrones of
Britannia**, priorité **campagne**, dépassement du plafond de 50 $ accepté **uniquement sur fal.ai**,
références à récupérer en ligne.

## État
- [x] Recherche des références ToB : texte fait (sources dans le plan). Images : bloquées par le
      réseau du conteneur cloud (Steam, ArtStation, Wikimedia, presse et fal.ai refusés).
- [x] Inventaire de l'existant de la carte de campagne (plan § 2)
- [x] Plan `docs/design/2026-10-02-campagne-tob.md` (lots TB0-TB7)
- [x] ADR 0149 (fal.ai hors plafond, enveloppe 25 $) + section dans `docs/budget.md`
- [x] Plan validé par le joueur (02/10) ; la suite se fait en session locale
- [x] TB0 **sans repeints** (ADR 0152, 02/10 : plus de crédit fal.ai, le joueur renonce ; chantier
      à 0 $). Critères = 13 références ToB dans `~/.cache/cent_ans/tb/refs/` (hors dépôt :
      `steam-05/06/07`, `yt-a-*`) et nos captures « avant » dans `~/.cache/cent_ans/tb/ours/`
      (`game/tests/ss_shot.gd`, Paris, distances 1100 / 400 / 90 / 35). Le joueur juge la
      direction sur le résultat de la vague 1 avant TB3.
- [ ] Vague 1 : TB1 saisons visibles (`../gp-tb1`, `feat/tb1`), TB2 désencombrement
      (`../gp-tb2`, `feat/tb2`) — lancés le 02/10. TB1 livré sur `feat/tb1` (retouches neige / mer / automne en
      cours) ; TB2 en cours. Échec `zg8_relief_test` déjà présent sur main (`rock_outcrops.gdshader`
      redéclare `campaign_vertical_scale`) : à corriger à la fusion.

## Prochaine étape (reprise en session locale)
Le travail reprend sur la machine du joueur, avec Godot, Blender, fal.ai et un accès web complet :
`git fetch origin && git checkout claude/dazzling-maxwell-bk09hi`.

1. **TB0** : rapatrier 10 à 15 captures ToB (non versionnées, droits tiers) : campagne large,
   moyenne et proche, hiver, colonie mineure, capitale, brouillard, vue stratégique. Prendre 3
   captures de notre carte (large, moyenne, proche, scripts `game/tests/*_shot.gd`). Les repeindre
   avec `fal-ai/nano-banana-2/edit` vers le rendu visé. Consigner chaque appel dans
   `docs/budget.md` (section TB). Faire valider la planche par le joueur.
2. **Vague 1, en parallèle** (zones distinctes) :
   - TB1 : commencer par le masquage de la teinte saisonnière par la carte de couleur
     (`game/shaders/satellite_ground.gdshaderinc:22-28`, même logique dans hb_ground, ADR 0143) ;
     puis mer et étalonnage par saison (`game/scripts/map/turn_light.gd`, `water.gdshader`),
     neige et suie sur les toits 1:1 (uniform `snow` sans setter ; `model_holder()` null dans
     `game/scripts/map/settlement_layer.gd:1614`).
   - TB2 : nuages, brouillard de guerre (`terrain.gdshader:45-57`), pictogrammes, étiquettes et
     noms de région (`settlement_markers.gd`, `marker_declutter.gd`, `label_placer.gd`).
3. Puis vague 2 (TB3, TB4), selon le plan § 4.

Repères utiles (inventaire du 02/10) : `campaign_season` (`game/scripts/map/season_visuals.gd:11`),
teintes de saison (`game/shaders/campaign_life.gdshaderinc:33-150`), stub de croissance
`replace_models` (`settlement_layer.gd:1643`), carte vivante (`game/scripts/map/life_*`, `life_folk/`).
