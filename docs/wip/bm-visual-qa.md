# bm-visual-qa : revue visuelle du client Bevy (lot vqa)

Méthode : `run_bg.sh --offscreen --script` (captures 1280x800, `UiScale` auto 0,9), plus la sonde `ui_kit.layout` du dump. 13 captures utilisées sur 15.

## Corrigé dans `client/src/ui/kit/`
- **Cause racine du texte qui déborde du cadre** (didacticiel, tous les panneaux, boutons, onglets) : Bevy 0.20 dessine un `ImageNode` dans la boîte de CONTENU (`VisualBox::ContentBox` par défaut). Le cadre peint était donc rétréci du padding du nœud (~30 px par côté sur un panneau) et le texte sortait du cadre, à gauche comme à droite. `UiTheme::sliced` et `UiTheme::image` utilisent maintenant `VisualBox::PaddingBox`. Vérifié : didacticiel et arbre féodal, texte à l'intérieur du cadre.
- Modales : largeur max 94 %, hauteur max 88 % de la fenêtre ; `panel_sized` : largeur max 96 %.
- Réglage `interface/text_size` : `UiTheme::text_scale` (suit `Settings`) multiplie toutes les polices du kit. Vaut pour les textes construits après le changement ; les panneaux déjà ouverts gardent leur taille jusqu'à leur reconstruction.
- Échelle d'interface (`UiScale`) : déjà portée par `settings::apply_ui_scale` ; la sonde travaille en pixels logiques.

## Défauts restants, par écran (dossier propriétaire)
- **Accueil** : propre. Sous-titre « France — Printemps 1339 » du bouton Continuer serré contre l'entrée suivante (`menu/`).
- **Choix de faction** (`menu/`) : (1) bouton « Commencer Commencer — France » (libellé doublé) ; (2) le cartouche « Cent Ans » du menu principal reste visible derrière l'écran et se mêle au titre « Printemps 1337 » en haut à droite .
- **Réglages** (`settings/`) : la modale est correcte (cadre, onglets, défilement). Dernière ligne de l'onglet Affichage coupée par le bas de la liste (défilement ; ok) ; la modale ne s'assombrit pas le fond de l'écran de faction.
- **Campagne HUD** (`campaign/hud/`) : bouton « Codex » du bas en retrait sur une 2e ligne et collé au bord droit ; barre de boutons du bas déborde à droite quand le panneau latéral est ouvert (« Re… » coupé par le panneau). Noms « Londres », « Lübeck » masqués par les boutons de carte (Politique, Terrain…).
- **Campagne, panneaux** : (1) F3 puis F4 ouvrent Diplomatie ET Techniques en même temps, les touches contournent `panel_stack::displaces` (`campaign/panels/`, ordre de pile à vérifier pour les raccourcis) ; (2) la carte du didacticiel (`UiTier::Tutorial`) passe au-dessus des panneaux et masque la colonne gauche de Diplomatie (`campaign/progress/tutorial.rs`) ; (3) l'arbre féodal dépasse le bas de la fenêtre (cadre inférieur hors écran, `campaign/panels/feudal/`).
- **Bataille HUD** (`battle/hud/`) : le panneau « Ordres du chef » (y 586-723) et « Formations » chevauchent la bande de cartes (y 692) ; la minicarte masque les 3 derniers boutons de vitesse et recouvre la bande ; sol de la bataille noir en capture hors écran (`battle/scene`, terrain non rendu en `--offscreen` ou en retard) ; fanions d'unités flottant sans modèle.
- **Codex** (`codex/`) : écran VIDE, que ce soit par `--state Codex` ou par le bouton du menu (fond brun seul). Le dump `codex` annonce 540 fiches, 579 widgets du kit, mais la sonde `ui_kit.layout` donne un rectangle (0,0,0,0) à tous les cadres (Search, panel, listes), même après 900 images et avec une requête réduite à 38 fiches : la mise en page des descendants de la racine `Codex` ne se calcule pas. Piste : `tweak()` / `Display` des barres `FamilyBar` / `RulesBar` ou la racine `GlobalZIndex(400)` sans `UiTier` ; non corrigé (hors dossier).
- **Modale du briefing** : plus de débordement en largeur à la sonde (aucune modale active en bataille de test) ; recontrôle visuel à faire avec une vraie pré-bataille.
- **Infobulles** : non capturées (pas de pointeur en script) ; largeur max 384 px déjà en place.

## Reste dans ui/
- Survol réel des infobulles et reconstruction à chaud des textes au changement de `text_size`.
- `ui/fa_ui.gd` (boutons/sceaux FA, ADR 0164) : non porté (variantes de boutons sceau/ruban).
