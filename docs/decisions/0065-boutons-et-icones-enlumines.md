# 0065 — Boutons-médaillons enluminés et icônes d'action à l'encre (DA5)

Date : 2026-09-26. Lot DA5 de la direction artistique (bible `docs/design/2026-09-25-bible-da.md`,
§ 5 et § 8 ; écart n° 5 du § 10).

## Contexte

Les icônes d'action venaient de game-icons.net (SVG CC BY 3.0, huit auteurs, épaisseurs de trait
variées) et les boutons du bandeau étaient des cadres plats. La planche de style validée par le
joueur (`docs/img/da/planche/`) fixe deux registres : un bouton-médaillon enluminé (la cloche de
fin de saison) et des icônes au trait d'encre brune, une seule main.

## Décision

1. **Deux familles, une image par entrée**, décrites dans `data/ui/icons_ink.json` (schéma
   `data/schemas/icons_ink.schema.json`) : 78 icônes d'action (110 identifiants d'interface via
   `targets`) et 15 médaillons. La cloche reprend l'image validée (aucune génération).
2. **Génération** : `cent-ans assets ink-icons` (`tools/cent_ans_tools/ink_icons.py`), modèle
   `openai/gpt-5-image-mini`, réutilise `portraits.generate` (enveloppe du lot = 5 $ moins les lignes
   « DA5 » de `docs/budget.md`, plafond global, une ligne par passe). Chaque prompt = bloc de style
   des données + sujet, avec une **image de référence** tirée de la planche validée (couronne à
   l'encre ; médaillon de la cloche) pour garder le cadre et le trait. Sources brutes réduites
   gardées dans `tools/da5_raw/` : une image présente n'est jamais régénérée (`--only`, `--limit`,
   `--dry-run`, `--build-only`).
