# bm-uileft3 (restes d'interface de campagne)
Base : le journal du HUD avait déjà le filtre par genre et le bloc « Monde » (hud/journal.rs), le clic sur le trésor ouvrait déjà l'Économie, et la couche « Routes commerciales » + la case « Signes » existaient dans le menu Filtres (hud/layers.rs, minimap.rs). Rien à refaire pour ces points.
Fait :
- Légende du mode Diplomatie : pastilles aux couleurs de position (`diplomacy_color`) et symbole de relation en daltonien (hud/help.rs `stance_symbol`).
- Survol : infobulle des couloirs maritimes (`nearest_lane` / `lane_tooltip`) et des routes commerciales (`nearest_route` / `route_tooltip`, la route gagne sur le couloir) immédiate, sans le délai de province (hud/hover.rs `LaneHover`, dump `hover_text.lane`).
- `feudal` dans `data/map/faction_borders.json` `modes` (+ enum du schéma) ; étape 2 du guide féodal = filtre Féodalité choisi (`MapMode::Feudal`), plus manuelle (panels/feudal/guide.rs).
Restes : le bouton « Routes » n'est pas dans la rangée des modes de la minicarte (Godot) mais dans le menu Filtres (déjà le choix de Godot MF1) ; cadre pulsant du guide.
