# MU — musique de campagne plus calme (luth, harpe, instruments anciens)

Demande du joueur (2026-10-02) : la musique de campagne « n'est pas bonne » ; il veut du baroque
plus détendu, sans instrument moderne (harpe, luth…), pour que l'anachronisme ne s'entende pas.

## État

- [x] Recherche Wikimedia Commons (API, licence lue page à page) : 17 enregistrements réels
      retenus — luth (M. Tomsińska ×4), vihuela (J. M. Moreno), théorbe (Kapsberger),
      luth-clavecin (Bach, M. Goldstein ×4), clavicorde (Cabezón, J. Benson), clavecin (Gibbons),
      violes (Hume ×2, Moulinié), « Greensleeves » aux instruments anciens, tiento de Mudarra.
- [x] Manifeste `tools/cent_ans_tools/era_music.py` (`WIKIMEDIA_TRACKS`, licence `CC BY-SA 2.0`
      acceptée).
- [x] Téléchargement + conversion (`uv run --project tools python -m cent_ans_tools.era_music`).
- [x] `data/audio/music.json` : nouvelles pistes en `primary` des contextes `campaign`,
      `campaign_<région occidentale>` et `court` ; anciennes pistes rétrogradées en `fallback` ;
      `war` (carte en guerre) = trois pièces graves et calmes + liste régionale ; pièces martiales
      déplacées dans le nouveau contexte `battle`.
- [x] Tests (`pytest`, `smoke.gd`), `CREDITS.md`, ADR 0166, nouveau contexte `battle`
      (`BattleMusicDirector`).

## Écarté

- Rendus au synthétiseur de Pracchia-78 (Milano, Galilei, Narváez) : pas de synthé (bible § 9).
- Internet Archive : licences déclarées douteuses (albums commerciaux versés en « CC0 »).
- Harpe seule : rien de libre et d'époque sur Commons (seulement harpe moderne ou synthé).

## Points ouverts

- `era_music.py` : agent utilisateur descriptif obligatoire (Commons renvoie une page HTML à
  « Mozilla/5.0 ») ; relancer le script reprend les téléchargements manquants.
- `menu` inchangé (estampie, « Onques ne fut ») : à aligner si le joueur le demande.
- Jugement à l'oreille du joueur ; `mudarra_moreno_vihuela` est une prise de concert (bruits de
  salle possibles).
