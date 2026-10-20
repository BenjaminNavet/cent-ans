# TS-T2 — forêts style Total War (ADR 0710)

Branche `ts/t2`, worktree `/Users/jean_hubert/dev/gp-ts-t2`. Plan : `docs/wip/ts-style-total-war.md`.

## État : fait (à fusionner par l'orchestrateur)
- `data/art/campaign_forest_look.json` + schéma `art_campaign_forest_look`, lu par `vegetation/look.rs`.
- Canopée : valeur 0,72, saturation 1,3, balance verte, variété arbre/peuplement, ombre du bas
  du houppier, variété de hauteur par peuplement (impostors + modèles + ombres portées).
- Tramage : vues d'azimut fondues ; bande modèle/imposteur tirée arbre par arbre (CPU).
- Vue moyenne/lointaine : imposteurs agrandis avec la distance (140 → 600 u, +70 %) : masses sombres.
- Lisière nette (`edges.rs`) : sous `edge.cut` seuls des isolés / petits bosquets groupés par cellules.
- Limite des arbres (`mask.rs`) : bande de 90 m ondulée ±120 m, forêt coupée sur pente 0,5-0,68.
- Hiver : conifères givrés (dessus du houppier, plus en altitude, 30 % verts en plaine),
  feuillus nus gris-brun (0,22/0,18/0,15), ajourés de près seulement.

## Diagnostic
- Atlas d'imposteurs : couleur moyenne opaque sRGB (82, 80, 58), kaki terne → taches beiges au loin.
- Blocs gris « chou-fleur » : PAS la végétation (restent avec `CENT_ANS_PERF_OFF=veg`) : affleurements
  rocheux `campaign/countryside/rocks.rs` (agrandis avec la distance) → hors lot.

## Restes
- Ombres portées très noires sous les arbres sur la neige (éclairage du sol, lot T1/T3).
- Mélange de conifères : laissé aux données existantes (`conifer_raster`, altitudes des essences).
- Perf : pas plus d'instances ; surcoût = 2 échantillons de plus par fragment d'imposteur + remplissage
  des cartes agrandies au loin (non mesuré sur machine calme).
