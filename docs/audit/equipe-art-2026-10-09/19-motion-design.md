# Motion designer — état des lieux (09/10, lecture seule, rien vérifié à l'écran)

## 1. État actuel
- Brique commune `game/scripts/ui/ui_motion.gd` (`UiMotion`) : fondu 0,12 s + glissement 8 px + son `ui_open`, courbe linéaire (aucun `set_trans`/`set_ease`) ; règle ADR 0097 + bible § 12.4. Utilisée dans ~12 fichiers (menus, panneaux tech/diplo/cour/fiche, toasts et panneaux latéraux de `ui_layout.gd`).
- ~20 `create_tween` écrits à la main dans 15 fichiers : `scene_fader_layer.gd` 0,25 s SINE ; `start_menu.gd` 0,6/0,45/0,3/1,2 ; `menu_backdrop_3d.gd` 1,1 ; `intro_cards.gd` 0,8 + zoom 6 % ; `loading_screen.gd` 0,45 ; `battle_loading_card.gd` 0,35 ; `season_banner.gd` 0,3 + 1,2 SINE ; bandeau de tour 1,1 + 0,45 ; `outcome_notice.gd` 6,0 + 0,8 ; `turn_wait_indicator.gd` ; `news_letters.gd` 0,35 ; `army_marker.gd` 0,3 ; `order_ripple.gd` 0,5 s.
- Boucles `sin()` : pulsation tutoriel, reflet du titre. Aucun `AnimationPlayer`.
- « Réduire les animations » respecté presque partout ; test `po5_motion_test.gd`.

## 2. Forces
- Règle écrite + API centrale déjà adoptée.
- Accessibilité traitée tôt ; tout déterministe en headless.
- Un peu de « juice » : onde d'ordre, cartouche de saison, intro, décor 3D du menu ; son lié à l'ouverture.

## 3. Faiblesses
1. **Bug probable de dérive dans `UiMotion`** (à confirmer à l'écran) : `fade_out` laisse +8 px, le `fade_in` suivant repart de là → un panneau ancré descend de 8 px à chaque cycle ; tweens précédents non arrêtés (double clic).
2. Glissement sans effet dans les conteneurs (toasts en VBox).
3. Toasts retirés d'un coup, les autres sautent.
4. Modales figées (`modulate.a = 1.0` forcé en zone MODAL), voile compris.
5. Panneaux non animés : faction, encyclopédie, codex, journal, raccourcis, mercenaires, rançon, infobulles.
6. ~12 durées éparpillées, certaines en dur, pas d'échelle commune, linéaire par défaut.
7. Pas de feedback : compteurs qui sautent, survol sans transition, pas de décalage 1 px à l'appui (prévu par la bible), cloche sans pulsation, aucune validation marquée.
8. Documentation de l'animation d'interface maigre.

## 4. Améliorations
| Prio | Action | Impact | Effort | Dépend de |
|---|---|---|---|---|
| P1-A | Fiabiliser `UiMotion` (position de repos, kill du tween, pas de glissement en Container, `Accessibility.reduce_motion()`) | Fort | S | QA |
| P1-B | Fondu de sortie des toasts, fondu des modales et du voile | Fort | S | UI |
| P2-C | Échelle nommée FAST 0,12 / MED 0,3 / SLOW 0,6 / CEREMONY 1,2, SINE/CUBIC OUT/IN ; bible § 12.4 | Moyen | M | DA |
| P2-D | `UiMotion` sur les panneaux figés | Moyen | S-M | UI |
| P3-E | Compteurs animés (0,4 s, teinte or/rouge) | Fort | M | économie |
| P3-F | Survol 0,08 s, décalage 1 px à l'appui, son de survol | Moyen | M | audio |
| P3-G | Pulsation de la cloche + badge TRANS_BACK | Moyen | S | UX |
| P3-H | Moments cérémoniels (sceau à la signature, rapport de saison qui se déroule) | Fort | L | art, audio |
| P4-I | Menu en cascade, générique défilant | Moyen | M | DA |
| P4-J | Transitions unifiées derrière SceneFader | — | S-M | — |

Coût : nul partout.
