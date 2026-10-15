# bm-chronicle — panneau Chronique (Bevy)

État : `game_api::chronicle` (décisions en attente, journal d'un tour classé par pertinence du cœur, « Monde », lettres filtrées par mode `interface/news_filter`, avis de décisions expirées) testé ; panneau `OpenPanel::Chronicle` / F5 à 3 onglets (Décisions, Journal avec filtre de genre et pli « Monde », Lettres), historique tenu par `ChronicleLog` (nourri par `TurnAdvanced`), section de dump `panel_chronicle`.
Prochaine étape : sceaux de cire avec écu, liens Codex, miniatures d'événements, clic d'une lettre = caméra sur la province, file `ModalQueue` pour l'ouverture automatique en début de tour, bouton « Chronique (n) » de la barre (lot hud).
