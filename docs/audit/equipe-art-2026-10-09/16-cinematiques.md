# Cinematic artist — état des lieux (09/10, lecture seule)

## 1. État actuel
- **Caméra campagne** `map/campaign_camera.gd` : zoom 22-2600, inclinaison 30°→70° selon le zoom, Q/E, jamais sous le relief, inertie (`data/ui/camera_feel.json`, ADR 0097), fondu vue 3D/parchemin (ADR 0124), relecture du tour IA (`ai_turn_replay.gd`, ADR 0073).
- **Caméra bataille** `battle/battle_camera.gd` : inclinaison auto, orbite, `follow_unit`, glissement minicarte/alertes, vue tactique Tab, cadrage d'ouverture (plan spécial siège).
- **Plans scriptés** : `battle_cinematic.gd` (un plan au premier choc, orbite 50°/6 s, ralenti 0,35, bandes noires, ADR 0055, `battle_staging.json`) ; `battle_speech.gd` (travelling discours + cri) ; secousse `wall_collapse_fx.gd`.
- **Front-end** : `menu_backdrop_3d.gd` (Paris au crépuscule), `intro_cards.gd`, `loading_screen.gd`, `battle_loading_card.gd`, `scene_fader.gd`.
- **Rejeu** `battle_replay_bar.gd` (ADR 0072) : re-simulation ×1-×8, caméra libre seulement.
- **Prologue** NT4 jamais capturé.

## 2. Forces
- Ressenti caméra en données, « réduire les animations ».
- Caméra rendue exactement au joueur ; plans passables ; points d'entrée de capture.
- Rejeu à coût quasi nul ; discours, choc au ralenti, heure du jour.

## 3. Moments sans mise en scène
1. Campagne → bataille : coupe sèche ; retour = simple fondu.
2. Fin de bataille : rien (ni déroute, ni étendard, ni ralenti).
3. Mort/capture du général, porte enfoncée, brèche, point pris, déroute d'une aile : pas de caméra.
4. Campagne : prise de ville, début de siège, mort d'un roi, ouverture (snap sur la première armée).
5. Déploiement : plan fixe.
6. Rejeu : ni réalisateur auto, ni caméras cinéma, ni mode photo.
7. Plan de choc : une fois par bataille, obstruction possible par murailles.
8. Recentrages du tutoriel/prologue en coupe.
9. Réglages éclatés (`camera_feel.json`, `battle_staging.json`, constantes de `battle_speech.gd`).

## 4. Améliorations
| # | Action | Impact | Effort | Dépend de |
|---|---|---|---|---|
| 1 | Plan de fin de bataille (3-4 s ralenti, orbite étendard/déroute) | Fort | S | UI, audio |
| 2 | « Réalisateur » d'événements à partir des alertes CB5, avec espacement | Fort | M | Rust, audio, UX |
| 3 | Transition campagne→bataille (zoom province, survol à l'arrivée, recul au retour) | Fort | M | UI, tech |
| 4 | Moments de campagne (ville prise, siège, carte « Mort du roi ») | Fort | M | narration, Rust |
| 5 | Caméras de rejeu (suivi, réalisateur auto, mode photo) | Moyen | M | UI |
| 6 | Anti-obstruction (rayon murailles/bâtiments) | Moyen | S | tech art |
| 7 | Ouverture de campagne : glissement France → capitale | Moyen | S | front-end |
| 8 | Déploiement : survol de la ligne ennemie | Moyen | S | gameplay |
| 9 | `data/fx/camera_shots.json` + schéma + test `pose_at` | Indirect | S | QA |
| 10 | Recentrages en `glide`, captures du prologue | Faible-moyen | S | UX |

Ordre : 1 → 6 → 2 → 3 → 4 ; 9 tôt. Coût : nul.
