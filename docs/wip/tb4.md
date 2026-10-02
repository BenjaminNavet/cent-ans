# TB4 — conséquences visibles de la guerre et des fléaux (note de reprise)

Spec : `docs/design/2026-10-02-campagne-tob.md` § 3 « TB4 ». Branche `feat/tb4`, worktree
`/Users/jean_hubert/dev/gp-tb4`. Rendu Godot seulement (`core/` intact).

## État
- [x] Squelette du test `game/tests/tb4_scars_test.gd`.
- [x] 1. Brûlis (`terroir_burn`) par-dessus la carte de couleur (SS2) et les matières (HB3) : appel déplacé après `sg_apply` / `hb_apply` dans `terrain.gdshader` ; couverture moyenne au loin (`campaign_life.gdshaderinc`) ; masque de terroir relu à son échelle (voir « Écarts »).
- [ ] 2. Peste : fosses, croix sur les portes, charrette des morts (3D, palier proche).
- [ ] 3. Champ de bataille marqué quelques tours (tertre, corbeaux, débris), durée dans `data/`.
- [ ] 4. Siège : engins en construction (N7) visibles dans le camp au fil des tours.

## Prochaine étape
Points 2 à 4 : `game/scripts/map/war_scars.gd` (réglages `data/ui/war_scars.json`), branché par
`CampaignLife`.

## Mesures du brûlis (02/10)
`godot --path game --resolution 1600x900 --script res://tests/ss_shot.gd -- --stats --season=summer
--distances=1100,400,90,40,15 --hide=Clouds --param=weather_enabled=false --param=cloud_shadow_amount=0
--devastate=prov_ile_de_france:100` (Paris, moyenne RVB du tiers bas ; sans `--devastate` : témoin).

| Distance | Témoin (non dévasté) | Dévasté, brûlis sous la carte (avant) | Dévasté, brûlis par-dessus (après) |
|---|---|---|---|
| 90 | 95 79 48 | 112 92 59 | 71 59 41 |
| 40 | 94 79 48 | 111 92 59 | 73 62 48 |
| 15 | 95 77 48 | 113 91 58 | 61 53 44 |

À 1100 et 400, le tiers bas de l'image sort de l'Île-de-France : pas d'écart (88 79 46 / 95 85 46).
« Avant » : même masque corrigé, ancien ordre (le brûlis ne noircissait rien : la zone dévastée
ressortait plus claire, champs remplacés par la friche). Avant tout correctif, dévasté = témoin
(95 78 48 à 90) : le masque était décalé (voir ci-dessous) et le brûlis recouvert.

## Écarts à la spec
- **Masque de terroir décalé (défaut antérieur, corrigé ici)** : `TerroirMask` peint un masque carré
  à la même échelle sur les deux axes (1024 / plus grand côté), mais `terrain.gdshader` le lisait
  avec `uv = p / map_size`. Depuis la carte 7168 × 6144 (OM), finages, vignes et brûlis étaient
  décalés vers le nord d'un sixième de l'ordonnée (Paris : ≈ 457 px). Lecture corrigée
  (`p / max(map_size)`) : sans cela, aucun brûlis n'apparaît là où la province est dévastée.
  Conséquence visible sur toute la carte : les finages (champs autour des colonies) reviennent
  autour de leur colonie ; les chiffres `--stats` de TB1 près de Paris changent (40 : 112 93 59 →
  94 79 48).

## Points ouverts
- La première exécution de `ss_shot.gd --stats` après un changement de shader peut donner des
  chiffres faux (cache de pipelines froid : 73 76 88 au lieu de 88 78 46) : relancer une fois.
