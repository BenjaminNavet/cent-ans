# 0150 — Lanceur Windows en `.exe`, commité à la racine

Date : 2026-10-02. Statut : accepté. Complète l'ADR 0117 (lanceur depuis les sources).

## Contexte

Sous Windows, on lance le jeu depuis un clone avec `Lancer Cent Ans.bat` (ADR 0117). Le joueur
veut un `.exe` : c'est ce qu'un utilisateur Windows cherche à la racine d'un dossier, et un
`.bat` paraît suspect ou s'ouvre dans un éditeur selon les réglages du poste.

Un exécutable doit exister dès le clone, donc avant toute compilation : le joueur n'a pas encore
forcément Rust, et c'est justement le lanceur qui le lui dit.

## Décision

- **`Lancer Cent Ans.exe` est commité à la racine du dépôt.** C'est le seul binaire compilé
  suivi par git (≈ 300 Ko ; `*.exe binary` dans `.gitattributes`). La DLL du cœur reste hors
  dépôt : elle change à chaque commit de `core/`, le lanceur presque jamais.
- **Source : `tools/launcher-windows/`**, crate Rust autonome, sans dépendance, hors de l'espace
  de travail `core/`. `tools/launcher-windows/build.sh` le compile (compilation croisée
  `cargo xwin` depuis le Mac, CRT statique comme la DLL, ADR 0087) et l'écrit à la racine. À
  relancer et commiter quand `src/main.rs` change.
- **Il fait ce que fait le `.bat`, rien de plus** : il retrouve le `bash.exe` de Git pour
  Windows (installation machine, utilisateur, scoop, ou à côté d'un `git.exe` du PATH ; jamais
  le bash de WSL), exécute `tools/launch.sh` depuis le dossier de l'exécutable en transmettant
  ses arguments, renvoie son code de sortie et garde la fenêtre ouverte en cas d'échec (sauf
  si `CI` est défini). Toute la logique de lancement reste dans `tools/launch.sh`.
- **Application console** : la fenêtre montre la compilation, l'import et le téléchargement du
  relief, qui durent plusieurs minutes au premier lancement.
- **Le `.bat` est conservé** comme solution de repli lisible. La CI Windows passe désormais par
  l'exécutable commité : c'est sa seule exécution sur un vrai Windows, puisqu'il est produit
  sur le Mac.

## Conséquences

- Les prérequis ne changent pas : Git pour Windows, Rust (avec les Build Tools) et Godot 4.7.
  L'exécutable ne les installe pas.
- L'exécutable n'est pas signé. Après un `git clone`, Windows ne l'a pas marqué comme venant
  d'Internet et SmartScreen ne s'affiche pas ; après un téléchargement du dépôt en zip, il
  affiche « Windows a protégé votre ordinateur » (Informations complémentaires → Exécuter quand
  même). Un antivirus peut aussi le bloquer : le `.bat` fait alors le même travail.
- Le binaire commité peut prendre du retard sur sa source si on oublie `build.sh`. La CI ne le
  détecte pas (une compilation croisée n'est pas reproductible à l'octet près depuis Windows) ;
  elle exécute les tests unitaires de la source et le binaire commité.
- Pas d'icône ni d'informations de version dans l'exécutable pour l'instant.
