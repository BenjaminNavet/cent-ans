# UI/UX artist — état des lieux (09/10, lecture seule)

## 1. État actuel
- `game/scripts/ui/` : 108 scripts, 23 341 lignes (plus gros : `rich_tooltip.gd`, `encyclopedia.gd`, `faction_select.gd`). HUD bataille `battle/battle_hud.gd`, panneau province et HUD carte dans `map/`.
- Thème `game/scenes/ui/parchment_theme.tres`, chargé par 48 fichiers, pas global dans `project.godot`.
- `UiType` 26/20/17/14 à 900 px (ADR 0097, bible § 12) ; `UiLayout` 6 zones ; `UiMotion`, `SceneFader`.
- Infobulles en sections + chaîne Alt (ADR 0109) ; file de modales, nouvelles triées (A6-L6).
- Polices IM Fell English, EB Garamond, Noto (symboles).
- Kit enluminé 9-slice, sceaux de cire, ~80 icônes game-icons, `entity_icons.json` / `icons_ink.json`.
- Accessibilité : daltonien, réduire animations, contraste, taille UI et texte.
- Tests : `po_ui_test` (C1-C3), `ib_*`, `p2c_ui_test`, `vn_ui_720_test`.

## 2. Forces
- Identité homogène (parchemin, encres, lettrines, sceaux), discipline DA.
- Gouvernance rare : ADR, bible, contrôles automatiques tailles/chevauchements.
- Infobulles de niveau AAA (prévision avant→après, prérequis, chaînes).
- Surcharge de notifications bien traitée.
- Briques d'accessibilité présentes.

## 3. Faiblesses
**Cohérence** : 98 `add_theme_font_size_override` ; taille figée 12 px (`faction_panel.tscn:146`) ; ~1 124 `Color(...)` littéraux et 3 sources de style (thème, `hud_style.gd`, `front_end_style.gd`) ; 57 `draw_string` hors thème ; Tech, Diplo, Cour, Fiche, Sauvegarde hors `UiLayout`.
**720p** : liste de garnison coupée ; ~10 médaillons dorés quasi identiques dans la barre du haut ; saison affichée deux fois ; `Caption` ≈ 12,6 px (sous le plancher) ; bande d'ost déborde de 10 px.
**Bataille** : écran chargé (journal ouvert, barre d'ordres + cartes), icônes d'état minuscules, rapport de forces = simple barre bleu/rouge.
**Accessibilité** : daltonien branché dans 4 fichiers seulement, absent en bataille ; bon/mauvais en vert/rouge seul dans les infobulles.
**Finition** : nombres non localisés (« 2.1 », « +0.25 ») ; vedette « +5 Piété » sans icône ; surprime absente du budget ; même pictogramme pour tous les incidents ; double son de clic ; verdict IB6 en attente.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| 1 | Plancher de texte 720p (`Caption` ≥ 15, test C3 à 1280×720) | Fort | S | 0 | QA |
| 2 | Nombres en français via `FrText.num()` | Moyen | S | 0 | — |
| 3 | Panneau province 720p : en-tête repliable, défilement | Fort | M | 0 | gameplay |
| 4 | Daltonisme en bataille et infobulles (hachures, ▲/▼ + libellé, Okabe-Ito) | Fort (~8 %) | M | 0 | bataille, DA |
| 5 | Barre du haut groupée (Gouverner/Guerre/Savoir), silhouettes distinctes, saison unique | Fort | M | 0 | DA, audio |
| 6 | HUD bataille allégé (journal en pastille, ordres fusionnés, icônes 20 px) | Fort | M | 0 | bataille |
| 7 | Finir migration P2f vers `UiType`, thème global | Moyen | M | 0 | dev |
| 8 | Jetons de couleur nommés à la place des `Color()` | Moyen | L | 0 | dev, DA |
| 9 | `draw_string` suivant `UiType`, cadran de la cloche refait | Moyen | M | 0 | DA |
| 10 | Pictogrammes manquants (incidents, vedettes, nouvelles unités) | Moyen | M | 0-5 $ | illustrateur |
| 11 | Panneaux restants dans `UiLayout` | Moyen | M | 0 | dev |
| 12 | Trancher IB6 et le son de clic | Faible | S | 0 | producer, audio |

Ordre : 1, 2, 12 → 3, 5, 4 → 6 ; 7-9 en lots mécaniques.
