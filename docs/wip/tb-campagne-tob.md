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
- [ ] Vagues 2 et 3 lancées ensemble le 02/10 : TB3 (`../gp-tb3`), TB4 (`../gp-tb4`), TB5
      (`../gp-tb5`), TB6 (`../gp-tb6`), branches `feat/tb3` à `feat/tb6`, ADR réservés 0153 à 0156.

## Points ouverts de la vague 1
- Ombres de nuages encore sombres et larges en vue moyenne d'été (à juger en jeu).
- Suie par ville non faite (demande un état par ville) ; écume de tempête non vérifiée ; brûlis
  (`terroir_burn`) toujours recouvert par la carte de couleur.
- Sceaux d'incident et sites de rencontre masqués par défaut (case « Signes » des filtres, ADR
  0151) : un incident à plusieurs tours d'échéance est invisible, à juger en partie pilote.
- Noms de région en vue moyenne : non vus sur capture (`ss_shot.gd` masque les `CanvasLayer`).
- `fe_ui_test` échoue (cadrage du sélecteur de faction), sans rapport avec TB.

## Prochaine étape
À la remise de chaque lot : fusion dans `feat/tb` (worktree `../gp-tb`), tests, une capture de
contrôle commune, puis `--ff-only` dans main. Tout **sans fal.ai** (ADR 0152).
- TB3 : villes et bâtiments qui grandissent, avec les kits existants (GA3, ADR 0138, villages TF)
  et Blender ; stub `replace_models` (`settlement_layer.gd`), `model_holder()` null. Périmètre à
  recadrer au lancement.
- TB4 : traces de la guerre et de la peste (plan § 3), dont la suie par ville et le brûlis.

Repères utiles (inventaire du 02/10) : `campaign_season` (`game/scripts/map/season_visuals.gd:11`),
teintes de saison (`game/shaders/campaign_life.gdshaderinc:33-150`), stub de croissance
`replace_models` (`settlement_layer.gd:1643`), carte vivante (`game/scripts/map/life_*`, `life_folk/`).
