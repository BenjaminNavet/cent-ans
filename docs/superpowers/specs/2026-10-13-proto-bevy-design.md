# Prototype de mesure Bevy contre Godot (P0) — design

Date : 2026-10-13. Statut : validé en brainstorming, en attente de relecture de la spec.

## Contexte

Le joueur envisage de remplacer le client Godot (`game/`, ~170 000 lignes de GDScript) par un moteur
Rust qu'il contrôle finement. Objectifs, par ordre d'importance : surface de contrôle maximale pour
les agents (développement futur), jeu plus rapide, jeu plus beau. Le coût de réécriture est accepté.

Choix déjà faits pendant le brainstorming (non rediscutés) :
- Base : **fork de Bevy** figé dans le dépôt (pas de traduction de Godot en Rust, pas de fork de
  Fyrox ni de Godot C++). `core/` reste tel quel et serait appelé directement, sans pont GDExtension.
- Migration **d'un bloc** vers la **parité complète** avant de supprimer le client Godot.
- Avant tout, un **prototype de mesure** (ce document), sur le modèle de l'ADR 0093 (Godot contre
  Unreal) : aucun banc public fiable ne compare Bevy et Godot.

## Découpage de la réécriture

| Étape | Contenu |
|---|---|
| **P0** | Prototype de mesure Bevy contre Godot, verdict en ADR (ce document). |
| P1 | Socle : fork de Bevy figé, client `client/` en Rust branché sur `core/`, surface de contrôle agents. |
| P2 | Carte de campagne : relief en streaming, provinces, colonies, vie de la carte, vue parchemin. |
| P3 | Bataille : figurines, animations, sièges, effets. |
| P4 | Interface : HUD, panneaux, infobulles et chaînes Alt, codex. |
| P5 | Audio, lanceur, export Windows, mise à jour automatique, puis bascule et suppression de `game/`. |

P1 à P5 n'ont chacune leur spec qu'après un verdict positif de P0. Verdict négatif : arrêt, l'ADR dit
pourquoi.

## Périmètre de P0

Scène commune aux deux moteurs, reprise de l'ADR 0093 (`tools/proto_moteur/assets/` : terrain, mur,
grange, meule, charrette, sapins, herbe, fantassins `infantry_0-2`), complétée par ce qui stresse le
jeu réel :
- une portion de relief réel (une page de hauteurs du jeu convertie en PNG 16 bits) ;
- **5 000 fantassins** instanciés et animés (marche en boucle) ;
- végétation dense (herbe et sapins instanciés, densité réglable) ;
- un **panneau d'interface parchemin** : texte, bouton, une infobulle au survol.

Même caméra, même lumière du matin, même trajet de caméra scripté (vue large puis vue rapprochée)
dans les deux moteurs. Hors périmètre : `core/`, sons, campagne, IA, assets nouveaux.

## Critères et seuil de décision

Mesurés sur le Mac du joueur, les deux moteurs **en alternance** dans une même fenêtre de temps
(principe de `--bench-ab`, ADR 0169), charge système notée :
1. **Vitesse** : temps d'image p50 et p95 ; temps CPU et temps GPU séparés quand le moteur les donne.
2. **Rendu** : 4 captures côte à côte (vue large, vue rapprochée), jugées à l'œil par le joueur.
3. **Contrôle par agents** : pour une même retouche (changer la densité de végétation et le vérifier
   sans capture), durée et tokens consommés par un agent ; liste de ce qui est interrogeable en texte.
4. **Interface** : effort pour obtenir le panneau parchemin (durée, lignes de code, défauts restants).

**Seuil :** la réécriture P1-P5 est lancée si Bevy est au moins aussi rapide (p50 et p95), pas
visiblement moins beau après une journée de réglage, et clairement meilleur sur le critère 3.

## Architecture

Tout vit dans `tools/proto_moteur/`, sans toucher `core/` ni `game/`. Travail dans un worktree
dédié ; seul le `main` local reçoit la fusion.