3. **Post-traitement par code, gratuit** :
   - icônes : alpha tiré de l'encre (écart de luminance à un fond de parchemin estimé localement
     par fermeture en niveaux de gris, ce qui efface vignettage et lavis), recadrage carré,
     **épaisseur de trait normalisée** (largeur médiane mesurée sur le squelette, décalage par
     distance signée jusqu'à 5 px à 128 px, soit ~1 px à 24 px), RVB = `HudStyle.INK` ;
     PNG 128 px dans `game/assets/icons/ink/` ;
   - médaillons : parchemin rendu transparent par remplissage depuis les bords, bord adouci, PNG
     256 px dans `game/assets/ui/medallions/`.
   Chaque dossier a un `index.json` (identifiant → fichier) et des `.import` avec mipmaps.
4. **Rendu** (`IconLibrary`) : l'index à l'encre est lu avant `icons.json` ; un identifiant couvert
   prend le PNG, sinon le SVG (repli automatique si un PNG manque). Teinte d'état : l'encre est cuite
   dans le PNG et `IconLibrary.tint(couleur)` donne la modulation (composantes > 1) qui la change en
   or au survol et à l'enfoncement, en encre pâlie désactivé (`apply_state_tints`). Médaillons :
   `decorate_medallion` (bouton existant, états par modulation) et `medallion_states` (normal,
   survol, enfoncé, désactivé dérivés d'une seule image par `Image.adjust_bcs`, utilisés par la
   cloche dessinée).
5. **Périmètre** : bandeau du haut (Diplomatie, Chronique, Cour, Techniques, Codex, Objectifs,
   Agents, Menu en médaillons ; le cadre plat ne revient qu'au survol), cloche de fin de saison
   (médaillon + banderole saison/année), Recruter et Former une armée (fiches de ville et de
   province), ordres de bataille et ordres du chef, filtres et légende de la carte, pastilles
   d'alerte, posture/ravitaillement/mouvement du sceau du général, jauges, classes, catégories.
   Les **icônes d'entité** (unités, bâtiments, techniques, compétences, ressources, régimes) restent
   des SVG : la bible les destine à des miniatures peintes (lot ultérieur).

## Conséquences

- Coût réel consigné dans `docs/budget.md` (section Direction artistique, lignes « DA5 »).
- Une nouvelle icône = une entrée du catalogue puis `cent-ans assets ink-icons --only <id>` ; une
  image ratée se refait en supprimant sa source brute.
- Les SVG game-icons.net restent dans le dépôt tant que les icônes d'entité les utilisent
  (`CREDITS.md` inchangé).
- Modulation > 1 : dépend du fait que le canevas 2D ne borne pas `modulate` avant le produit avec
  la texture (vrai en Godot 4) ; à défaut, l'icône resterait à l'encre au survol, sans autre effet.

## DA5b — Icônes d'entité en miniatures peintes (2026-09-26)

Bible § 8 : « Icônes d'entité : petites miniatures peintes, cadre commun. Jamais de pictogramme
plat au milieu d'une liste de miniatures. » Les icônes d'entité restaient des SVG game-icons.net.

1. **Catalogue** `data/ui/entity_icons.json` (schéma `data/schemas/entity_icons.schema.json`) :
   une miniature par entité (unités, bâtiments, techniques, compétences, ressources, régimes,
   catégories d'unité des marqueurs de bataille), taille 128 px, paramètres du cadre et du
   recadrage, bloc de style commun de génération.
2. **Réutiliser d'abord** : une entrée `source` dérive gratuitement une illustration peinte
   existante (`game/assets/illustrations/unit_*`, `bld_*`, `tech_*`, 16:9). Le carré est découpé
   sur le sujet : `crop` explicite dans les données quand l'œil l'a corrigé (texte peint, sujet
   décentré), sinon recherche automatique déterministe (énergie de détail et saturation, fonds
   d'azur diaprés atténués, léger biais central, somme maximale par image intégrale).
3. **Générer seulement ce qui manque** : une entrée `subject` (pas d'illustration) est générée
   une fois (`openai/gpt-5-image-mini` via `portraits.generate`, image de référence
   `docs/img/da5b/reference_miniature.png`, prompt = amorce + sujet puis bloc de style commun,
   un seul sujet lisible à 32 px sur fond d'azur diapré). Source brute gardée dans
   `tools/da5b_raw/` (jamais régénérée). Enveloppe du lot 6 $ (lignes « DA5b » de
   `docs/budget.md`).
4. **Cadre commun peint par code** : filet d'encre, or bruni en dégradé (clair en haut à gauche),
   filet d'azur, filet d'encre, coins arrondis ; peinture réduite avec léger contraste,
   saturation et accentuation pour rester lisible à 32 px. PNG RGBA dans
   `game/assets/icons/entity/` + `index.json` (id → fichier, recadrages utilisés).
   CLI : `cent-ans assets entity-icons` (`--dry-run`, `--only`, `--limit`, `--build-only`,
   `--envelope`), planche `docs/img/da5b/planche_miniatures.png` (32/64/128 px).
5. **Rendu** : `IconLibrary` lit `entity/index.json` en premier — priorité miniature d'entité >
   encre > SVG ; `is_entity(id)` ; une miniature n'est jamais teintée ; `decorate_button` passe
   `expand_icon` pour qu'une miniature 128 px ne gonfle pas la taille minimale du bouton.
   Marqueurs de bataille et cartes d'unité du bandeau : miniature de catégorie
   (`unit_category_<rendu>`, un seul registre, plus de mélange encre/SVG) ; cartes du bilan :
   miniature du type d'unité. La miniature couvre la plaque (elle porte son cadre) au lieu d'être
   posée sur une pastille de parchemin.

Conséquences : les SVG d'entité ne servent plus que de repli (identifiant sans miniature) ;
`CREDITS.md` inchangé tant qu'ils restent dans le dépôt. Les traits s'affichent toujours par
catégorie (icônes à l'encre) : des miniatures par trait (59) sont un lot possible.

## DA7c — une icône à l'encre par trait de personnage (2026-09-26)

Bible § 8. Les 59 traits (`data/traits/`) n'avaient pas d'icône propre : les quatre points
d'affichage (infobulle riche, fiche de personnage, encyclopédie liste et fiche) montraient tous
l'icône générique de catégorie (`trait_category_<personality|martial|governance|physical|
acquired>`, 5 icônes DA5).

1. **Même catalogue, même outil, un groupe de plus.** `data/ui/icons_ink.json["icons"]` gagne
   59 entrées `"group": "trait"` (ajouté à l'énumération de `data/schemas/icons_ink.schema.json`) :
   un prompt visuel par trait (anglais, sans texte, même bloc de style DA5), `id == targets[0] ==`
   l'identifiant du trait (`trait_admiral`…), pour que `IconLibrary.get_icon(trait.id)` prenne
   l'icône directement, avec repli sur l'icône de catégorie générique si elle manque
   (`IconLibrary.resolve`, inchangé).
2. **`cent-ans assets ink-icons` réutilisé, pas de nouvel outil**, étendu pour porter plusieurs
   lots indépendants sur le même catalogue : `Entry.group`, filtre `group=` sur `entries()`/
   `plan()`, `lot_spent(ledger, prefix=)` (défaut `"DA5 :"`, rétro-compatible). La commande CLI
   gagne `--group`, `--subject`, `--budget-cap` pour que DA7c ait sa propre ligne de grand livre
   et sa propre enveloppe (4 $) sans toucher au plafond DA5 déjà quasi épuisé (4,84 $ sur 5 $) :
   ```
   cent-ans assets ink-icons --group trait \
     --subject "DA7c : icônes de trait à l'encre" --budget-cap 4.0 --dry-run
   ```
3. **Rendu** : les quatre sites (`RichTooltip.trait_tip`, `CharacterSheet._fill_traits`,
   `Encyclopedia._entry_icon`, `Encyclopedia._trait_fiche`) passent l'identifiant du trait plutôt
   que `"trait_category_" + category` à `IconLibrary`.

Coût réel consigné dans `docs/budget.md` (lignes « DA7c »). Conséquences : les traits d'un même
personnage se distinguent désormais d'un coup d'œil (fiche, infobulle, cour, encyclopédie) ; un
trait ajouté plus tard sans image générée retombe sur l'icône de sa catégorie, jamais une icône
manquante.
