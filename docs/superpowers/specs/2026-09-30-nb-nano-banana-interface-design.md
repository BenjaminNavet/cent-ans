# NB — Nano Banana 2 : ancre de style et interface enluminée

Date : 2026-09-30. Statut : approuvé en conversation, en attente de relecture écrite.
Références : bible DA `docs/design/2026-09-25-bible-da.md`, kit d'interface ADR 0050,
`tools/cent_ans_tools/ui_illumination.py`, `tools/cent_ans_tools/openrouter.py`.

## 1. Objectif

Profiter de Nano Banana 2 (`google/gemini-3.1-flash-image` sur OpenRouter) pour :

1. donner à la direction artistique une **image d'ancrage** (planche maîtresse), jointe à
   toute génération future comme référence de style ;
2. mesurer ce que valent NB2, NB2 Lite, Nano Banana Pro et l'actuel `openai/gpt-5-image-mini`
   sur nos sujets, pour chiffrer la reprise de l'existant dans un chantier ultérieur ;
3. redessiner les 16 textures du kit d'interface enluminée sans perdre l'exactitude du 9 tranches.

Hors périmètre (chantiers suivants, budget propre) : illustrations narratives, carte de
campagne, reprise des portraits/événements/icônes, interface 2× HiDPI.

## 2. Budget

Plafond **10 $**, clé OpenRouter du joueur, section « Nano Banana NB » de `docs/budget.md`
(format 6 colonnes lu par `budget.py`).

| Lot | Enveloppe |
|---|---|
| NB-DA planche maîtresse | 1,00 $ |
| NB0 sonde des modèles | 1,50 $ |
| NB1 kit d'interface | 5,00 $ |
| Réserve (rejets, retouches) | 2,50 $ |

Toute génération passe par `--dry-run` (estimation) puis `budget.check`. Prix estimés NB2 :
0,045 $ (512), 0,067 $ (1K), 0,10 $ (2K), 0,15 $ (4K) ; Lite ≈ moitié. Le coût réel
(`usage.cost`) fait foi dans le grand livre.

## 3. Client OpenRouter (évolution de `openrouter.py`)

`request_image` et `generate_image` gagnent des paramètres optionnels, rétrocompatibles :

- `image_config: dict | None` → champ `image_config` du corps (`aspect_ratio`, `image_size`
  parmi `512`, `1K`, `2K`, `4K`) ;
