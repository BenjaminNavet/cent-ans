# bm/menu — front end Bevy

État : menu principal, choix de faction, sauvegarde / chargement, pause : faits et prouvés par script
(`client/src/menu/proof_new_game_save_load.txt`, lancer avec `client/run_bg.sh --state MainMenu --script …`).

Reste (Godot) : décor 3D `menu_backdrop_3d.gd` (ici carte illustrée 2D), `faction_map_picker.gd` (carte cliquable + filtres
royaume / rang), portrait du souverain et chiffres clés des cartes, graine, `battle_prologue_invite.gd`, `loading_screen.gd`,
`intro_cards.gd` plein écran (ici file de modales), écrans Batailles historiques / démos / rejeux / personnalisée
(`custom_battle_screen.gd` : le bouton ouvre `AppState::Battle` sans paramètres), Aide (F1) du menu pause,
sauvegarde automatique tous les N tours (`save_slots.gd autosave`, à brancher par le HUD campagne), vignettes de sauvegarde.
