# Lot BATDA (boucle juge, direction artistique des batailles)

Source : out/juge2/iter-01-art.md.

## Etat
- Constat 3 (prepass sans normales) : cause trouvee = `SkyDome` et `Skyline` (client/src/battle/decor/horizon.rs),
  maillages opaques sans normales rendus dans la prepass normale de la SSAO. Normales ajoutees (dome vers l'interieur, skyline vers le haut).
- Constat 1 (cavalerie monochrome) : cause = reglage de livree `data/fx/battle_soldier_look.json` (desaturation 0,5, valeur 0,5-0,78
  sur un bleu #1F3A93 deja sombre). Reglage releve (0,3 ; 0,68-0,95 ; far_glow 0,18). Bannieres : echelle selon la distance
  maintenant dans le meme fichier (`banner.scale_per_m` 0,02, min 1,0).
- Constat 2 (terrain/horizon) : voir ci-dessous.

## Prochaine etape
Controle visuel par la session principale (`tools/compare_views.sh --only battle_default`, `--only battle_low`).

## Constat 2
Dump : horizon actif, `real_relief` vrai (prov_julich, anneau jusqu'a 175 m), panorama `picardy_plains`. Le relief reel et l'horizon
existent deja (ADR 0020/0032). Cause probable de la bande sombre : teintes des bois lointains trop sombres (`far_woods.tints`) contre un
ciel clair : eclaircies et variees dans data/fx/battle_decor_look.json. Reste a juger a l'oeil.

## Tests
`cargo test --features dev --lib` : 3 echecs (campaign::markers::dn, campaign::provinces::data, menu::historical), hors perimetre, non verifies sur main.
