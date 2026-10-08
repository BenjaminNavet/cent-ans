# DN V1 — Audit des assets 2D de l'interface (08/10)

Lecture seule : comptages par `ls`/scripts sur `game/assets/`, `data/`, `tools/`. Aucun Godot, aucune capture.
Registre visé : enluminure (bible DA § 1, 5, 8, 12, 13) ; génération locale Z-Image Turbo (ADR 0190).
Réserve : la bible DA n'a pas été relue en détail pour cet audit ; les briefs ci-dessous sont à raccorder à sa charte.

## 1. Inventaire par famille

| Famille | Existant (nombre, source) | Faible / placeholder |
|---|---|---|
| Cadres 9-patch | `ui/illumination/` : 14 PNG + `kit.json` (panel, panel_illuminated, top_bar, tooltip, inset, tabs, 4 états de bouton, slider_track, barres or/azur). Procéduraux (`ui_illumination.py`). 4 redessins `nb/` (panel, panel_illuminated, tooltip, top_bar) par Gemini (NB1, `data/art/ui_ornaments.yaml`) | Barres, onglets, boutons, slider, inset : filets de 1 à 9 px sans ornement ; un seul cadre « fenêtre » pour tout |
| Boutons | 4 états procéduraux (`button_*.png`), thème `scenes/ui/parchment_theme.tres` (StyleBoxTexture + StyleBoxFlat focus) | Un seul bouton ; pas de bouton principal (fin de tour, valider) ni de bouton d'icône rond |
| Icônes d'action et HUD | `icons/ink/` : 159 fichiers (198 ids dans l'index : act_, hud_, battle_*, gauge_, class_, branch_, cat_), PNG encre 128 px, générés (`ink-icons`) | `cat_default` est le repli ; ids « tout en SVG » encore présents sous `icons/` |
| Médaillons de HUD | `ui/medallions/` : 15 PNG enluminés générés (agents, chronicle, codex, court, diplomacy, end_turn, filters, form_army, journal, legend, menu, objectives, recruit, research, technologies) | Pas de médaillon pour les stances, la trésorerie, le ravitaillement, les alertes |
| Icônes d'entité | `icons/entity/` : 168 miniatures peintes 128 px cadrées or/azur (92 unit, 90 tech*, 60 skill*, 60 bld*, 20 res, 14 diet ; index = 168 ids) ; générées (planches + découpe, `entity-icons`) | Toutes les unités, bâtiments, techs, compétences, ressources, régimes de `data/` en ont une |
| Icônes libres | `icons/*.svg` : 494 fichiers (game-icons.net, CC BY 3.0 : lorc 176, delapouite 128, skoll, caro…) + `icons.json` : traits (59 ids, tous couverts), gauges, hud, trait_category_*, building_category_* | Monochrome sépia, hétérogène avec les miniatures peintes ; les traits (59) n'existent qu'en SVG |
| Sceaux | `ui/fa/seals/` : 2 cires (Met Open Access, CC0) ; sceau de général dessiné en code (`HudStyle.draw_wax_seal`, `general_seal.gd`) ; `ui/fa/materials/` : 2 plaques cuir/laiton (ambientCG CC0) | Pas de sceaux de faction ni de matrices par type (alliance, traité, vassal) |
| Curseurs | `ui/cursors/` : 6 PNG (move, melee, ranged, ranged_blocked, siege, forbidden), bataille seulement | Aucun curseur de campagne (flèche, main, déplacer, attaquer, assiéger, embarquer) ni de menu |
| Armoiries | `heraldry/` : 177 `fac_*` + 535 fichiers bannière (banner/pennon/standard par faction) + `houses.json` ; dessinées/générées par code (`heraldry`, `banners`) | Les blasons d'écus de ~130 petites factions sont schématiques ; blasons des miniatures de faction inventés (wip mflux) |
| Illustrations Codex / Encyclopédie | `illustrations/` : 375 JPG (177 fac, 84 cdx, 45 tech, 39 unit, 30 bld), mflux local ou OpenRouter, charte « miniature » | 392 des 476 fiches `data/codex/cdx_*.json` n'ont pas d'image (voir § 3) |
| Événements | `events/` : 117 JPG sur 153 `data/events/*.json` | 36 événements sans image |
| Portraits | `portraits/` : 245 `chr_*` + `aged/` (vieillis) + `archetypes/` (burgher, etc.) ; 600 fichiers au total | 253 personnages en données, 245 portraits : ~8 manquants (cf. `portrait_loader.gd`) |
| Écrans de chargement | `art/loading/` : 11 `ld_*` (Crécy, Poitiers, Calais, Paris…), enluminures du domaine public (Wikimedia Commons recadrées), catalogue `data/ui/illustrations.json` | Aucune scène de chargement pour les batailles hors liste, ni pour l'Orient/Baltique |
| Fins, vignettes | `art/endings/` : 4 (victoire/défaite de bataille et de campagne) ; `art/vignettes/` : 16 `vg_*` (plague, siege, war…) ; domaine public | Une seule fin par issue, sans variante par faction ni par conclusion (couronne, exil, ruine) |
| Fonds de menu | `ui/menu_map.jpg` + `menu_map.json`, `boot_splash.png` (procédural `menu-art`) ; fond 3D `menu_backdrop_3d.gd` | Menu et splash sans enluminure propre |
| Ornements | `ui/fa/` (FA5) ; `data/art/ui_ornaments.yaml` : decor `initial_dragon`, `fleuron_divider`, `drollery_hare` définis en YAML | Lettrines : `lettrine.gd` dessinée en code ; aucune lettrine générée posée ; séparateurs et drôleries non produits dans `game/assets/` |
| Textes de placeholder | Glyphes Unicode dans le code (✠ ✔ ⚔ ⚜ ♔ ⚑ ⚒ ✉ ♨ ❦ ☰ ★ ▸) : `codex_hub.gd`, `encounter_window.gd`, `encyclopedia.gd`, `tutorial.gd`, `accessibility.gd`, `season_report.gd`, `alerts.gd`, `crusade_section.gd`, `feudal_section.gd`, `ransom_panel.gd` | Pas d'emoji couleur ; glyphes de police en place d'icônes |

