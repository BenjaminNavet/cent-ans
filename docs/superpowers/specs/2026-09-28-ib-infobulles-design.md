# IB — Infobulles : mise en page en sections et chaînes de bulles à la touche

Date : 2026-09-28. Statut : spec écrite, en attente de relecture par le joueur.
Demande : comparer une infobulle de Baldur's Gate 3 (armure « Barkskin ») à celles du jeu et faire
mieux ; puis « pouvoir ouvrir plusieurs infobulles en maintenant une touche : une infobulle, puis
depuis un mot-clé une autre, etc. ».
Chantier : **IB**. Remplace le volet « infobulles riches » de P2c (P2c garde codex et encyclopédie).
ADR réservé : **0109** (touche de chaîne et deux niveaux de détail).

## 1. Constat

Le socle existe et il est riche :

- `RichTooltip` (`game/scripts/ui/rich_tooltip.gd`, F2) produit le BBCode des infobulles (unités,
  bâtiments, techniques, ressources, jauges, traits, compétences, classes) à partir de
  `GameCatalog` (`data/`) et des dictionnaires `live` de `CampaignSim` ; `make_panel` l'affiche en
  parchemin.
- `CodexBubbles` (`game/scripts/codex/codex_bubbles.gd`, H2 + B1) gère des bulles imbriquées :
  survol d'un mot-lien 0,35 s → bulle fille, pile de 6, épinglage par clic droit ou par T,
  épinglage des ancêtres, grâce de 0,4 s, Échap.
- `CodexText` lie automatiquement les alias des 234 fiches du Codex qui ont une `entity`.

Défauts, face à BG3 :

| # | BG3 | Cent Ans |
|---|---|---|
| D1 | En-tête distinct (bandeau, nom coloré par rareté, sous-titre, illustration) | Icône 28 px et titre en ligne, sous-titre accolé |
| D2 | Un chiffre vedette en écusson (« 12 Armour Class ») | Stats en ligne continue « Mêlée 8 · Tir 0 · … » |
| D3 | Sections séparées par des filets | Un seul `RichTextLabel`, lignes enchaînées |
| D4 | Un effet nommé par ligne | `_effects_block` : tous les effets sur une ligne, séparés par « · » |
| D5 | Valeur résultante (« porte la CA à 16 ») | Variation seule (« Armure +2 ») |
| D6 | Avertissement avec icône ⚠ | Ligne rouge « Indisponible : … » ; prérequis en « Requiert : X » sans état |
| D7 | Pied chiffré (poids, valeur) | Pied = rappel de touche ; coût noyé dans le corps |
| D8 | Mots-clés à infobulle propre, en chaîne | Chaîne possible mais pénible (voir § 3) |
| D9 | Infobulle courte, détail sur « Inspect » | Tout est toujours affiché (une unité dépasse 12 lignes) |
| D10 | — | Corps en 14 px (taille Caption de la bible § 12, pas Body) ; largeur fixe 330 px |
| D11 | — | ~120 `tooltip_text` littéraux en texte brut sans titre (`map_ui.gd`, `battle_hud.gd`, `pre_battle_dialog.gd`…) ; `unit_card.gd:385` colle du texte brut après `RichTooltip.unit` |

Défauts de la chaîne actuelle (D8) :

- T est une **bascule** : un appui par niveau (survol, T, survol, T…).
- La première infobulle est **native** (fenêtre surgissante de Godot) : elle disparaît dès que la
  souris bouge, ses mots-clés sont inatteignables sans T.
- T ouvre aussi l'**arbre des techniques** quand rien n'est verrouillable.
- Une bulle fille ne montre qu'un **résumé du Codex**, pas l'infobulle riche de l'entité (stats,
  coût, forces).
- Seuls les alias du Codex sont des liens : effets (`EFFECT_LABELS`), stats (`STAT_LABELS`) et
  jauges ne le sont pas.
- Au-delà de 6 bulles, la plus ancienne se ferme ; les bulles s'empilent au même endroit.

## 2. Mise en page en sections

### 2.1 Modèle

Les constructeurs de `RichTooltip` renvoient une **description structurée** au lieu d'une chaîne :

```gdscript
# TooltipSpec (Dictionary, présentation pure)
{
  "id": "unit_crossbowmen",          # entité (pour la chaîne et le Codex), "" sinon
  "kind": "unit",                    # unit | building | technology | resource | gauge | trait | skill | class | rule | plain
  "title": "Arbalétriers génois",
  "subtitle": "tireurs, 120 hommes",
  "icon": "unit_crossbowmen",        # icône d'en-tête (48 px)
  "headline": [{"icon": "strength", "label": "Effectif", "value": "96 / 120"},
               {"icon": "morale", "label": "Moral", "value": "62"}],   # 1-2 chiffres vedettes
  "stats": [{"key": "melee", "value": 4}, ...],                        # grille 2 colonnes
  "effects": [{"text": "Tir en bataille +10 %", "sign": 1, "before": 40, "after": 44}],
  "traits": {"strengths": [...], "weaknesses": [...], "abilities": [...]},
  "requires": [{"text": "Arsenal", "met": true}, {"text": "Arbalète à cranequin", "met": false}],
  "warnings": ["Indisponible : aucune place de recrutement"],
  "flavour": "Venus de Gênes, ils…",
  "footer": {"cost": "800 ₶", "upkeep": "60 ₶ / saison", "time": "2 tours"},
  "detail": [...]                    # lignes réservées à la version détaillée (§ 2.3)
}
```

