# bm-hudfix : cloche de fin de saison, pastilles d'alerte, minicarte (client Bevy)

Branche `bm/hudfix`, dossier `client/src/campaign/hud/` (seal.rs, mod.rs, minimap.rs).

État : fait.
- Cloche (`end_turn_cluster.gd::_draw_medallion`) : rayon 62 (124 px), image `end_turn.png` entière sans découpe ni anneau ajouté, ombre à 0,86 du rayon, états survol / enfoncé / désactivé dérivés de l'image par `adjust_bcs` (valeurs de `IconLibrary.medallion_states`), liseré rubrique seulement si une décision bloque, banderole 100 × 34 (20 sans 2e ligne) à mi-rayon, filet d'or.
- Pastilles (`AlertBadge._draw`) : 190 × 30, parchemin ou cire (types dangereux), filet gauche rubrique / or, disque d'icône cerclé d'or, libellé 15 px, cartouche du compteur.
- Minicarte : couleurs pleines du mode de carte (politique = couleur héraldique du propriétaire, autres modes = teinte du filtre à pleine force, terrain = parchemin), trapèze de la vue caméra (4 coins projetés au sol, encre + cœur pâle), armées en disque cerclé d'or (joueur) / losange (autres), cadre d'encre ; rangée Filtres (touche F, menu des modes, Échap le ferme avant la pause) / Politique / Relief / Légende en boutons enluminés du kit.

Écarts : pas de filet d'or reliant les pastilles à la cloche ; pas de `fit_to` (la minicarte garde 280 px de large, Godot la rétrécit à sa zone) ; le menu Filtres n'a pas les cases « Routes commerciales » / « Signes » (couches absentes du client) ; l'état « fenêtre modale ouverte » de la cloche n'est pas suivi (seul « tour en cours » la désactive).