(* index entity : les comptes unit/tech/skill/bld sont ceux de fichiers PNG, le total de l'index est 168.)

## 2. Clés attendues par le code sans image

- `IconLibrary` (`icon_library.gd`) : ordre miniature d'entité > encre > SVG > repli de catégorie (`cat_*`) > `cat_default`. Ids sans icône propre (repli seulement) : 9 religions (`rel_*`), 4 ordres de chevalerie (`ord_*`), 6 édits (`edict_*`). Les `battle_ability_*` et `battle_order_*` sont couverts à l'encre.
- Préfixes dynamiques (`gauge_`, `hud_season_`) : construits à l'exécution ; tous les ids trouvés existent (10 gauges, 4 saisons).
- `encyclopedia.gd` (`ILLUSTRATIONS_DIR`) : chemin `illustrations/<id>.jpg` ; sans fichier, la fiche s'affiche sans image (36 événements, 392 fiches Codex, 8 portraits).
- `battle_cursor.gd` : `ui/cursors/<nom>.png` avec `ResourceLoader.exists` (repli sur la flèche système) ; aucun curseur campagne.
- `data/ui/illustrations.json` : 57 chemins `File:…` sont des références Commons (sources), non des fichiers ; les 11 + 16 + 4 images référencées existent.
- `data/ui/front_end.json` : bannières `fac_france_banner.png`, `fac_burgundy_banner.png`, `oriflamme.png`, `fac_genoa_banner.png` sont référencées en relatif (résolues dans `heraldry/banners/`, présentes).
- Aucun `preload()` d'image UI sans fichier détecté ; le thème n'a pas de `Icon` par défaut.

## 3. Outils de génération et commandes

Tous passent par `uv run --project tools cent-ans assets <cmd>`, option `--local` (mflux, Z-Image Turbo q8 dans `~/models/mflux/z-image-turbo-q8`, `CENT_ANS_MFLUX_MODEL`) ; `--dry-run` liste prompts et coût ; graine dérivée de l'id.
- `event-art --local [--limit N]` : miniatures d'événements 16:9 → `game/assets/events/` (`event_art.py`).
- `codex-art --local [--category lieu,bataille] [--limit N]` : 16:9 → `illustrations/cdx_*` (`codex_art.py`) ; catégories par défaut sauf personnages et plantes.
- `illustrations --local` : factions, unités, bâtiments, techs (`data/ui/illustrations.json`/`entry_art.py`).
- `portraits --local`, `portrait-archetypes --local` : 3:4 → `portraits/` (`portraits.py`, `portrait_archetypes.py`).
- `entity-icons --local` : planches 1:1 + découpe, cadre or/azur → `icons/entity/` (`entity_icons.py`, catalogue `data/ui/entity_icons.json`).
- `ink-icons --local` : icônes d'action et médaillons → `icons/ink/`, `ui/medallions/` (`ink_icons.py`, `data/ui/icons_ink.json`).
- `art-plates --local` : loading/ending 16:9, vignettes 21:9 (`art_plates.py`, `data/ui/illustrations.json`).
- `ui-ornaments --local [--only id] [--sheet planche.png] [--install]` : kit enluminé depuis `data/art/ui_ornaments.yaml` (pièces `panel`, `panel_illuminated`, `top_bar`, `tooltip` + `decor`).
- `horizon-panoramas --local`, `materials --local`, `ground-materials generate` : panoramas 21:9, textures 1:1.
- Procéduraux sans image : `ui-illumination` (kit), `menu-art` (fond de menu), `heraldry`, `banners`, `unit-emblems`, `icons` (SVG game-icons).
- Hors mflux : Qwen-Image-Edit (30 min/image) pour figurines (`docs/pipeline-assets-3d.md`), Gemini/NB2 via OpenRouter en dernier recours. Contraintes DN : un seul générateur à la fois (48 Go).
- Limites connues de Z-Image : texte et blasons non respectés (poser les vrais après coup), pas de cohérence multi-images.

## 4. Liste priorisée des assets manquants ou faibles

Priorité P1 (visible en permanence ou à chaque partie), P2 (fréquent), P3 (confort). Chemins cibles sous `game/assets/`.

### P1 — cadres, boutons, HUD
1. `ui/illumination/nb/button_normal.png`, `button_hover.png`, `button_pressed.png`, `button_disabled.png` : bouton de parchemin à filet d'or, coins en feuille de lierre, 4 états alignés (`ui-ornaments`, 9-patch).
2. `ui/illumination/nb/button_primary.png` (+ hover, pressed) : bouton principal (fin de tour, valider) en azur et or, plus large, sceau de cire discret.
3. `ui/illumination/nb/tab_selected.png`, `tab_unselected.png` : onglets-signets de manuscrit, sélectionné doré, repos vélin.
4. `ui/illumination/nb/inset.png` : cadre intérieur de champ de lecture, double filet encre, liseré rubrique.
5. `ui/illumination/nb/slider_track.png` + `slider_grabber.png` : rail filet d'or ; poignée en cabochon de cire.
6. `ui/illumination/nb/bar_gold_h.png`, `bar_azure_h.png`, `bar_gold_v.png` : barres de séparation avec fleurons discrets.
7. `ui/illumination/nb/frame_modal.png` : cadre de fenêtre modale (événement, rencontre, diplomatie), marges 44 px, drôlerie en bas, distinct de `panel`.
8. `ui/illumination/nb/frame_portrait.png` : cadre de portrait de personnage (`portrait_frame.gd`), or et azur, écu en pied.
9. `ui/illumination/nb/frame_icon_slot.png` : alvéole carrée 64 px sous les icônes d'entité et de ressource.
10. `ui/illumination/nb/progress_bar_frame.png` et `progress_fill.png` : barre de chargement/recherche en filet d'or et encre rouge.
11. `ui/illumination/nb/scroll_bar.png` : rail et poignée de défilement de parchemin.
12. `ui/illumination/nb/tooltip_arrow.png`, `tooltip_title_band.png` : bandeau titre d'infobulle (`tooltip_view.gd`).
13. `ui/medallions/stances.png`, `treasury.png`, `supply.png`, `alerts.png`, `siege.png` : médaillons enluminés manquants du HUD (mêmes dimensions que `agents.png`).

### P1 — curseurs et sceaux
14. `ui/cursors_campaign/arrow.png`, `hand.png`, `move.png`, `attack.png`, `siege.png`, `embark.png`, `forbidden.png` : curseurs de campagne 32 px, gantelet et plume ; chemin lu par un futur `campaign_cursor.gd`.
15. `ui/seals/seal_alliance.png`, `seal_treaty.png`, `seal_vassalage.png`, `seal_marriage.png`, `seal_war.png` : matrices de sceau de cire pour les écrans de diplomatie.
16. `ui/seals/seal_wax_red.png`, `seal_wax_green.png`, `seal_wax_blue.png` : cires vides teintables sans blason (le blason est posé par code).

### P1 — écrans-clés
17. `art/menu/menu_bg_illuminated.jpg` : fond de menu principal en enluminure (carte des Cent Ans et chevaliers), 16:9.
18. `boot_splash.png` : refonte du splash en page enluminée (titre en lettrines, sans texte généré).
19. `art/endings/end_campaign_victory_{france,england,burgundy}.jpg` : couronnement par camp, 16:9.
20. `art/endings/end_campaign_defeat_exile.jpg`, `end_campaign_defeat_ruin.jpg` : exil, ruine de la maison.
21. `art/endings/end_battle_draw.jpg`, `end_battle_retreat.jpg` : issues manquantes de bataille.

### P2 — contenu éditorial
22. 36 miniatures d'événements manquantes → `events/<id>.jpg` : lancer `event-art --local` ; commencer par les événements historiques (mort d'Ivan Kalita, de Gediminas, d'Andronic III, couronnement de Dušan, chute de Tlemcen, siège de Caffa, prise d'Algésiras, séisme de Gallipoli) puis les génériques (feu de grange, marchand détroussé, pont effondré).
23. Codex lieux (28 sans image) → `illustrations/cdx_<lieu>.jpg` : villes (Reims, Montpellier, Cassel…), vue au sol, dessin à l'encre colorée.
24. Codex batailles et guerres (16 + 16) → `illustrations/cdx_<id>.jpg` : mêlée en miniature, ex. `cdx_combat_des_trente`, `cdx_cassel`.
25. Codex unités (34) → `illustrations/cdx_<unité>.jpg` : fiches « Franc-archer », « Pavois », « Compagnies d'ordonnance » ; réutiliser l'image `unit_*` correspondante quand elle existe.
26. Codex bâtiments (25), techniques (29), institutions (15), religions (10), économie (11), médecine (7) : lot `codex-art --local --category batiment,technique,institution,religion,economie,medecine` (~97 images).
27. Codex mécaniques de jeu (63, `cdx_jeu_*`) → `illustrations/cdx_jeu_<x>.jpg` : bandeau générique par thème (impôt, recrutement, terrain, vision, flancs, population) plutôt qu'une image par fiche.
28. Codex personnages (87 sans image) : réutiliser `portraits/chr_*` ; rien à générer (vérifier le lien dans `encyclopedia.gd`).
29. Portraits manquants (~8) → `portraits/chr_<id>.jpg` : lister par comparaison `data/characters/` ↔ `portraits/` puis `portraits --local`.
30. Plantes (20) : planche botanique à l'encre colorée → `illustrations/cdx_<plante>.jpg`.

