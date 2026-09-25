# Bulles partout — Codex « Le jeu » et verrouillage façon BG3

Session historien, 25 septembre 2026. Suite de `2026-09-23-histoire-et-savoir.md`.

## Objectif
Le joueur curieux doit pouvoir, depuis n'importe quel texte du jeu, survoler un mot-clé, lire une
bulle, **la verrouiller avec T**, puis survoler un mot-clé de cette bulle pour en ouvrir une autre,
et ainsi de suite (Baldur's Gate 3). Les bulles parlent d'histoire **et** de mécaniques :
bâtiments, unités, navires, techniques, règles de campagne et de bataille.

## Comportement attendu (interaction)
1. Survol d'un mot-clé (lien rubriqué) 0,35 s → bulle.
2. **T** (`codex_pin_tooltip`) : verrouille la bulle ou l'infobulle visible la plus récente —
   bulle du Codex non épinglée, infobulle riche (F2), ou infobulle simple (`tooltip_text`) de
   n'importe quel contrôle, convertie en bulle interactive avec auto-liens.
3. Une bulle verrouillée ne se ferme pas quand la souris la quitte ; on y entre, on survole ses
   mots-clés, T verrouille la suivante. Clic droit reste un raccourci d'épinglage.
4. Échap ferme tout ; clic sur une bulle → fiche complète.
5. Chaque infobulle affiche en pied « T : maintenir ouverte ».

## Contenu
- Nouvelles catégories : `mecanique` (famille « Le jeu »), `batiment`, `unite`, `technique`
  (famille « Armées, navires et bâtiments »).
- Nouveau champ `gameplay` (« En jeu ») : comment le jeu modélise le sujet, chiffres tirés du
  code/données (jamais inventés). Fiche : encadré « En jeu » ; bulle : résumé + 1re phrase.
- `entity` couvre aussi `edict_`, `order_`, `ship_` ; le titre de l'infobulle riche d'une entité
  liée devient un lien vers sa fiche.
- Toute fiche mécanique garde une partie historique (pourquoi c'est ainsi au XIVe siècle).

## Lots
| Lot | Contenu |
|---|---|
| B1 Infra | T universel, pied « T : maintenir », `gameplay` dans bulle + fenêtre, titres d'infobulles liés via `entity`, conversion des infobulles simples les plus vues en riches auto-liées |
| B2 Mécaniques campagne | ~25 fiches `mecanique` : économie, ordre, population, mouvement, ravitaillement, météo, saisons, diplomatie, agents, colonies, vision, relief… |
| B3 Mécaniques bataille et siège | ~25 fiches : moral, déroute, formations, ordres, terrain, relief, murailles, incendies, engins, batailles navales… |
| B4 Bâtiments et ressources | une fiche par bâtiment (31) et ressource (11) |
| B5 Unités, navires, techniques | une fiche par unité (27), navire (4), techniques clés |
| B6 Audit historique | vérification des contenus ajoutés depuis le 24/09 (naval, sièges, bâtiments BR1, colonies, repères, relief) |
