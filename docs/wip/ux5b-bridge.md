# UX5b bridge

FAIT : (1) `neighbour` dans `get_diplomacy` (CampaignState::neighbour_factions) + tri « Voisinage » ; (2) pastilles de
déblocage de `tech_tree_view.gd` avec icône (`unlocks.units/buildings` étaient déjà exposés ; repli sur ⚔/⛫).
NON FAIT : (3) `embarked` : sim-campaign n'a aucun état « en mer » (traversées résolues dans le tour, ADR 0167 :
l'armée termine toujours dans un port, `ArmyPosition` = Field|Settlement). `false` est donc exact aujourd'hui ;
afficher « embarqué » demande de créer un état de voyage (choix de conception) — laissé en l'état.
Tests : ux5_d_test, ux5_t_test, ux5_u_test OK ; cargo clippy + test -p godot-bridge OK.
