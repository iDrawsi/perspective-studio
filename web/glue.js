// Liaison manuelle avec le moteur WebAssembly : aucune bibliothèque.
// Protocole : modele_rendre(...) renvoie une longueur ; out_ptr() donne l'adresse du texte SVG en mémoire.
// `source` : adresse du fichier .wasm, ou ses octets (page exportée : le moteur est intégré à la page).
export async function chargerMoteur(source = "engine.wasm") {
  let octets = source;
  if (typeof source === "string") {
    const reponse = await fetch(source);
    if (!reponse.ok) throw new Error(`Moteur introuvable (${reponse.status})`);
    octets = await reponse.arrayBuffer();
  }
  const { instance } = await WebAssembly.instantiate(octets, {});
  const { memory, out_ptr } = instance.exports;
  const x = instance.exports;
  const decoder = new TextDecoder();
  // la mémoire peut avoir grandi : on relit toujours memory.buffer après l'appel
  const lire = (longueur) => (longueur === 0 ? "" : decoder.decode(new Uint8Array(memory.buffer, out_ptr(), longueur)));
  return {
    modeleVider: () => x.modele_vider(),
    modelePiece: (...a) => x.modele_piece(...a),
    modelePoint: (px, py) => x.modele_point(px, py),
    modeleRendre: (az, el, dist, focaleMm) => lire(x.modele_rendre(az, el, dist, focaleMm)),
    modeleDecor: (...a) => x.modele_decor(...a),
    modeleDecorPoint: (px, py) => x.modele_decor_point(px, py),
    modeleDecorChar: (c) => x.modele_decor_char(c),
    modeleFace: (id, cara, quoi) => x.modele_face(id, cara, quoi),
    modeleErreur: () => [x.modele_erreur(), x.modele_erreur_piece()],
    modeleLumiere: (lx, ly, lz, ambiant) => x.modele_lumiere(lx, ly, lz, ambiant),
    modeleRayon: () => x.modele_rayon(),
    modeleDistanceAuto: (focaleMm) => x.modele_distance_auto(focaleMm),
    modeleStats: () => [x.modele_stats(0), x.modele_stats(1)],
  };
}