### P2 — icônes
31. `icons/entity/rel_*.png` × 9 : une icône peinte par religion (croix latine, orthodoxe, croissant, étoile, lollard, hussite, païen, arménien).
32. `icons/entity/ord_*.png` × 4 : jarretière, toison d'or, étoile, compagnie de la cour.
33. `icons/entity/edict_*.png` × 6 : paix de Dieu, carême strict, levée de milice, franchises de marché, aide féodale, aucun.
34. `icons/entity/trait_*.png` × 59 : traits en miniature peinte à la place du SVG sépia (lot de 60 en planches 4×4).
35. `icons/entity/gauge_*.png` × 10 : jauges (moral, santé, ravitaillement, troubles, richesse…).
36. `icons/entity/class_*.png` × 4 : noblesse, clergé, bourgeois, paysans.
37. `icons/ink/glyph_cross.png`, `glyph_check.png`, `glyph_star.png`, `glyph_arrow.png`, `glyph_swords.png`, `glyph_crown.png` : remplaçants à l'encre des glyphes de police (✠ ✔ ★ ▸ ⚔ ♔) dans `codex_hub.gd`, `tutorial.gd`, `accessibility.gd`, `encounter_window.gd`.
38. `icons/ink/season_report_{lands,armies,works,world,table}.png` : remplacent les glyphes de `season_report.gd` (⚑ ⚔ ⚒ ✉ ♨).

