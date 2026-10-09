# Marketing art — état des lieux (09/10, lecture seule, 3 images regardées)

## 1. État actuel
- **README** (200 l., en français) : accroche, galerie de 6 captures (`docs/img/readme/`, 1600×900, 08/10, suivies via `!docs/img/readme/`), régénérables par `game/tests/readme_gallery.gd` ; licences GPL v3 (code) et CC BY-SA 4.0 (assets).
- **Dépôt GitHub** `BenjaminNavet/cent-ans` (public) : bonne description, 10 topics ; pas de `homepageUrl` ; pas d'image Open Graph personnalisée ; 0 étoile ; **aucune Release** (seul `cent-ans-relief` en a). Distribution par `git clone -b stable` et lanceurs `Lancer Cent Ans.{command,sh,exe,bat}`.
- **Icône d'application absente partout** : pas de `game/icon.*` ni `config/icon` dans `project.godot` ; `application/icon=""` macOS et Windows dans `export_presets.cfg` (l. 29, 97, 98) ; lanceur Rust `tools/launcher-windows/` sans icône.
- **Splash** `game/assets/ui/boot_splash.png` : carte sépia, cartouche en serif générique (pas IM Fell English).
- **Écran titre** (`menu.jpg`) : lettrine « C » or sur azur, IM Fell, ost devant Paris au crépuscule — l'image la plus « marque ».
- **Identité** : bible DA § 1-4 et § 12 (enluminure, palette, IM Fell), aucune section logo/marketing. Métadonnées d'export présentes (`fr.navet.centans`, 1.0). Ni CHANGELOG, ni bande-annonce, ni page Steam/itch, ni key art.

## 2. Forces
- Galerie authentique, régénérable par script après chaque grosse fusion.
- Écran titre fort et cohérent ; la lettrine fait déjà office de logo.
- Positionnement clair (« Total War libre sur la guerre de Cent Ans »), topics bien choisis.
- Bible DA solide pour tout visuel promotionnel ; README juridiquement propre.

## 3. Faiblesses
1. **Pas d'icône** : application exportée et `.exe` du lanceur affichent l'icône Godot ou celle de l'OS.
2. **Pas de Release GitHub** : ni page de téléchargement, ni notes, ni binaires ; installation par git.
3. Pas d'image Open Graph : partages réseaux/Discord génériques.
4. Logo non unifié (splash sépia en serif générique vs écran titre IM Fell azur et or).
5. README en français seul, vite technique ; ni badges, ni GIF, ni bandeau titre.
6. `bataille.jpg` : artefacts de netteté/postérisation (« filtre huile »), vue de dos, ciel plat.
7. Pas de key art ni de déclinaisons (capsule, bannière, carré).
8. Pas de bande-annonce ni de GIF de bataille.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| P1 | Icône : lettrine « C » simplifiée (1024/256/16 px), `.icns`/`.ico`/`icon.svg` branchés dans `project.godot`, `export_presets.cfg`, lanceur Windows | Fort | S | 0 | DA, build |
| P1 | Image Open Graph 1280×640 (écran titre recadré + logo + accroche), téléversement manuel par le propriétaire | Fort | S | 0 | — |
| P1 | Première Release GitHub v0.x : binaires macOS/Windows, notes illustrées, CHANGELOG | Très fort | M | 0 | CI, signature macOS, ADR 0212 |
| P2 | Logo unifié (lettrine + IM Fell, déclinaisons), splash refait, section « Marque » dans la bible | Moyen-fort | S-M | 0 | DA, UI |
| P2 | En-tête README : bandeau, badges, bouton « Télécharger », résumé anglais, GIF de charge 5-8 s | Fort | S-M | 0 | Release |
| P2 | Remplacer `bataille.jpg` (de face, lumière rasante, bannières) ; vérifier netteté/JPEG du script | Moyen | S | 0 | rendu |
| P3 | Key art enluminé (Crécy ou Poitiers) décliné en capsules Steam et bannière itch | Fort à terme | L | 0 à 500-2 000 € | DA, licences |
| P3 | Bande-annonce 60-90 s via `--screenshot`, `--closeup`, `--historical` | Fort | L | 0-300 € | audio, mise en scène |
| P3 | Pages itch.io puis Steam (FR/EN, 5 captures min.) | Fort | M | 0-100 $ | Release, localisation |
| P4 | Kit presse `docs/press/` + `homepageUrl` | Moyen | S | 0 | communauté |

Ordre : icône et Open Graph tout de suite ; la Release v0.1 est le vrai déblocage ; puis logo, README, key art, bande-annonce, boutiques.
