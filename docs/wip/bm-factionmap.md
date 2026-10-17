# bm-factionmap — carte d'Europe de l'écran de choix de faction

Défaut joueur : « pas de carte d'Europe quand je choisis une autre faction ».

Cause : la carte vivait seulement dans l'onglet « Toutes les factions », sous 65 pastilles de royaume
sans défilement ; les volets d'onglet ne bornaient pas leur hauteur, donc la carte sortait de l'écran
(et n'existait pas du tout dans « Départs recommandés »). Le choix d'une faction ne changeait que la fiche.

Correctif (menu/faction_select.rs, menu/faction_map.rs) :
- carte d'Europe dans un panneau permanent entre les onglets (400 px) et la fiche (360 px), visible dans tous les onglets ;
- onglet « Toutes les factions » : filtres (pastilles de royaume dans une zone défilante de 84 px) + liste défilante bornée ;
- volets d'onglet `flex_grow 1 / min_height 0`, listes de cartes bornées ;
- faction choisie : contour doré (existant) + anneau doré posé sur son centre (`MapRing`, `frame_choice`), repli sur la bannière pour les sans-terre.
- dump `menu.faction_map.map` (taille du nœud, visibilité, volets actifs, pixels de terre, anneau).

Preuve : script France / Angleterre / Bourgogne via onglet recommandés et « tout » : nœud 488x424 visible, anneau déplacé à chaque choix. Test `the_ring_frames_the_chosen_faction`.

Restes : pas de zoom/cadrage rogné de la carte (Godot non plus) ; « Croisés » est une faction sans terre, listée seulement dans « Toutes les factions » (le pitch recommandé n'en a que 3).
