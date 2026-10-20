# TS-T9 — rochers et ombres sur la neige

Branche `ts/t9` (worktree `gp-ts-t9`). Plan : `docs/wip/ts-style-total-war.md`.

## Plan
1. Rochers (`countryside/rocks.rs`, `data/art/rock_outcrops.yaml`) : croissance lointaine plafonnée
   (`max_display_m`, `far_scale` réduit), teinte sombre (`albedo_scale`), enfoncement (`sink`),
   moins nombreux en plaine enneigée l'hiver (`winter_flat_keep`), dump par taille.
2. Ombres : plancher d'opacité de l'ombre au sol (`ground_look.shadow_max_opacity`, bleutée par la
   couleur du ciel) dans `terrain.wesl` (changement minimal) ; fonction pure testée dans `shadows.rs`.

## État (fait)
- Rochers : `capped_scale` (plafond `max_display_m` 700 m, `far_scale` 1,7 -> 1,3), `albedo_scale` 0,6 (roche
  sombre), enfoncement `sink` 0,18, amincissement hivernal en terrain plat (`winter_flat_keep` 0,3 sous la pente
  `flat_slope` 0,08, hachage stable). Dump `rocks` : `size_buckets_m`, `winter_thinned`, `max_display_m`, `sink`.
- Ombres : les arbres projettent dans la carte d'ombre du soleil ; le noir venait du sol (ambiant seul).
  `terrain.wesl` (fin du fragment) : plancher `1 - ground_look.shadow_max_opacity` (0,4) de la luminance du sol
  éclairé, teinte = lumière ambiante (ciel, bleutée). Uniform : `season.patches.w` (inutilisé jusque-là).
  Miroir testé : `shadows::capped_shadow`.
- Vue Grésivaudan d=60 hiver (avant plafond dur) : 1456 visibles, 316 < 500 m, 797 < 800 m, 343 > 800 m, 59 amincis.

## Restes
- Neige sur le dessus des rochers : non faite (matériau partagé par affleurement ; il faudrait un shader).
- Ombre « collée au pied » : l'ombre est celle de la carte d'ombre (cascades), non retouchée ; à juger à l'oeil.
