# bm-chronicle — panneau Chronique (Bevy)

État : `game_api::chronicle` (décisions en attente, journal d'un tour classé par pertinence du cœur, « Monde », lettres filtrées par mode `interface/news_filter`, avis de décisions expirées) testé ; panneau `OpenPanel::Chronicle` / F5 à 3 onglets (Décisions, Journal avec filtre de genre et pli « Monde », Lettres), historique tenu par `ChronicleLog` (nourri par `TurnAdvanced`), section de dump `panel_chronicle`.
Prochaine étape : sceaux de cire avec écu, liens Codex, miniatures d'événements, clic d'une lettre = caméra sur la province, file `ModalQueue` pour l'ouverture automatique en début de tour, bouton « Chronique (n) » de la barre (lot hud).

## campui7 (vague 8)
Fait : sceaux de cire des lettres avec l’écu de la faction (`wax_seal`, `heraldry/<faction>.png` sur disque parchemin dans l’anneau de cire) ; mots du Codex cliquables dans le texte des décisions (`codex::text::format_auto`, survol = bulle, clic = `OpenCodex`) ; déplier une lettre = caméra sur sa province, sinon fiche diplomatique de sa faction (`letter_action`, port de `_on_news_activated`, `DiplomacyUi::show`) ; choix d’une décision = son `ui_order` / `ui_order_refused`.
Reste : miniatures d’événements (asset absent), file `ModalQueue` d’ouverture automatique, bouton « Chronique (n) » de la barre (lot hud), cire irrégulière (Godot `wax_points` : disque lisse ici).