- `seed: int | None` ;
- `images` déjà présent (références, jusqu'à 14 pour NB2) ; `generate_image` le transmet.

`generate_image` estime le prix selon `image_size` (table de tokens par taille : 512 → 747,
1K → 1120, 2K → 1680, 4K → 2520 — valeurs publiques de Google, recalées sur `usage.cost` après la première image — multipliée par `image_output` du modèle). Pas de
transparence native : les pièces à détourer sont générées sur fond vert uni `#00FF00`.

## 4. NB-DA — planche maîtresse (ancre de style)

- **Sources** : 4 à 6 folios en domaine public (reproductions Wikimedia Commons) des
  manuscrits de la bible § 1 (Pucelle, *Grandes Chroniques* BnF fr. 2813, Froissart de
  Gruuthuse, *Très Riches Heures*). Téléchargés dans `tools/nb_raw/sources/` avec un
  `SOURCES.md` (URL, manuscrit, folio, mention de licence). Non livrés dans le jeu,
  non commités (répertoire ignoré par git).
- **Synthèse** : NB2, 2K, 3:2, `reasoning_effort` élevé ; prompt = bloc `STYLE` commun +
  consigne de planche : nuancier (teintes héraldiques § 3.1 et interface § 3.2), trait d'encre,
  dorure à la feuille, une frise, un coin, une lettrine sans lettre lisible, une figure en buste,
  un petit paysage ; aucun texte. 4 variantes (seeds consignées).
- **Jugement** : planche contact des 4 variantes, choix du joueur.
- **Livrables** : `data/art/style/anchor.png` (variante retenue, 2K) et
  `data/art/style/anchor.yaml` (modèle, seed, prompt, sources) ; § 13 « Image d'ancrage »
  ajouté à la bible DA : tout nouvel outil de génération NB joint l'ancre en première image
  et dit dans le prompt « Image 1 is the style reference ».

## 5. NB0 — sonde des modèles

- 3 sujets existants : un portrait (souverain déjà généré), un événement, une icône d'entité,
  prompts reconstruits depuis les données par les outils existants (`portraits.py`,
  `event_art.py`, `entity_icons.py`), sans les modifier : un script de sonde appelle leurs
  fonctions de construction de prompt.
- Modèles : NB2, NB2 Lite, Nano Banana Pro (1 image chacun) × {sans ancre, avec ancre} ;
  gpt-5-image-mini = l'asset existant (0 $). 3 × 3 × 2 = 18 images, 1K.
- Livrable : `docs/research/nb0_models.png` (grille sujets × modèles, avec/sans ancre, coût
  par image en légende) + `docs/research/nb0-sonde-modeles.md` (verdict du joueur, modèle
  retenu pour NB1 et pour les chantiers suivants).

## 6. NB1 — kit d'interface redessiné

### 6.1 Principe (guide de forme + ancre)

Pour chacune des 16 textures de `kit.json`, NB2 reçoit deux images :

1. l'ancre de style ;
2. la texture procédurale actuelle, agrandie ×6 (plus proche voisin) sur fond vert,
   comme **guide de forme**.

Consigne : redessiner le cadre en respectant exactement la géométrie du guide (épaisseur des
bordures, coins, zone intérieure), dans le style de l'ancre ; intérieur en vélin uni ; fond
extérieur vert uni. Format le plus proche du ratio de la pièce ; taille 1K (512 pour barres et
curseur). 3 variantes par texture, seeds consignées.

Pièces de décor isolées (≈ 10 : lettrines, drôleries, rinceaux, fleurons de séparation), sur
fond vert, pour usage ultérieur ; générées et stockées, câblage hors périmètre.

### 6.2 Chaîne locale (`tools/cent_ans_tools/ui_ornaments.py`)

API :

- `build_requests(kit, anchor) -> list[OrnamentRequest]` : prompts et références depuis
  `data/art/ui_ornaments.yaml` (schéma `data/schemas/ui_ornaments.schema.json` : id de pièce,
  consigne propre, ratio, taille, nombre de variantes) ;
- `generate(requests, raw_dir, dry_run)` : appels payants, brutes dans `tools/nb_raw/ui/`
  (réutilisées si présentes, ignorées par git) ;
- `key_out(image) -> RGBA` : détourage du vert (distance de chrominance + décontamination
  des franges) ;
- `fit_to_piece(image, piece) -> RGBA` : rognage sur la boîte englobante, réduction
  (Lanczos) à `size` de `kit.json`, recalage des marges 9 tranches ;
- `seam_fix(image, piece)` : raccord des zones étirées/répétées (bandes centrales), réutilise
  `material_gen.make_tileable` sur l'axe concerné ;
- `contact_sheet(...)` : planche avant/après.

`ui_illumination.build` gagne une option : si `game/assets/ui/illumination/nb/<id>.png`
(variante retenue, traitée) existe, elle remplace le tracé procédural ; sinon le procédural
reste (repli). `kit.json` et `parchment_theme.tres` ne changent pas.
Choix des variantes : `data/art/ui_ornaments.yaml` porte `selected: <n>` par pièce.

CLI : `uv run --project tools cent-ans assets ui-ornaments [--dry-run] [--only id]…
[--sheet planche.png]`.

### 6.3 Jugement

Planche avant/après des 16 textures (lecture fichier), choix des variantes par le joueur,
puis **une** capture en jeu d'un écran chargé (budget 3 captures respecté).

## 7. Tests

pytest (`tools/tests/test_ui_ornaments.py`, `test_openrouter.py` étendu), sans appel réseau :

- corps de requête : `image_config` et `seed` présents seulement si fournis ; estimation
  selon `image_size` ;
- `key_out` : aucun pixel à dominante verte restant au-delà d'un seuil sur une image synthétique ;
- `fit_to_piece` : dimensions = `kit.json`, marges opaques aux bons endroits ;
- `seam_fix` : écart aux raccords ≤ 4/255 ;
- repli : pièce absente → sortie identique au procédural (même seed) ;
- schéma YAML validé.

Godot : `smoke.gd` passe ; test existant du thème (s'il charge les textures) inchangé.

## 8. Organisation

- Branche `feat/nb`, worktree `../game_project-nb` ; note `docs/wip/nb.md`.
- NB-S (squelette) : API vide, schéma, YAML vide typé, tests `skip`, section budget → commit.
- Ordre : NB-S → client OpenRouter → NB-DA (jugement) → NB0 (jugement) → NB1 chaîne →
  NB1 génération → jugement → fusion `--ff-only` dans `main`.
- ADR **0135** « Ancre de style et redessin guidé du kit d'interface » (numéro réservé ;
  0129-0134 laissés aux chantiers en cours) : pourquoi guide de forme + ancre plutôt que
  panneaux entiers ou ornements assemblés.
- Agents : chaîne locale et tests → `cent-ans-mech` ; appels payants et jugements →
  session principale.

## 9. Risques

- NB2 ne respecte pas la géométrie du guide → `fit_to_piece` recale ; si l'écart reste
  visible, retouche conversationnelle (image + consigne) sur la réserve ; au pire repli
  procédural pour la pièce.
- Frange verte au détourage des ors → décontamination + test.
- Textures 9 tranches petites (≤ 204 px) : le détail fin se perd à la réduction ; accepté en
  1×, l'interface 2× est un chantier ultérieur.
