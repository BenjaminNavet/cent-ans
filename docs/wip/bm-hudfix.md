# bm-hudfix : cloche de fin de saison, pastilles d'alerte, minicarte (client Bevy)

Branche `bm/hudfix`, dossier `client/src/campaign/hud/` (seal.rs, mod.rs alertes, minimap.rs).

État : en cours.
1. Cloche : port exact de `end_turn_cluster.gd::_draw_medallion` (rayon 62, image entière sans découpe, états `IconLibrary.medallion_states`, ombre, liseré rubrique seulement si bloqué, banderole). 
2. Pastilles : 190×30, bandeau parchemin / cire, filet gauche, disque d'icône, cartouche du compteur.
3. Minicarte : couleurs pleines du mode courant, cadre trapèze de la vue caméra, rangée Filtres (F) / Politique / Relief / Légende en boutons enluminés.

Prochaine étape : voir l'ordre ci-dessus.