`RichTooltip.to_bbcode(spec)` reste disponible (repli, tests, Codex) ; `RichTooltip.build(spec,
detailed)` construit le contrôle. Les appelants actuels qui passent une chaîne à `tooltip_text`
continuent de marcher : `tooltip_text` porte une clé `ib:<kind>:<id>` (ou le BBCode, repli), et
`_make_custom_tooltip` reconstruit la spec.

### 2.2 Rendu (`TooltipView`, nouveau `game/scripts/ui/tooltip_view.gd`)

De haut en bas, séparés par un filet parchemin (`HSeparator` stylé) seulement entre blocs non vides :

1. **En-tête** : bandeau légèrement plus sombre (`HudStyle`), icône 48 px à gauche, titre en
   taille Heading (`UiType`), couleur de catégorie (unité, bâtiment, technique, trait, règle —
   palette dans `data/ui/tooltip_style.json` avec schéma), sous-titre en italique sur sa propre ligne.
2. **Chiffres vedettes** : 1 ou 2 écussons (icône + grand chiffre + libellé). Choix par type :
   unité = effectif et moral (armée) ou coût et mêlée/tir (recrutement) ; bâtiment = effet
   principal ; province/jauge = valeur de la jauge ; technique = coût en recherche. Table dans
   `data/ui/tooltip_style.json` (`headline` par `kind`).
3. **Effets** : un par ligne, icône, signe en vert ou rouge ; si `before`/`after` connus :
   « Moral 60 → 65 » (D5). La valeur résultante vient du core via `live` ; aucun calcul de règle
   dans `game/`.
4. **Stats** : grille 2 colonnes icône + libellé + valeur (version détaillée seulement pour les unités,
   sauf les deux vedettes).
5. **Forces / faiblesses / capacités** : puces vertes, rouges, neutres.
6. **Conditions** : prérequis ✓ (vert) / ✗ (rouge) ; avertissements avec icône ⚠ (D6).
7. **Ambiance** : texte `description` en italique atténué, toujours en dernier bloc de contenu.
8. **Pied** : coût, entretien, durée avec icônes, à droite ; à gauche, en Caption atténué, l'aide
   de touche (« Alt : explorer »).

Tailles : corps en Body (`UiType`, 17 px à 900 px de référence), largeur 360 px × échelle
d'interface, hauteur ajustée, bornée à 70 % de l'écran (au-delà : défilement dans la bulle
verrouillée seulement).

### 2.3 Deux niveaux de détail (D9)

- **Survol simple** : version courte — en-tête, vedettes, effets, forces/faiblesses, conditions
  non remplies, pied. Visée : ≤ 8 lignes de corps.
- **Bulle verrouillée** (chaîne, § 3) : version complète — plus stats, capacités, époque,
  recrutement, prérequis remplis, ambiance, `detail`.

Pas de touche dédiée au détail : Maj est pris en bataille (file d'ordres, `battle_input.gd`) et
Ctrl par les groupes. Verrouiller suffit à tout lire.

### 2.4 Couverture

- Toutes les fonctions de `RichTooltip` passent au modèle.
- Les ~120 `tooltip_text` en texte brut passent par `RichTooltip.plain(title, body, hint)` (titre +
  corps + raccourci en pied) ; textes déplacés dans `data/ui/tooltips.json` (schéma), avec les
  tables `HUD_TEXTS`, `GAUGE_TEXTS`, `EFFECT_LABELS`, `STAT_LABELS` aujourd'hui codées en dur dans
  `rich_tooltip.gd`.
- `unit_card.gd:385` : le détail de bataille devient des `effects`/`warnings` de la spec.

## 3. Chaîne de bulles à la touche maintenue

### 3.1 Touche

Action **`tooltip_explore`**, **Alt** (Option sur macOS), listée dans la fiche des raccourcis (`shortcut_sheet.gd`).
Alt seul est libre sur la carte et en bataille (seul usage : Alt+Maj+1…6 des formations, ignoré
quand Maj est enfoncé). T garde son rôle actuel de bascule (compatibilité B1) ; l'aide en pied
annonce Alt. Décision consignée dans l'ADR 0109.

### 3.2 Comportement

- **Alt enfoncé au-dessus d'une infobulle native visible** (ou d'un contrôle qui en a une) : elle
  est convertie sur-le-champ en bulle verrouillée, version complète (§ 2.3), à la place exacte de
  l'infobulle native (pas de saut).