### P3 — chargement, ornements, confort
39. `art/loading/ld_<bataille>.jpg` × 8 : Azincourt, Cocherel, Castillon, Formigny, Cassel, Rosebecque, Nicopolis, Tannenberg, 16:9, scène d'époque.
40. `art/loading/ld_orient.jpg`, `ld_baltique.jpg`, `ld_mediterranee.jpg` : écrans pour les factions de l'Est, du Nord et de Méditerranée.
41. `art/vignettes/vg_{crusade,heresy,tournament,embassy,harvest,shipwreck,truce,coronation}.jpg` : 8 vignettes d'événements (21:9) à ajouter à `illustrations.json`.
42. `ui/illumination/nb/initial_*.png` × 12 : lettrines enluminées (A, C, D, E, I, L, M, N, P, R, S, V), 128 px, pour `lettrine.gd`.
43. `ui/illumination/nb/fleuron_divider.png`, `fleuron_small.png` : séparateurs de section (YAML `fleuron_divider` produit).
44. `ui/illumination/nb/drollery_{hare,snail,knight,monk}.png` : drôleries de marge pour pied de fenêtre.
45. `ui/illumination/nb/corner_{tl,tr,bl,br}.png` : écoinçons séparés pour fenêtres redimensionnables.
46. `ui/illumination/nb/ribbon_banner.png` : bandeau-phylactère pour titres de fenêtre (`illuminated_title.gd`).
47. `ui/fa/seals/seal_{france,england,burgundy}_ring.png` : sceaux des trois camps jouables pour les lettres (`news_letters.gd`).
48. `ui/illumination/nb/parchment_tile.png` : texture de vélin tuilable 512 px pour fonds de listes et de lettres.
49. `ui/illumination/nb/ink_blot.png`, `wax_drip.png` : taches d'encre et gouttes de cire pour l'habillage des lettres.
50. `ui/illumination/nb/chain_link_frame.png` : cadre de liste d'armée (`army_strip.gd`) en maille et cuir, distinct du cadre d'enluminure.

## 5. Ordre d'exécution proposé (file mflux sérielle)
1. Cadres et boutons (1–13) : `ui-ornaments --local --only … --sheet` puis choix `selected` et `--install` ; le 9-patch exige symétrie et marges larges, donc 3 variantes par pièce et relecture humaine.
2. Fonds et fins (17–21) : `art-plates --local` après ajout des entrées à `data/ui/illustrations.json`.
3. Événements (22) puis Codex lieux/batailles/unités (23–25) : `event-art --local` puis `codex-art --local`, 1 min par image, ~190 images ≈ 3 h.
4. Icônes d'entité (31–36) : ajouter les groupes à `data/ui/entity_icons.json` puis `entity-icons --local`.
5. Curseurs, sceaux (14–16, 47) : petits PNG ; dessin procédural possible (pas d'IA) si le rendu est faible.
6. Le reste (P3) selon le temps machine ; priorité DN : campagne > bataille > UI.
