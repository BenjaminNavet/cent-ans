# CO-E — images du Codex (à reporter dans la section CO-E de `co-colonies.md`)

Branche `worktree-agent-afb58be858d1e0727`. Génération locale (Z-Image Turbo, ADR 0190), 0 $, aucune ligne dans `docs/budget.md`.

## Logique réelle du jeu (`game/scripts/codex/codex_window.gd`, `art_path_of`)
Ordre : `illustrations/<cdx id>.jpg`, puis (désormais) `portraits/chr_<slug>.png`, puis pour l'`entity` :
`events/`, `illustrations/`, `portraits/`, `icons/entity/<entity>.png`. Le calcul « 76 sans image »
ignorait le portrait par identifiant (4 personnages : Adolphe de La Marck, Baudouin de Luxembourg,
Jean le Bel, Taddeo Pepoli, tous déjà couverts par `portraits/chr_*.png`) : **rien à générer** pour eux.
`has_entity_image` connaît maintenant ce repli (`SLUG_IMAGES`).

## Couverture
- 64 entrées nature sans aucune image (animal 24, arbre 23, roche 9, oiseau 8) : 1 planche chacune.
- 8 entrées économie n'avaient qu'une icône `icons/entity/res_*.png` (trop petite pour une fiche) : 8 miniatures de scène
  (une scène concrète par ressource, sans titre dans le prompt : la première passe avait peint des titres).
- 2 plantes (pavot, saule) empruntaient une miniature de technologie (éponge soporifique, écorce de saule) : planches d'herbier propres.
- Total : 74 images 640×360 dans `game/assets/illustrations/cdx_*.jpg` (+ 74 `.import` écrits à la main selon le modèle des existants).
- Style nature : planche de bestiaire / herbier / lapidaire sur vélin, sujet isolé, sans texte (`codex_art.NATURE_STYLE`, sujets anglais par entrée `NATURE_SUBJECTS`).
- Rejets/refaits : cerf (rouge flagrant, refait), phoque (ressemblait à un chien, refait), arbousier (fraisier, refait),
  bouleau (assiette en fond, refait), chêne vert (échec technique, refait), 8 économie (texte peint, refaites).
  Bords sombres de 3 px (ours, terres arides, + cadre bois/drap) rognés à l'installation.
- Acceptés tels quels (imparfaits, non flagrants) : chêne vert (feuilles de chêne lobées), cerf b (robe rousse), peuplier, saule, blocs erratiques (cadre rectangulaire visible).

## Revue des réutilisations (233 entrées réutilisant une image d'entité)
Filtre texte catégorie de l'entrée / préfixe de l'entité (evt, fac, bld, tech, unit, chr) : 28 combinaisons, la quasi-totalité cohérente
(bataille→evt, bâtiment→bld, technique→tech, unité→unit, personnage→chr, lieu→fac pour les États). Vignettes inspectées pour les cas douteux :
- pavot → `tech_soporific_sponge` et saule → `tech_willow_bark` : scènes de soins pour une fiche de plante, **remplacés** (planches d'herbier).
- Froissart → `evt_chroniqueur`, Jeanne de Flandre → `evt_hennebont`, Rouen → `evt_proces_de_rouen`, pèlerinage → `evt_pelerinage`,
  reliques → `evt_miracle_local`, papauté d'Avignon → `fac_papacy`, Hugues Quiéret → `evt_sac_de_southampton` : pertinents ou acceptables, conservés.
- Hugues Quiéret possède un portrait `chr_hugues_quieret.png` : le jeu le préfère désormais à la miniature d'événement (ordre `art_path_of` modifié).
- Non inspectés visuellement (texte cohérent) : bataille→evt (13), batiment→bld (24), technique→tech (29), unite→unit (30), medecine→tech (5), lieu→fac (21), institution/religion/savoir/guerre/dynastie/economie.
  Point ouvert : `cdx_taille` → `tech_royal_taxation`, `cdx_ost_feodal` → `bld_muster_field` à vérifier à l'œil si le joueur s'en plaint.

## Outils
`codex_art.py` : catégories nature en tête de `CATEGORIES`, `NATURE_*`, `ECONOMY_SCENES`, plantes sans réutilisation d'entité, `SLUG_IMAGES`.
Tests : `tools/tests/test_codex_art_nature.py`. Smoke Godot non concluant dans ce worktree (pas de dylib à jour : `economy.json` rejeté par la dylib de main) ; la modification GDScript est un simple réordonnancement de candidats.
