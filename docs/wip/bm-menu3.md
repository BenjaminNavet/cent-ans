# bm/menu3 — restes du front end

Fait : invitation au didacticiel, technologies / hold / nom de composition / miniatures et infobulles (bataille personnalisée), démos de siège montées sur une campagne jetable (province, landmark, engins, assaillant), bouton Rejeux (`OpenReplays`), Échap via KeyMap, F6 / F9, vérification sauvegarde -> chargement (condensé d'état).

Reste :
- `battle_prologue_invite.gd` (invite du didacticiel de bataille, Oui / Non / Ne plus demander) : non portée, c'est une autre invite.
- Après une bataille lancée du menu, `flow::battles` renvoie vers `AppState::Campaign` (et non le menu) : hors de mon dossier.
- Décor 3D du menu, carte cliquable des factions, portraits / chiffres clés, graine, `loading_screen.gd`, Aide F1 du pause, sauvegarde automatique, vignettes (voir bm-menu.md).
- Entrée (confirmation) n'est pas une action de KeyMap.
