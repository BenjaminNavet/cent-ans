# 0097 — Gabarit d'interface et d'étalonnage (chantier PO)

Date : 2026-09-27. Statut : accepté. Spec : `docs/superpowers/specs/2026-09-27-po-polish-design.md`.
Détail normatif : bible DA § 12 (`docs/design/2026-09-25-bible-da.md`).

## Contexte

Le joueur trouve le jeu « amateur ». La planche `docs/img/po/avant/` (10 vues de la tranche
verticale, 1280×720) montre que l'art de base tient (bible DA, DA0-DA7, figurines fines, portraits)
mais que ce qui relie les assets ne tient pas :

- chaque écran pose ses panneaux en positions absolues : jusqu'à six fenêtres flottantes qui se
  chevauchent (bandeau de fin de tour sur le panneau de province, rencontre sur la minicarte, libellé
  de chemin sur le journal), et la colonne du menu titre sort de l'écran à 720 px ;
- 179 `add_theme_font_size_override` donnent des tailles au hasard, souvent sous 12 px ;
- un message d'outil (« `uv run` … ») est affiché au joueur ;
- la lumière de bataille est un midi neutre codé en dur (`battle_atmosphere.gd`), la carte est un
  vert plat sous une brume grise.

## Décision

1. **Zones d'écran fixes.** Un autoload `UiLayout` possède six zones (`TOP_BAR`,
   `BOTTOM_SELECTION`, `MINIMAP`, `SIDE_PANEL`, `TOASTS`, `MODAL`) aux rectangles fixés par ancres
   en proportion de l'écran. API : `claim(zone, control)`, `release(control)`, `zone_rect(zone)`,
   `toast(text, icon, seconds)`, signal `side_panel_changed`. `SIDE_PANEL` n'a qu'un occupant ;
   `MODAL` assombrit le fond et bloque les entrées ; `TOASTS` empile 3 avis au plus, 6 s chacun.
   Les rectangles ne dépendent jamais de la taille minimale des enfants (boucles UI1).
2. **Quatre tailles de texte**, variations de type du thème `parchment_theme.tres` : `Title` 26,
   `Heading` 20, `Body` 17 (taille par défaut actuelle), `Caption` 14 px à la hauteur de référence
   de 900 px, soit 31 / 24 / 20 / 17 px à 1080p (échelle d'interface ×1,2). Rien sous `Caption`.
   Classe `UiType` pour les appliquer ; plus de surcharge de taille dans un écran migré.
3. **Espacements** 4 / 8 / 16 / 24 px, constantes de thème.
4. **Mouvement** : `UiMotion` (fondu 0,12 s + glissement 8 px, rien en headless), `SceneFader`
   (fondu 0,25 s entre scènes), sons `ui_click` / `ui_open` (CC0).
5. **Mode développeur** : clé `dev_mode` de `Settings`, vraie avec `--dev`, accesseur
   `Settings.is_dev()`. Tout texte d'outil n'est affiché qu'en mode dev ; sinon `push_warning`.
6. **Heure du jour** : bloc `time_of_day` (`morning`, `midday`, `evening`) de
   `data/fx/atmosphere.json`, validé par le schéma. Les préréglages de soleil de
   `battle_atmosphere.gd` y sont déplacés. L'étalonnage reste la LUT procédurale existante. En
   bataille, l'heure est un tirage de rendu déterministe sur la graine (`midday` exclu par temps
   couvert) ; le core ne choisit rien.
7. **Vérifications** : tests headless `po_ui_test.gd` (C1 textes d'outil, C2 rectangles sans
   chevauchement à 1280×720 et 1920×1080 et un seul panneau latéral, C3 tailles) et
   `po_grade_test.gd` (C4 : chaque contexte carte × saison et bataille × météo × heure résout un
   préréglage complet).

## Conséquences

- Phase 1 : seule la tranche verticale (menu, faction, carte, fin de tour, rencontre, bataille,
  résultat) est migrée, par les lots PO1 à PO5.
- Phase 2 : cour, techniques, diplomatie, codex, encyclopédie, arbre familial, fiche personnage,
  sièges et naval migrent mécaniquement, sans nouvelle règle de style (lots P2a-P2f), y compris
  le nettoyage des surcharges de taille restantes.
- Tout nouveau panneau (HL2, suites de CV3) rejoint `SIDE_PANEL` ou `MODAL` au lieu d'une
  position absolue.
- Budget : 0 $ (LUT procédurales, textures et sons CC0) ; enveloppe ≤ 3 $.
