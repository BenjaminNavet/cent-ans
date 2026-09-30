# Réalisme — suite des lots (plan approuvé par le joueur le 30/09)

Chantiers de référence : `docs/wip/sr.md` (SR, fusionné), `docs/wip/cr.md` (cavaliers),
`docs/wip/fc.md` (FPS et décor campagne). Mandat : autonomie, semi-réaliste, NB2 ≈ 20 $ au
total (3,52 $ dépensés). La conversion image → 3D des soldats (fal.ai) se fait dans une AUTRE
session : ne pas la lancer ici ; fusionner vite pour la débloquer.

## Consigne de reprise
> Lis ce fichier, `docs/wip/cr.md`, `docs/wip/fc.md`, `git log --oneline -15`, puis continue au
> premier lot non coché. Brutes hors dépôt : `~/dev/cent-ans-raw/`.

## Lots
- [x] L1 — FC : corriger `fc2_impostors_test` (échec après fusion de main), fusion `--ff-only`.
- [x] L2 — CR2+CR3 : acier crédible, bassinets, caparaçons drapés ; fusion.
- [ ] L3 — CR4 cheval et lances : crinière et queue en mèches (cartes alpha), robe avec normale
      de poil, lances d'angles et de longueurs variés, flammes en tissu ondulant ; recuisson,
      captures A/B (agent visuel unique, ≤ 30 captures).
- [ ] L4 — Colombage `TimberFrame` (SR5, à faire) : réexport du kit Blender avec la couche en fin
      de tableau et correction de `first_plain`, choix régional (Normandie, Île-de-France,
      Angleterre, Flandre : colombage ; Midi : pierre/torchis), toits bleus du château Kenney.
- [x] L5 — Herbe de campagne plus lisible (d 12-20), clé `veg_max_distance` par préréglage
      (portée des imposteurs 700 → 1000 en Haute/Ultra si le budget le permet).
- [ ] L6 — FC4 banc A/B Metal sur machine calme (aucun agent actif) : `main` avant FC contre
      après, 3 passes chacun, résultats dans `docs/wip/fc.md` et l'ADR 0137.

## Journal
- 09-30 : plan écrit ; SR et CR1 dans main (dfdf05026).
