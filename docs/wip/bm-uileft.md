# bm-uileft : restes de l'interface (uicamp, uibattle, hudfix)

Branche `bm/uileft` (fusionne `bm/hudfix`).

## Fait
- Cartes fixes sur `FitZone` : tutoriel (Left), traditions et montre des hommes d'armes (Side).
- Bouton « Menu » : message propre `menu::pause::TogglePause` (plus l'arête Échap : un panneau ouvert reste ouvert).
- Journal d'événements en haut à gauche (`campaign/hud/journal.rs`), replié au départ, premier enfant de la colonne du conseil ; lignes de `ChronicleLog` (le filtre de genre et l'onglet « Monde » restent dans la fenêtre Chronique).
- Bataille : panneau des formations de groupe replié dès qu'aucun déploiement ne tourne (même déploiement sauté) ; journal de bataille replié au départ ; sous-titre du chroniqueur en bulle sous le journal (colonne gauche) au lieu du toast (`audio/advisor.rs`, campagne inchangée).
- Cloche désactivée tant qu'une fenêtre modale ou le menu pause est ouvert (`refresh_modal`) ; filet d'or entre les pastilles et la cloche ; minicarte à l'échelle de sa zone (`fit_ratio`, 0,18 × 0,28 de l'écran, 0,2 à 1).

## Restes
- Journal de campagne : pas de filtre de genre ni d'onglet « Monde » dans le widget (dans la fenêtre Chronique).
- Bulle du chroniqueur sans bouton « fermer ».
