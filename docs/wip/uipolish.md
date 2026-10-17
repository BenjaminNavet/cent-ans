# uipolish — défauts d'interface du testeur visuel

État : les 9 points sont traités dans le code (voir la liste ci-dessous) ; fmt, clippy et `cargo test` passent. Captures de contrôle faites (`out/uip/`, 7 sur 8) : diplomatie, technologies, Codex, armée, pause de bataille OK ; Entrée vérifiée par dump. Terminé.

1. Diplomatie : 5 raisons d'opinion au plus (le reste dans l'info-bulle du total), en-tête non écrasé (`flex_shrink` 0, 55 %).
2. Technologies : cases à cocher avec contour, nœuds élargis (252 px, colonnes 288), titres plus longs.
3. Une seule bulle : l'indice d'ordre (`HoverHint`) se tait quand une bulle kit est ouverte ou que le pointeur est sur l'interface.
4. Panneau d'armée : la zone `Bottom` laisse 34 px pour la barre « l'ost de… ».
5. Échap en bataille : menu de pause de bataille (`menu/battle_pause.rs`).
6. Codex : fond cuir (plus clair que le noir) ; ouverture sur la première fiche lue.
7. Entrée dans une confirmation (`modal_enter`) la confirme.
8. Météo « Pluie » : voulu à Crécy (`crecy.json` : pluie au début, ciel dégagé à 50 s ; le dump `battle_weather` donne le ciel final « clear », la HUD la météo simulée). Rien changé.
9. BRP : clics souris sans effet sur l'interface sous `--offscreen` (caméras sur image) ; voir le rapport.
