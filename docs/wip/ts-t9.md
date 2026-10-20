# TS-T9 — rochers et ombres sur la neige

Branche `ts/t9` (worktree `gp-ts-t9`). Plan : `docs/wip/ts-style-total-war.md`.

## Plan
1. Rochers (`countryside/rocks.rs`, `data/art/rock_outcrops.yaml`) : croissance lointaine plafonnée
   (`max_display_m`, `far_scale` réduit), teinte sombre (`albedo_scale`), enfoncement (`sink`),
   moins nombreux en plaine enneigée l'hiver (`winter_flat_keep`), dump par taille.
2. Ombres : plancher d'opacité de l'ombre au sol (`ground_look.shadow_max_opacity`, bleutée par la
   couleur du ciel) dans `terrain.wesl` (changement minimal) ; fonction pure testée dans `shadows.rs`.

## État
Squelette.
