# ADR 0770 : forêts, saisons des arbres et hameaux composés de la campagne

Date : 2026-10-23. Lot FORET (suite du lot TERRAIN).

## Contexte
Les juges (iter-02/03) relevaient des arbres « boules de coton » à échelle démesurée, des saisons peu lisibles sur la végétation et aucun hameau. Le sol de campagne étant noir avant le correctif fb6b6e23d, le rendu réel n'avait pas été jugé. Au rendu corrigé : les grandes forêts ont un bon grain, les bosquets isolés sont des masses lisses trop grandes, les bosquets d'hiver à distance moyenne deviennent des boules blanches (les trous de ramure ne sont pas tracés au-delà du mip 1,5), printemps et été sont quasi identiques sur les arbres, et les 3 495 hameaux de `data/map/hamlets.json` ne portent aucun bâtiment (les villages sont dessinés par `towns/`).

## Décision
- Arbres (aucun nouvel échantillonneur, coût ALU seulement) : paramètre de données `far.near_scale` (0,8) de `campaign_forest_look.json` : taille des couronnes près de la caméra, ramenée à 1 par la croissance lointaine existante ; l'ombre portée (`shadow_cast.wesl`) suit. Feuillus : printemps jaune-vert, été vert plus sombre ; hiver : au-delà du mip 0,8 la ramure se voit par assombrissement modulé par le bruit des feuilles, pas par des trous.
- Hameaux : composés en données seules par le mode `village` (kind `hamlet`) de `map_countryside.json` : quatre anneaux de maisons par région (ouest, nord, Angleterre, sud, Ibérie, montagne) tirées des modèles `dn/buildings` régionaux (colombage et chaume, brique, tuile, adobe, pierre), jardins ; grossissement `size_scale` 1,7, visibles sous 90 unités, jamais en forêt (filtre `forest_max`, ajouté au mode `village` dans `placement.rs`).
- Les modèles `dn/` sont des fichiers locaux ignorés par git : sans eux, `proto` journalise l'échec et saute la maison.

## Conséquences
Pas de nouvelle ressource GPU. Cours et haies : seule la cour de la ferme normande (modèle) existe ; des haies de hameau ne sont pas posées (le mode `village` ne sait pas étirer un prop), à traiter avec un mode « enclos » ancré aux hameaux. L'ardoise n'a pas de modèle régional dédié.
