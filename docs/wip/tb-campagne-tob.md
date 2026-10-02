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
- [x] Vague 1 (02/10) : TB1 saisons visibles (ADR 0150, `docs/wip/tb1.md`) et TB2 désencombrement
      (ADR 0151, `docs/wip/tb2.md`) fusionnés dans `feat/tb` puis main. Capture de contrôle
      (été 1100 / 400, hiver 400, Midlands hors de vue 300) : saisons reconnaissables, terres hors
      de vue en sépia avec relief lisible, frontières discrètes, écus réduits.
- [x] `zg8_relief_test` réparé (`rock_outcrops.gdshader` inclut `campaign_relief.gdshaderinc`).
- [x] Mandat du joueur (02/10) : autonomie complète, enchaîner les vagues sans le consulter ; le
      contrôle avant TB3 devient une relecture par la session principale (capture + tests).
- [x] TB4 traces de guerre (ADR 0157, `docs/wip/tb4.md`), TB5 mer et côtes (ADR 0163,
      `docs/wip/tb5.md`), TB6 lumière et atmosphère (ADR 0156, `docs/wip/tb6.md`) : relus sur
      captures, retouchés une fois chacun, fusionnés dans main le 02/10 (f954a2559). Worktrees
      supprimés.
- [x] TB3 bâtiments qui grandissent (ADR 0162, `docs/wip/tb3.md`) : dans main le 03/10
      (aa9fc2288) après 4 passes. Maquettes en « signes de carte » (8 familles × 3 niveaux),
      posées autour de la maquette de ville GC2 (style par défaut, ADR 0158) ; signes de ville,
      faubourgs 1:1 et enceinte seulement dans le style `real`. Planches :
      `~/.cache/cent_ans/tb/tb3/tb3-planche-*.png`.
- Captures : plafond levé par le joueur le 02/10. Planches hors dépôt dans
  `~/.cache/cent_ans/tb/` (`final1-sheet.jpg` = état de main après TB1, 2, 4, 5, 6).

## Points ouverts
- Ombres de nuages encore sombres et larges en vue moyenne d'été (à juger en jeu).
- Suie par ville non faite (demande un état par ville) ; écume de tempête non vérifiée ; brûlis
  (`terroir_burn`) toujours recouvert par la carte de couleur.
- Sceaux d'incident et sites de rencontre masqués par défaut (case « Signes » des filtres, ADR
  0151) : un incident à plusieurs tours d'échéance est invisible, à juger en partie pilote.
- Noms de région en vue moyenne : non vus sur capture (`ss_shot.gd` masque les `CanvasLayer`).
- `fe_ui_test` échoue (cadrage du sélecteur de faction), sans rapport avec TB.

- TB4 : pas d'historique des batailles dans le pont (marques perdues au rechargement sauf dernier
  tour) → à ajouter dans `core/` ; colonnes de fumée (`Fires`) en traits noirs droits ; charrette
  de peste un peu grosse (`plague.cart_scale`).
- TB5 : écume du rivage (antérieure) encore sur bruit de valeur ; hauts-fonds pâles près de
  Douvres et du Raz adoucis, pas supprimés ; bench à refaire machine calme.
- TB6 : SSIL gardé (médiane 0,58 ms, bruit ± 2 ms), SDFGI écarté ; brume très discrète.
- Tests sensibles à la charge (> 60) : `tb6_light_test`, `fk_folk_test` (seuil 8 ms),
  `da7d_overlap_test` ; `tb2_declutter_test` stabilisé (seconde passe complète avant mesure).
- Autres sessions sur la carte : HC (arbres, forêts, lacs, `feat/hc`), GC, SA (armées), EN
  (rouge réservé aux ennemis), FA (assets de bataille). TB ne touche pas leurs fichiers.

## Prochaine étape
1. Fusionner l'historique des batailles (`../gp-tb8`, `feat/tb-battles`) : vérification
   fmt / clippy / cargo test / Godot en cours sur une base à jour de main ; puis `--ff-only`.
2. Restes TB3 : murs bruns du kit contre murs clairs GC ; suie et enceinte construites en jeu
   invisibles sur la maquette GC (matériau partagé, à voir avec la session GC) ; pas de
   bâtiments hors les murs pour villages, châteaux, abbayes ; mine jamais vue en situation ;
   ombres portées jusqu'à 330 (97 appels de dessin à 90).
3. Reste TB5 : aplats clairs à bords anguleux dans l'eau côtière (vu à La Rochelle).
4. Bancs à refaire machine calme (TB3, TB5, TB6). TB7 (interface) reporté (ADR 0152).

Repères utiles (inventaire du 02/10) : `campaign_season` (`game/scripts/map/season_visuals.gd:11`),
teintes de saison (`game/shaders/campaign_life.gdshaderinc:33-150`), stub de croissance
`replace_models` (`settlement_layer.gd:1643`), carte vivante (`game/scripts/map/life_*`, `life_folk/`).
