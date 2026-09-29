# NT6 c et d — petites suites (IA au repos, réaffectation des touches)

Branche `feat/nt6cd-leftovers`.

## État

- **d (touches)** : `game/scripts/ui/key_bindings.gd` (classe `KeyBindings`), réglage `input/bindings`
  (`settings.gd`), onglet « Commandes » de `settings_menu.gd` (bouton « Changer », capture, échange en
  cas de conflit, « Rétablir par défaut »). Test `game/tests/nt6cd_keys_test.gd` OK. Smoke à relancer
  avec la dylib (`core/build.sh`).
- **c (IA au repos)** : à faire. Mesure de référence `century_probe 464 1 2 3 4 5` (binaire de base
  dans le scratchpad) en cours.

## Notes

- Actions listées = celles de `ShortcutSheet.CAMPAIGN_SECTIONS`. `codex_pin_tooltip` et
  `tooltip_explore` (contextuelles, partagent T avec Technologies) sont exclues de la détection de conflit.
- Le préréglage AZERTY/QWERTY ne change que les libellés ; les réaffectations s'y ajoutent.
