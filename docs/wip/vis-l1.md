# VIS-L1 SOL-PHOTO (wip)

Lot du rapport `docs/audits/2026-10-30-juge-visuel-iter3.md` (### L1). Branche de départ : VIS-R4
(`worktree-agent-a47dbd6c148e66466`, montagne `mountain_stages` à garder intacte). ADR : 0826.

## Constat de départ (à lire avant de reprendre)

- Le « bake du sol par image » du rapport est déjà fait : ADR 0742 cuit l'albédo régional par **nœud de relief**
  (atlas `texture_2d_array`, texel ≈ 1 pixel écran). Le fragment principal ne fait que lire la couche. Une clipmap
  centrée caméra doublerait ce mécanisme et casserait le lien nœud/LOD ; elle n'est pas construite (voir ADR).
- Le « peigné » vient de `hb_far_patch` et du bloc parcelles de `fields_apply` : cellules en lanières (1:4 à 1:9)
  dont le petit côté tombe sous 2-3 px, échantillonnées une fois par texel sans préfiltrage.
- La couture : `bake_fragment` évalue `regional_ground` avec le **texel du nœud** comme empreinte, constant par
  nœud ; deux nœuds voisins de niveaux différents n'ont pas la même empreinte, donc pas le même régime de fondu.
- La carte de couleur est déjà à s 0,52 / v 0,43 / h 84° (aperçu) : la perte de chroma est dans la chaîne de rendu
  (lumière, AgX, `aerial.desaturate`, grade), pas dans la carte.

## État

- [x] a. Découpage de `terrain.wesl` en modules (sans changement de rendu)
- [x] b. Préfiltre des lanières par largeur écran (données `view.strip_px`)
- [x] c. Chroma et valeur au sol (données `view.ground_look`)
- [x] d. Mesures avant/après
- [x] e. ADR 0826 + INDEX

## Mesures

Voir ADR 0826 (tableau avant/après).

## Reste (voir ADR 0826)

- Textures de culture Z-Image (sillons, chaume, haie) non produites ; v forêt / v pré à relire ; v de d 600 (0,64) trop clair
  (haze, lot L3). Fusion : clippy complet, `client/tests/smoke.sh`, sonde seed bots, mesure GPU (pas lancés dans le lot).