- **Alt maintenu** :
  - survol d'un mot-clé d'une bulle → bulle fille après **0,12 s** (au lieu de 0,35 s), déjà
    verrouillée ;
  - survol d'un autre mot-clé de la même bulle → la branche issue du mot précédent est remplacée ;
  - le mot-clé source reste surligné dans la bulle parente tant que sa fille est ouverte.
- **Alt relâché** : la chaîne reste ouverte tant que la souris est sur une bulle ; hors de toute
  bulle, fermeture après la grâce actuelle (0,4 s) ; les bulles épinglées au clic droit restent.
  Échap ferme tout. Clic gauche sur une bulle : fiche du Codex (inchangé).
- **Sans Alt** : comportement H2 actuel (délai 0,35 s, bulles non verrouillées).

### 3.3 Contenu des bulles filles

- Mot-clé désignant une **entité** (unité, bâtiment, technique, trait, ressource, compétence,
  personnage) → la bulle affiche la **spec riche** de l'entité (`RichTooltip` version complète), et
  non le résumé du Codex ; le Codex reste accessible au clic.
- Mot-clé désignant une **règle** (effet, stat, jauge, posture) → bulle `kind: "rule"` : titre,
  texte de `data/ui/tooltips.json` (avec les `{rule.…}` résolus comme aujourd'hui), lien vers la
  fiche du Codex s'il y en a une.
- Nouveau schéma de lien `ib:<kind>:<id>` dans `CodexText` à côté de `cdx:` ; les libellés
  d'effets, de stats et de jauges générés par `RichTooltip` sont émis comme liens `ib:rule:<clé>`.
  Liens en couleur de mot-clé (non soulignés quand déjà lus, comme `cdx:`).

### 3.4 Placement

- Chaque fille se place **à côté** de sa parente (à droite, sinon à gauche, sinon dessous), alignée
  sur la ligne du mot-clé ; jamais par-dessus une ancêtre.
- **Fil d'Ariane** en haut de la bulle la plus récente dès le 3ᵉ niveau
  (« Arbalétriers › Pavois › Fortifications »), chaque segment cliquable ramène à ce niveau
  (ferme les descendantes).
- `MAX_BUBBLES` passe de 6 à 10 ; si la place manque, les ancêtres les plus anciennes se
  **réduisent** à leur en-tête (clic pour les rouvrir) au lieu de se fermer.

## 4. Lots

| Lot | Contenu | Agent | Dépend de |
|---|---|---|---|
| IB0 | Squelette : `TooltipSpec`, `TooltipView` vide, `data/ui/tooltip_style.json` + `tooltips.json` + schémas, action `tooltip_explore`, tests désactivés, ADR 0109, planche « avant » (`ib_shot.gd`) | orchestrateur | fusion de `feat/p2c-codex` |
| IB1 | Rendu en sections (§ 2.1-2.2) ; unités, bâtiments, techniques migrés | dev | IB0 |
| IB2 | Autres constructeurs, ~120 infobulles brutes, tables vers `data/` (§ 2.4) | mech | IB1 |
| IB3 | Chaîne : touche maintenue, conversion de l'infobulle native, délai court, remplacement de branche (§ 3.2) | dev | IB0 |
| IB4 | Liens `ib:`, bulles filles riches et de règle, placement latéral, fil d'Ariane, réduction (§ 3.3-3.4) | dev | IB1, IB3 |
| IB5 | Avant → après (`before`/`after`) : champs `live` manquants au pont, côté `core/` si besoin | dev | IB1 |
| IB6 | Intégration, planche avant/après, jugement du joueur | orchestrateur | tous |

IB1 et IB3 en parallèle (fichiers disjoints : `rich_tooltip.gd`/`tooltip_view.gd` contre
`codex_bubbles.gd`). Budget : 0 $.

## 5. Tests et critères

- `ib_layout_test.gd` : pour chaque `kind`, la spec produit les blocs attendus ; version courte
  ≤ 8 lignes de corps pour les 20 unités de 1337 ; aucune infobulle ne dépasse l'écran à 720p ;
  tailles issues de `UiType` (contrôle C1 de `po_ui_test`).
- `ib_chain_test.gd` : Alt sur un contrôle à infobulle → 1 bulle verrouillée ; survol simulé de
  3 mots-clés successifs avec Alt → chaîne de 4 ; changer de mot dans une bulle remplace la
  branche ; relâcher Alt puis sortir → fermeture après la grâce ; Échap → 0 bulle ; T ouvre
  toujours l'arbre des techniques hors infobulle.
- `smoke.gd` vert ; aucun `tooltip_text` brut restant hors liste d'exceptions (test de grep).
- Jugement du joueur sur la planche avant/après (infobulle d'unité, de bâtiment, de province, chaîne
  de 4 bulles).

## 6. Hors périmètre

Aucune règle de jeu nouvelle ; comparaison avec l'équipement (sans objet dans ce jeu) ;
prise en charge de la manette ; refonte du contenu des fiches du Codex.

## 7. Décisions du joueur (28/09)

1. Touche de chaîne : **Alt**.
2. Détail complet : **réservé aux bulles verrouillées**.

Spec validée (« vas-y ») ; ADR 0109 écrit.
