# EP4 — son de mêlée de proximité

Lot du chantier « batailles épiques » (`docs/wip/epic.md`). Objectif : bataille qui sonne
massive — cris et fracas d'armes en zoomant près d'un combat, émetteurs par front de mêlée,
couches selon la distance caméra. S'appuie sur AU1 (`docs/wip/au1-audio.md`,
`game/scripts/audio/battle_audio.gd`) ; ne touche pas `battle_voices.gd` ni `battle_speech.gd`.

## État

- [x] Sources CC0 repérées et vérifiées page par page (licence CC0 1.0 uniquement) : 27 nouveaux
      sons Freesound (acier/acier, acier/bois, armure/maille, effort/rage, râles, chutes de
      corps/armure, chevaux, foule de mêlée). Détail : `game/assets/audio/SOURCE.md`.
- [x] `tools/cent_ans_tools/audio_bank.py` : nouveaux `Clip` (12 acier/acier, 8 acier/bois,
      6 armure/maille, 10 effort/rage, 10 râles, 6 chutes, 6 hennissements, 1 choc de charge de
      cavalerie synthétisé, 3e nappe de mêlée massive) ; 112 clips générés, `pytest
      tools/tests/test_audio_bank.py` OK.
- [x] `data/audio/sound_bank.json` + schéma : événements `armor_hit`, `effort_cry`, `body_fall`,
      `cavalry_charge_impact`, `arrow_flyby` ; `sword_clash`/`shield_bash`/`death_groan`/
      `horse_neigh` étendus ; nappe `melee_bed_3` ; section `fronts` (`max_emitters`, `near_m`,
      `mid_m`, `engaged_full`, `event_period_s`, `beds`).
- [x] `battle_audio.gd` (`_update_fronts`, `_front_emitter`, `_play_front_event`,
      `_pick_near_event`) : régiments en mêlée groupés en fronts (paires `id`/`target`), les
      `fronts.max_emitters` plus proches de la caméra reçoivent un émetteur 3D dédié dont le
      volume/densité suit l'effectif engagé et les pertes récentes (`_front_soldiers`). Sous
      `near_m` : chocs individuels tirés au hasard, pondérés, jamais deux fois de suite le même
      type (+ `SoundBank.pick_stream` qui évite de répéter le même fichier). Entre `near_m` et
      `mid_m` : une des 3 nappes massives (`fronts.beds`, une par rang de proximité) remplace les
      chocs, plus des chocs épars. Au-delà de `mid_m` : pas d'émetteur dédié, la nappe globale
      (`_update_beds`) et l'ambiance lointaine prennent le relais. Cavalerie : grondement qui
      enfle puis impact au début de la charge. Volée : sifflement additionnel programmé au-dessus
      de la caméra si sa trajectoire passe à moins de 55 m (`_maybe_flyby_over_camera`).
      `play_at(clip, position)` inchangée (API pour EP5).
- [x] Test headless `game/tests/ep4_audio_test.gd` : comptes de clips, front proche avec émetteur
      / front loin sans émetteur, pas de répétition immédiate d'un type de choc, budget de voix
      jamais dépassé (8 fronts denses), charge de cavalerie, sifflement au-dessus de la caméra.
- [x] `core/build.sh`, `godot --headless --path game --import`, test `ep4_audio_test.gd` et
      `au1_audio_test.gd` OK, `smoke.gd` OK (exit 0, aucune erreur de script). `git merge main`
      (fast-forward puis merge propre, aucun conflit). `pytest tools/tests/test_audio_bank.py`
      OK, `ruff check`/`ruff format --check` propres. `docs/budget.md` : ligne EP4 à 0 $.

## État : terminé

Reste à l'oreille (pas de session d'écoute ici) : régler `volume_db` des nouveaux événements et
la densité des fronts si trop chargé/creux en jeu réel ; EP5 peut ajouter tambours/trompettes via
`BattleAudio.play_at(clip, position)` sans toucher à `_update_fronts`.

## Décisions

- Ordres criés génériques (spec) : aucune source CC0 de cris de commandement sans langue
  identifiable trouvée (résultats en anglais/allemand explicites, ou hors-sujet). Non ajouté ;
  noté comme limite. Les cris de charge/guerre existants (AU1) couvrent déjà les ordres non
  verbaux à l'engagement.
- Choc de charge de cavalerie : pas de source CC0 dédiée trouvée (« horse charge impact »,
  « cavalry charge » infructueux) ; construit à partir de sources déjà vérifiées (galop
  `cavalry_bed`, chocs boucliers/épées) plutôt que d'un enregistrement dédié.

## Prochaine étape

Écrire les `Clip` dans `audio_bank.py`, lancer la génération, puis le regroupement par front dans
`battle_audio.gd`.
