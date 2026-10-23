# 0753 — Focus clavier et manette du kit UI

Statut : accepté. Lot UX (juge iter-02, partie B, constat 3 : « clavier 4/10, manette 0/10 »).

## Contexte
Aucun menu ni panneau ne se parcourait sans souris : pas de focus, `Tab` pris par `campaign_next_idle`, aucun code manette. Les boutons du kit sont des `bevy_ui_widgets::Button` : la souris envoie `Activate`, que `on_activate` convertit en `ButtonPressed`.

## Décision
- `ui/kit/focus.rs` (`FocusPlugin`, ressource `UiFocus`, dump `ui_focus`) : un anneau de focus (`Outline` or pâle de 3 px) sur un bouton ou un curseur à la fois. `Tab` / `Maj+Tab` : ordre de lecture (lignes à 10 px près, puis gauche à droite, avec bouclage) ; flèches : voisin spatial (cône de 45°, pénalité latérale) ; `Entrée` / `Espace` : `Activate` sur le focus ; `Échap` : inchangé (menus, pile de panneaux) ; flèches gauche/droite sur un curseur : ±5 % de sa plage.
- **Portée** : seule la couche du dessus prend le focus (plus haut `GlobalZIndex` parmi les ancêtres du candidat). Une modale garde donc le focus ; un panneau central ou un menu aussi. Les boutons masqués (`display: none`, taille nulle, `InheritedVisibility` faux) ou désactivés sont exclus.
- **Quand les touches sont vivantes** : toujours dans les menus d'avant-partie, sous une modale et avec une fenêtre centrale ouverte ; sur le HUD nu de la campagne ou de la bataille, seulement après **F9** (amendé par l'ADR 0775 : F8 est la console, portée unique du HUD, focus initial sûr, stick gauche) (le Tab y garde `campaign_next_idle`, les flèches y gardent la caméra ; clic souris ou Échap rendent les touches). Les touches utilisées sont consommées (`ButtonInput::reset`) pour ne pas déclencher aussi un raccourci. Champ de texte actif : le focus se tait.
- **Manette** : croix directionnelle = flèches (et engage le focus), A (sud) = Entrée, B (est) = Échap. Pas de curseur libre ni de roues d'ordres avec la manette (hors lot) ; le focus directionnel + A/B suffit pour les menus, les modales, les panneaux de campagne et le bandeau Conseiller.
- Pas d'`InputFocus` de Bevy : sa dispatch `Entrée` déclencherait un second `Activate`. Notre ressource est la seule source.

## Conséquences
- `Tab` ne passe plus à l'armée suivante quand un panneau central ou une modale est ouvert (il navigue) ; sur la carte nue il garde son rôle. L'audit proposait `.`/`,` pour `campaign_next_idle` : non fait, la portée rend le conflit inutile.
- Limites : curseurs et champs de texte se parcourent mais seul le curseur se règle au clavier ; molette de liste (`scroll_list`) non pilotée par le focus (le focus ne fait pas défiler automatiquement vers un bouton hors de vue) ; manette non testée sur matériel (logique pure testée).
