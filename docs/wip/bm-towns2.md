# bm-towns2 : villes de campagne au zoom proche (client Bevy)

Branche `bm/towns2` (fusion de bm/mats et bm/models). Dossiers : `client/src/campaign/{towns,markers}/`.

## État
- En cours : style des lieux `map.town_style` (`towns/style.rs`) : `maquette` par défaut comme Godot
  (aucune ville 1:1, ni ville emblématique 1:1, ni lointain : les maquettes / glb DN restent à toute distance).

## Prochaine étape
- Portée des maquettes à la distance du rig (Godot), hachage `String.hash`, onde de vent des bannières,
  matériaux blancs restants, dumps par distance (60/45/25/10) près de Paris.
