// Lecteur de modèle : lit un fichier de modèle (JSON), l'envoie au moteur et le fait vivre (pièces mobiles, démonstration).
// Ce fichier est utilisé tel quel par l'application ET par la page HTML exportée : aucun calcul de dessin ici, tout est dans le moteur.
const BORNE = 2000;
const num = (v, min, max, defaut) => (typeof v === "number" && Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : defaut);
const couleur = (c, defaut = null) => (typeof c === "string" && /^#[0-9a-f]{6}$/i.test(c) ? c.toLowerCase() : defaut);
const GENRES = { boite: 0, cylindre: 1, prisme: 2 };
const DECORS = { rect: 0, cercle: 1, poly: 2, texte: 3, grille: 4 };
const CHAMPS_PIECE = {
  boite: ["w", "h", "d", "r", "bevel", "seg"], cylindre: ["r", "l", "axe", "bevel", "seg", null], prisme: ["prof", null, null, null, null, null],
};
const CHAMPS_DECOR = {
  rect: ["w", "h", "r", null], cercle: ["r", null, null, null], poly: [null, null, "ferme", null],
  texte: ["taille", "graisse", "ancre", "police"], grille: ["r", "pas", "colonnes", "lignes"],
};

/** Style d'affichage. « contours » : traits seuls sur fond uni (les faces cachent ce qui est derrière en reprenant la couleur du fond) ; « couleurs » : faces ombrées d'origine. */
export const STYLE_DEFAUT = { mode: "contours", fond: "#0d0e10", trait: "#c8cbd0", accent: "#ff6363", ep: 1.3, teinte: true };
export function lireStyle(s) {
  const d = STYLE_DEFAUT;
  return {
    mode: s?.mode === "couleurs" ? "couleurs" : "contours", fond: couleur(s?.fond, d.fond), trait: couleur(s?.trait, d.trait), accent: couleur(s?.accent, d.accent),
    ep: num(s?.ep, 0.3, 4, d.ep), teinte: typeof s?.teinte === "boolean" ? s.teinte : d.teinte,
  };
}
/** Couleur de fond effective. */
export const fondDe = (modele, style) => (style.mode === "contours" ? style.fond : modele.fond);
/** Feuille de style du dessin (le moteur ne fait que la géométrie ; l'apparence est ici, donc modifiable sans recalcul). */
export function cssIllustration(modele, style) {
  let css = "#mod path{vector-effect:non-scaling-stroke;stroke-linejoin:round;stroke-linecap:round}";
  if (style.mode !== "contours") return css + "#mod .m{stroke-width:.6}#mod .e{fill:none;stroke:rgba(40,24,4,.42);stroke-width:.8}";
  const { fond, trait, ep } = style;
  css += `#mod .m{fill:${fond}!important;stroke:${fond}!important;stroke-width:.6}#mod .e{fill:none;stroke:${trait};stroke-width:${ep}}`;
  css += `#mod g[data-p] path{fill:none!important;stroke:${trait}!important;stroke-width:${+(ep * 0.7).toFixed(2)};stroke-opacity:.6}#mod g[data-p] text{fill:${trait}!important;stroke:none!important;fill-opacity:.6}`;
  if (style.teinte) {
    const mobiles = new Set(modele.commandes.flatMap((c) => c.cibles.map((t) => t.cle)));
    for (const p of modele.pieces) if (p.cle && mobiles.has(p.cle))
      css += `#mod .e[data-p="${p.id}"]{stroke:${style.accent}}#mod g[data-p="${p.id}"] path{stroke:${style.accent}!important;stroke-opacity:.8}#mod g[data-p="${p.id}"] text{fill:${style.accent}!important}`;
  }
  return css;
}

/** Lit un modèle : tout est borné, tout ce qui est inconnu est ignoré, sinon une erreur claire. */
export function lireModele(texte) {
  const d = typeof texte === "string" ? JSON.parse(texte) : texte;
  if (d?.format !== "perspective-studio-modele") throw new Error("Ce fichier n'est pas un modèle Perspective Studio.");
  if (!Array.isArray(d.pieces) || d.pieces.length === 0 || d.pieces.length > 80) throw new Error("Le modèle doit contenir de 1 à 80 pièces.");
  let id = 1;
  const pieces = d.pieces.map((q, i) => {
    if (!(q?.genre in GENRES)) throw new Error(`Pièce ${i + 1} : type inconnu.`);
    const decors = (Array.isArray(q.decors) ? q.decors : []).slice(0, 60).flatMap((x) => {
      if (!(x?.genre in DECORS)) return [];
      return [{
        id: id++, genre: x.genre, cara: num(x.cara, 0, 5, 0) | 0, x: num(x.x, -BORNE, BORNE, 0), y: num(x.y, -BORNE, BORNE, 0), rot: num(x.rot, -360, 360, 0),
        p: Object.fromEntries(CHAMPS_DECOR[x.genre].filter(Boolean).map((k) => [k, num(x.p?.[k], -BORNE, BORNE, 0)])),
        fill: couleur(x.fill), trait: couleur(x.trait), ep: num(x.ep, 0, 20, 1),
        pts: x.genre === "poly" ? (Array.isArray(x.pts) ? x.pts : []).slice(0, 64).map((s) => [num(s?.[0], -BORNE, BORNE, 0), num(s?.[1], -BORNE, BORNE, 0)]) : [],
        texte: x.genre === "texte" ? String(x.texte ?? "").slice(0, 120) : "",
      }];
    });
    return {
      id: id++, cle: typeof q.cle === "string" ? q.cle.slice(0, 40) : null, nom: String(q.nom ?? "").slice(0, 40), genre: q.genre,
      pos: [0, 1, 2].map((k) => num(q.pos?.[k], -BORNE, BORNE, 0)), rot: [0, 1, 2].map((k) => num(q.rot?.[k], -360, 360, 0)),
      p: Object.fromEntries(CHAMPS_PIECE[q.genre].filter(Boolean).map((k) => [k, num(q.p?.[k], -BORNE, BORNE, 0)])),
      couleur: couleur(q.couleur, "#8a93a3"), aretes: [0, 1, 2].includes(q.aretes) ? q.aretes : 0,
      pts: q.genre === "prisme" ? (Array.isArray(q.pts) ? q.pts : []).slice(0, 64).map((s) => [num(s?.[0], -BORNE, BORNE, 0), num(s?.[1], -BORNE, BORNE, 0)]) : [],
      decors,
    };
  });
  const cles = new Set(pieces.map((p) => p.cle).filter(Boolean));
  const commandes = (Array.isArray(d.commandes) ? d.commandes : []).slice(0, 12).map((c, i) => {
    const min = num(c?.min, -BORNE, BORNE, 0), max = Math.max(min + 1e-6, num(c?.max, -BORNE, BORNE, 1));
    const cibles = (Array.isArray(c?.cibles) ? c.cibles : []).slice(0, 8)
      .filter((t) => cles.has(t?.cle) && (t.cible === "pos" || t.cible === "rot") && [0, 1, 2].includes(t.comp))
      .map((t) => ({ cle: t.cle, cible: t.cible, comp: t.comp, k: num(t.k, -1000, 1000, 1) }));
    return {
      id: String(c?.id ?? `c${i}`).slice(0, 30), nom: String(c?.nom ?? `Commande ${i + 1}`).slice(0, 40), unite: String(c?.unite ?? "").slice(0, 6),
      min, max, defaut: num(c?.defaut, min, max, min), pas: num(c?.pas, 0.01, 100, 1), cibles,
      demo: { periode: num(c?.demo?.periode, 0.5, 120, 4), phase: num(c?.demo?.phase, 0, 1, 0) },
    };
  });
  const l = d.lumiere ?? {};
  return {
    nom: String(d.nom ?? "Sans titre").slice(0, 60), pieces, commandes,
    lumiere: Array.isArray(l.dir) ? { dir: [0, 1, 2].map((k) => num(l.dir[k], -10, 10, 0)), ambiant: num(l.ambiant, 0, 1, 0.22) } : null,
    fond: couleur(d.fond, "#0a0b0d"), style: lireStyle(d.style),
    camera: { az: num(d.camera?.az, -180, 180, 35), el: num(d.camera?.el, -89, 89, 25), focale: num(d.camera?.focale, 14, 300, 50), zoom: num(d.camera?.zoom, 0.3, 5, 1) },
  };
}

/** Envoie le modèle au moteur ; `deltas` : pour chaque clé de pièce, décalages de position et de rotation (commandes). */
function envoyer(moteur, modele, deltas) {
  moteur.modeleVider();
  for (const p of modele.pieces) {
    const dl = deltas.get(p.cle) ?? { pos: [0, 0, 0], rot: [0, 0, 0] };
    const v = p.p;
    const [a, b, c, d, e, f] = CHAMPS_PIECE[p.genre].map((k) => (k ? v[k] : 0));
    moteur.modelePiece(GENRES[p.genre], p.id, ...p.pos.map((x, k) => x + dl.pos[k]), ...p.rot.map((x, k) => x + dl.rot[k]), a, b, c, d, e, f, parseInt(p.couleur.slice(1), 16), p.aretes);
    for (const [x, y] of p.pts) moteur.modelePoint(x, y);
    for (const dc of p.decors) {
      const [a1, b1, c1, d1] = CHAMPS_DECOR[dc.genre].map((k) => (k ? dc.p[k] : 0));
      const col = (h) => (h ? parseInt(h.slice(1), 16) : -1);
      moteur.modeleDecor(DECORS[dc.genre], dc.id, dc.cara, dc.x, dc.y, dc.rot, a1, b1, c1, d1, col(dc.fill), col(dc.trait), dc.ep);
      for (const [x, y] of dc.pts) moteur.modeleDecorPoint(x, y);
      for (const ch of dc.texte) moteur.modeleDecorChar(ch.codePointAt(0));
    }
  }
  if (modele.lumiere) moteur.modeleLumiere(...modele.lumiere.dir, modele.lumiere.ambiant);
}

/**
 * Fait vivre un modèle dans `scene`. `vue()` renvoie { az, el, focale, dist } (dist ≤ 0 : cadrage automatique).
 * Démonstration : chaque commande oscille entre son minimum et son maximum selon sa période.
 */
export function creerLecteur({ scene, moteur, modele, vue, vitesse = () => 1 }) {
  const valeurs = Object.fromEntries(modele.commandes.map((c) => [c.id, c.defaut]));
  const reduit = typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
  let tDemo = 0, enDemo = false, visible = true, raf = 0, dernier = 0, surChange = () => {};
  function rendre() {
    const deltas = new Map();
    for (const c of modele.commandes) for (const t of c.cibles) {
      const dl = deltas.get(t.cle) ?? { pos: [0, 0, 0], rot: [0, 0, 0] };
      dl[t.cible][t.comp] += valeurs[c.id] * t.k;
      deltas.set(t.cle, dl);
    }
    envoyer(moteur, modele, deltas);
    const v = vue();
    const svg = moteur.modeleRendre(v.az, v.el, v.dist, v.focale);
    if (svg) scene.innerHTML = svg;
    return !!svg;
  }
  function pas(maintenant) {
    raf = 0;
    if (!enDemo) return;
    const dt = Math.min(0.1, (maintenant - dernier) / 1000); dernier = maintenant;
    if (visible) {
      tDemo += dt * vitesse();
      for (const c of modele.commandes) {
        const { periode, phase } = c.demo;
        valeurs[c.id] = (c.min + c.max) / 2 + ((c.max - c.min) / 2) * Math.sin(2 * Math.PI * (tDemo / periode + phase));
      }
      rendre(); surChange();
    }
    raf = requestAnimationFrame(pas);
  }
  return {
    valeurs, rendre,
    /** Change la valeur d'une commande (bornée) et redessine. */
    regler(id, v) {
      const c = modele.commandes.find((x) => x.id === id); if (!c) return;
      valeurs[id] = num(v, c.min, c.max, c.defaut); rendre();
    },
    demo(actif) {
      enDemo = !!actif && !reduit;
      if (enDemo && !raf) { dernier = performance.now(); raf = requestAnimationFrame(pas); }
      return enDemo;
    },
    enDemo: () => enDemo, mouvementReduit: reduit,
    /** La démonstration se met en pause quand la scène n'est pas à l'écran. */
    surveiller(el) { if (typeof IntersectionObserver === "function") new IntersectionObserver((e) => { visible = e[0].isIntersecting; }, { threshold: 0.2 }).observe(el); },
    surChangement(f) { surChange = f; },
  };
}