```
tools/proto_moteur/
  scene.json        # description commune : objets, positions, caméra, trajet, lumière, densités
  bevy/             # crate binaire Rust, Bevy figé (dernière version publiée), hors workspace core/
  godot/            # projet Godot minimal existant, complété
  bench/            # script Python (uv) : lance les deux en alternance, agrège les mesures
  out/              # CSV de mesures et captures (gitignorés, sauf les 4 captures finales)
```

### Composants

- **`scene.json`** : source unique de la scène pour les deux moteurs (objets, transformations, caméra,
  trajet, lumière, densités d'unités et de végétation, chemin de la heightmap).
- **Client Bevy** : chargement glTF, terrain maillé depuis la heightmap, fantassins instanciés et
  animés, herbe et sapins instanciés, panneau parchemin en `bevy_ui`. Surface de contrôle en ligne de
  commande :
  - `--dump-state` : entités, composants, compteurs en JSON ;
  - `--metrics` : temps CPU/GPU par passe, nombre de draw calls, en CSV ;
  - `--shot <fichier>` : capture à un instant du trajet ;
  - `--set clé=valeur` : surcharge d'une valeur de `scene.json` sans recompiler.
- **Client Godot** : même scène, mêmes options (`--metrics`, `--shot`, `--set`, et `--dump-state`
  dans la mesure du possible pour comparer honnêtement le critère 3).
- **Banc** (`bench/`) : alterne Bevy et Godot (au moins 3 passes chacun), même trajet, produit un CSV
  et un tableau p50/p95 par moteur.

### Contraintes

- **Focus du joueur** : aucune fenêtre au premier plan. Godot via `tools/godot_bg.sh` ou en headless ;
  Bevy via `open -g` (application empaquetée minimale) si un rendu sans fenêtre visible est impossible.
- **0 $** de dépense cloud ; aucun asset généré.
- **Pas de push** : tous les commits restent locaux tant que le joueur ne l'a pas autorisé.
- Le crate Bevy n'entre pas dans le workspace `core/` (build et `target/` séparés).

## Risques et parades

- **Animation de 5 000 unités** : si le skinning instancié de Bevy ne tient pas, animation précalculée
  en texture (vertex animation) — appliquée **aux deux moteurs** pour garder la comparaison honnête.
- **Machine chargée** : mesures en alternance, charge système relevée à chaque passe.
- **Rendus non comparables** (exposition, tonemapping, espace colorimétrique) : étalonnage préalable
  sur une scène neutre avec des valeurs identiques.
- **Dérive de durée** : plafond d'une journée d'agents ; un critère non mesuré dans ce délai est
  déclaré tel quel dans l'ADR, jamais estimé.

## Vérification

- `cargo build --release`, `cargo clippy -- -D warnings`, `cargo test` (chargement de `scene.json`
  et de la heightmap) dans `tools/proto_moteur/bevy/`.
- `uvx ruff check --fix` et `uvx ruff format` sur `bench/`.
- Une exécution de chaque client produit son CSV de mesures et sa capture.
- Captures : 10 au plus pour toute la tâche, dont les 4 finales destinées au joueur.

## Exécution

Une vague de 3 agents Sonnet en parallèle (scène commune + banc ; client Bevy ; compléments Godot),
puis intégration, mesures et ADR par la session principale. Squelette commité d'abord, note
`docs/wip/proto-bevy.md` mise à jour à chaque commit `wip:`.

## Livrables

1. `tools/proto_moteur/{scene.json,bevy/,godot/,bench/}`.
2. `docs/wip/proto-bevy.md`.
3. Tableau de mesures et 4 captures côte à côte.
4. ADR `docs/decisions/0347-moteur-bevy-ou-godot.md` (numéro à confirmer au moment de l'écrire) :
   verdict selon le seuil, lancement ou non de P1-P5.
5. Aucun push.
