// Éditeur de modèle : l'interface (liste de pièces, champs, sélection, annuler/rétablir, fichiers).
// Tout le calcul (faces, découpe BSP, caméra, dessin) est fait par le moteur Rust ; ici on ne garde que l'état et l'interface.
import { chargerMoteur } from "./glue.js";

// ---------- Types de pièces ----------
// champs : [clé, libellé, min, max, pas]
const GENRES = {
  boite: {
    nom: "Boîte", num: 0,
    champs: [["w", "Largeur", 1, 400, 1], ["h", "Hauteur", 1, 400, 1], ["d", "Profondeur", 1, 400, 1],
             ["r", "Rayon des coins", 0, 100, 0.5], ["bevel", "Bevel (chanfrein)", 0, 30, 0.5], ["seg", "Segments des coins", 1, 24, 1]],
    defaut: { w: 60, h: 40, d: 30, r: 4, bevel: 2, seg: 4 },
  },
  cylindre: {
    nom: "Cylindre", num: 1, axe: true,
    champs: [["r", "Rayon", 1, 200, 0.5], ["l", "Longueur", 1, 400, 1], ["bevel", "Bevel (chanfrein)", 0, 30, 0.5], ["seg", "Segments", 3, 64, 1]],
    defaut: { r: 12, l: 40, axe: 2, bevel: 1, seg: 24 },
  },
  prisme: {
    nom: "Prisme", num: 2, contour: true,
    champs: [["prof", "Profondeur", 1, 400, 1]],
    defaut: { prof: 20 },
  },
};
const CONTOUR_DEFAUT = [[-20, -15], [20, -15], [20, 15], [-20, 15]];
const BORNE = 2000;

const argsPiece = (p) => {
  const v = p.p;
  const [a, b, c, d, e, f] = p.genre === "boite" ? [v.w, v.h, v.d, v.r, v.bevel, v.seg]
    : p.genre === "cylindre" ? [v.r, v.l, v.axe, v.bevel, v.seg, 0] : [v.prof, 0, 0, 0, 0, 0];
  return [GENRES[p.genre].num, p.id, ...p.pos, ...p.rot, a, b, c, d, e, f, parseInt(p.couleur.slice(1), 16), p.aretes];
};

// ---------- État ----------
let modele = { nom: "Sans titre", pieces: [] };
let selection = null;
let selDecor = null;                   // décor sélectionné (de la pièce sélectionnée)
let prochainId = 1;
let sale = true;                       // le modèle a changé : il faut le renvoyer au moteur
let historique = [], futur = [];
const nouvelleId = () => prochainId++;
const piece = (genre, nom, pos, rot, p, couleur = "#8a93a3", aretes = 0, pts = null, decors = []) =>
  ({ id: nouvelleId(), nom, genre, pos: [...pos], rot: [...rot], p: { ...GENRES[genre].defaut, ...p }, couleur, aretes, pts: genre === "prisme" ? (pts ?? CONTOUR_DEFAUT.map((q) => [...q])) : [], decors: decors.map((d) => ({ ...d, id: nouvelleId() })) });

// ---------- Décors : dessins à plat sur les faces ----------
const CARAS = {
  boite: [[0, "Devant"], [1, "Derrière"], [2, "Droite"], [3, "Gauche"], [4, "Dessous"], [5, "Dessus"]],
  cylindre: [[0, "Extrémité avant (z+)"], [1, "Extrémité arrière (z−)"]],
  prisme: [[0, "Face avant (z+)"], [1, "Face arrière (z−)"]],
};
const TYPES_DECOR = { rect: ["Rectangle", 0], cercle: ["Cercle", 1], poly: ["Ligne / polygone", 2], texte: ["Texte", 3] };
const DEFAUT_DECOR = {
  rect: { p: { w: 20, h: 10, r: 2 }, fill: "#4d5565", trait: "#9aa3b2", ep: 1 },
  cercle: { p: { r: 8 }, fill: null, trait: "#9aa3b2", ep: 1 },
  poly: { p: { ferme: 0 }, fill: null, trait: "#9aa3b2", ep: 1.5, pts: [[-12, 0], [12, 0]] },
  texte: { p: { taille: 8, graisse: 400, ancre: 1, police: 0 }, fill: "#e8eaee", trait: null, ep: 0, texte: "Texte" },
};
/** Décor prêt à l'emploi (les valeurs de `dec` l'emportent sur les valeurs par défaut du type). */
const decor = (genre, cara, x, y, dec = {}) => {
  const b = DEFAUT_DECOR[genre];
  return { genre, cara, x, y, rot: 0, ...JSON.parse(JSON.stringify(b)), ...dec, p: { ...b.p, ...(dec.p ?? {}) } };
};

const EXEMPLES = {
  telecommande: () => ({ nom: "Télécommande", pieces: [
    piece("boite", "Corps", [0, 0, 0], [0, 0, 0], { w: 56, h: 150, d: 22, r: 10, bevel: 3, seg: 6 }, "#6f7683", 0, null, [
      decor("texte", 0, 0, 66, { texte: "REMOTE", p: { taille: 6, graisse: 700, ancre: 1 }, fill: "#2a2e36" }),
      decor("poly", 0, 0, 0, { pts: [[-21, -62], [21, -62]], trait: "#2a2e36", ep: 1 }),
      decor("rect", 0, 0, -57, { p: { w: 30, h: 8, r: 2 }, fill: "#1c1f26", trait: "#2a2e36" }),
    ]),
    piece("cylindre", "Bouton 1", [0, -42, 11], [0, 0, 0], { r: 9, l: 8, axe: 2, bevel: 1.5, seg: 24 }, "#a05a5a", 0, null, [decor("texte", 0, 0, 3, { texte: "A", p: { taille: 10, graisse: 700 }, fill: "#ffffff" })]),
    piece("cylindre", "Bouton 2", [-14, -14, 11], [0, 0, 0], { r: 6, l: 7, axe: 2, bevel: 1, seg: 20 }, "#8a93a3", 0, null, [decor("texte", 0, 0, 2.4, { texte: "B", p: { taille: 7, graisse: 700 }, fill: "#2a2e36" })]),
    piece("cylindre", "Bouton 3", [14, -14, 11], [0, 0, 0], { r: 6, l: 7, axe: 2, bevel: 1, seg: 20 }, "#8a93a3", 0, null, [decor("texte", 0, 0, 2.4, { texte: "C", p: { taille: 7, graisse: 700 }, fill: "#2a2e36" })]),
    piece("boite", "Cadran", [0, 35, 11], [0, 0, 0], { w: 36, h: 36, d: 6, r: 18, bevel: 1.5, seg: 12 }, "#4d5565", 0, null, [
      decor("cercle", 0, 0, 0, { p: { r: 13 }, fill: null, trait: "#9aa3b2", ep: 1 }),
      decor("poly", 0, 0, 0, { pts: [[0, 0], [0, -11]], trait: "#e8eaee", ep: 1.5 }),
    ]),
    piece("prisme", "Antenne", [0, -84, 0], [0, 0, 0], { prof: 8 }, "#555d6b", 0, [[-5, -12], [5, -12], [3, 12], [-3, 12]]),
  ] }),
  traversent: () => ({ nom: "Pièces qui se traversent", pieces: [
    piece("boite", "Boîte", [0, 0, 0], [0, 0, 0], { w: 200, h: 120, d: 80, r: 12, bevel: 6, seg: 5 }, "#6f7683", 0, null, [
      decor("texte", 0, -60, -30, { texte: "BSP", p: { taille: 26, graisse: 700, ancre: 1 }, fill: "#e8eaee" }),
      decor("poly", 0, 0, 40, { pts: [[-80, 0], [80, 0]], trait: "#e8eaee", ep: 1.5 }),
    ]),
    piece("cylindre", "Tige", [60, 0, 40], [0, 0, 0], { r: 25, l: 90, axe: 2, bevel: 3, seg: 24 }, "#a05a5a"),
    piece("prisme", "Pointe", [-70, -60, 0], [0, 0, 0], { prof: 40 }, "#5a8a6a", 0, [[-20, 0], [20, 0], [0, 50]]),
    piece("boite", "Cube tourné", [10, 20, 0], [20, 35, 15], { w: 60, h: 60, d: 60, r: 0, bevel: 0, seg: 1 }, "#8a7a4a", 1, null, [
      decor("cercle", 0, 0, 0, { p: { r: 20 }, fill: null, trait: "#2a2e36", ep: 2 }),
      decor("texte", 0, 0, 7, { texte: "3D", p: { taille: 20, graisse: 700, ancre: 1 }, fill: "#2a2e36" }),
    ]),
  ] }),
};

// ---------- Éléments ----------
const $ = (id) => document.getElementById(id);
const scene = $("scene"), alerte = $("alerte"), info = $("info"), msg = $("msg"), liste = $("liste"), form = $("form");
const cam = { az: $("az"), el: $("el"), focale: $("focale"), auto: $("auto"), dist: $("dist") };
const courante = () => modele.pieces.find((p) => p.id === selection) ?? null;
const dire = (t) => { msg.textContent = t; };

// ---------- Historique ----------
const instantane = () => JSON.stringify({ modele, selection, prochainId });
const majBoutonsHistorique = () => { $("annuler").disabled = historique.length < 2; $("refaire").disabled = futur.length === 0; };
function valider() {
  const s = instantane();
  if (historique[historique.length - 1] === s) return;
  historique.push(s); if (historique.length > 100) historique.shift();
  futur = [];
  majBoutonsHistorique();
  try { localStorage.setItem("perspective-studio-modele", s); } catch { /* stockage indisponible : sans effet */ }
}
function restaurer(s) {
  const d = JSON.parse(s); modele = d.modele; selection = d.selection; prochainId = d.prochainId; selDecor = null; sale = true;
  for (const p of modele.pieces) p.decors ??= [];   // sessions enregistrées avant les décors
  toutRedessiner();
}
function annuler() { if (historique.length < 2) return; futur.push(historique.pop()); restaurer(historique[historique.length - 1]); majBoutonsHistorique(); }
function refaire() { if (!futur.length) return; const s = futur.pop(); historique.push(s); restaurer(s); majBoutonsHistorique(); }

// ---------- Moteur ----------
let moteur = null;
function envoyer() {
  moteur.modeleVider();
  for (const p of modele.pieces) {
    moteur.modelePiece(...argsPiece(p));
    if (p.genre === "prisme") for (const [x, y] of p.pts) moteur.modelePoint(x, y);
    for (const d of p.decors) {
      const [a, b, c, e] = d.genre === "rect" ? [d.p.w, d.p.h, d.p.r, 0] : d.genre === "cercle" ? [d.p.r, 0, 0, 0]
        : d.genre === "poly" ? [0, 0, d.p.ferme, 0] : [d.p.taille, d.p.graisse, d.p.ancre, d.p.police];
      const col = (h) => (h ? parseInt(h.slice(1), 16) : -1);
      moteur.modeleDecor(TYPES_DECOR[d.genre][1], d.id, d.cara, d.x, d.y, d.rot, a, b, c, e, col(d.fill), col(d.trait), d.ep);
      if (d.genre === "poly") for (const [x, y] of d.pts) moteur.modeleDecorPoint(x, y);
      if (d.genre === "texte") for (const ch of d.texte) moteur.modeleDecorChar(ch.codePointAt(0));
    }
  }
  sale = false;
}
const MESSAGES = {
  1: (n) => `Dimensions invalides pour « ${n} ».`,
  2: (n) => `Le contour de « ${n} » n'est pas convexe : un prisme doit avoir un contour sans creux.`,
  3: () => "Aucune pièce : ajoute une boîte, un cylindre ou un prisme.",
};
function surligner() {
  scene.querySelectorAll(".sel").forEach((e) => e.classList.remove("sel"));
  if (selection !== null) scene.querySelectorAll(`path.m[data-p="${selection}"], path.e[data-p="${selection}"]`).forEach((e) => e.classList.add("sel"));
  if (selDecor !== null) scene.querySelectorAll(`[data-d="${selDecor}"]`).forEach((e) => e.classList.add("sel"));
}
function rendu() {
  if (!moteur) return;
  if (sale) envoyer();
  const mm = +cam.focale.value;
  const rb = moteur.modeleRayon();
  if (rb > 0) {
    cam.dist.min = Math.ceil((rb * 1.05) / 5) * 5;
    cam.dist.max = Math.max(6000, Math.ceil(rb * 12));
    if (cam.auto.checked) cam.dist.value = Math.min(+cam.dist.max, Math.round(moteur.modeleDistanceAuto(mm)));
  }
  cam.dist.disabled = cam.auto.checked;
  $("o-az").textContent = cam.az.value + "°"; $("o-el").textContent = cam.el.value + "°";
  const champ = (2 * Math.atan(18 / mm) * 180) / Math.PI;
  $("o-focale").textContent = `${mm} mm · ${champ.toFixed(0)}°`; $("o-dist").textContent = cam.dist.value;
  const svg = moteur.modeleRendre(+cam.az.value, +cam.el.value, cam.auto.checked ? 0 : +cam.dist.value, mm);
  if (!svg) {
    const [code, id] = moteur.modeleErreur();
    alerte.textContent = (MESSAGES[code] ?? (() => "Rendu impossible."))(modele.pieces.find((p) => p.id === id)?.nom ?? "pièce");
    if (code === 3) { scene.innerHTML = ""; info.textContent = ""; }
    return;
  }
  alerte.textContent = "";
  scene.innerHTML = svg;
  surligner();
  const [faces, fragments] = moteur.modeleStats();
  info.textContent = `${modele.pieces.length} pièce${modele.pieces.length > 1 ? "s" : ""} · ${faces} faces · ${fragments} fragments après découpe`;
}

// ---------- Liste et formulaire ----------
function dessinerListe() {
  liste.replaceChildren(...modele.pieces.map((p) => {
    const b = document.createElement("button");
    b.type = "button"; b.setAttribute("aria-pressed", String(p.id === selection));
    const i = document.createElement("i"); i.style.background = p.couleur;
    const t = document.createElement("span"); t.textContent = p.nom;
    const s = document.createElement("small"); s.textContent = GENRES[p.genre].nom;
    b.append(i, t, s);
    b.addEventListener("click", () => selectionner(p.id));
    return b;
  }));
  $("dupliquer").disabled = $("supprimer").disabled = selection === null;
}
function selectionner(id, decorId = null) { if (id !== selection) selDecor = null; selection = id; if (decorId !== null) selDecor = decorId; dessinerListe(); dessinerForm(); surligner(); valider(); }

const el = (tag, attrs = {}, ...enfants) => { const e = Object.assign(document.createElement(tag), attrs); e.append(...enfants); return e; };
/** Champ à curseur + saisie numérique. `lire`/`ecrire` accèdent à la valeur dans le modèle. */
function champNombre(libelle, lire, ecrire, min, max, pas) {
  const sortie = el("output"); const curseur = el("input", { type: "range", min, max, step: pas }); const saisie = el("input", { type: "number", min, max, step: pas });
  const maj = (v) => { curseur.value = v; saisie.value = v; sortie.textContent = v; };
  maj(lire());
  const appliquer = (brut) => { const v = Math.min(Math.max(+brut, min), max); if (!Number.isFinite(v)) return; ecrire(v); maj(v); sale = true; rendu(); };
  curseur.addEventListener("input", () => appliquer(curseur.value));
  saisie.addEventListener("input", () => { if (saisie.value !== "") appliquer(saisie.value); });
  curseur.addEventListener("change", valider); saisie.addEventListener("change", valider);
  return el("label", {}, el("span", { className: "ligne" }, libelle, sortie), curseur, saisie);
}
function triplet(libelle, tab, min, max, pas, noms = ["x", "y", "z"]) {
  const champs = tab.map((_, i) => {
    const saisie = el("input", { type: "number", min, max, step: pas, value: tab[i] });
    saisie.addEventListener("input", () => { const v = Math.min(Math.max(+saisie.value, min), max); if (saisie.value !== "" && Number.isFinite(v)) { tab[i] = v; sale = true; rendu(); } });
    saisie.addEventListener("change", valider);
    return el("label", {}, el("span", {}, noms[i]), saisie);
  });
  return el("div", {}, el("p", { className: "aide" }, libelle), el("div", { className: "trio" }, ...champs));
}
function dessinerForm() {
  const p = courante();
  $("piece").disabled = false;
  if (!p) { form.replaceChildren(el("p", { className: "aide" }, "Aucune pièce sélectionnée. Clique sur le dessin ou sur la liste.")); return; }
  const g = GENRES[p.genre];
  const nom = el("input", { type: "text", value: p.nom, maxLength: 40 });
  nom.addEventListener("input", () => { p.nom = nom.value || g.nom; dessinerListe(); });
  nom.addEventListener("change", valider);
  const couleur = el("input", { type: "color", value: p.couleur });
  couleur.addEventListener("input", () => { p.couleur = couleur.value; sale = true; dessinerListe(); rendu(); });
  couleur.addEventListener("change", valider);
  const aretes = el("select", {}, ...[["Contours et arêtes vives", 0], ["Toutes les arêtes", 1], ["Contours seulement", 2]].map(([t, v]) => el("option", { value: v, selected: p.aretes === v }, t)));
  aretes.addEventListener("change", () => { p.aretes = +aretes.value; sale = true; rendu(); valider(); });
  const blocs = [
    el("label", {}, "Nom", nom),
    triplet("Position (x vers la droite, y vers le bas, z vers toi)", p.pos, -BORNE, BORNE, 1),
    triplet("Rotation (degrés)", p.rot, -360, 360, 5),
    ...g.champs.map(([cle, lib, min, max, pas]) => champNombre(lib, () => p.p[cle], (v) => { p.p[cle] = v; }, min, max, pas)),
  ];
  if (g.axe) {
    const axe = el("select", {}, ...[["x", 0], ["y", 1], ["z (vers toi)", 2]].map(([t, v]) => el("option", { value: v, selected: p.p.axe === v }, t)));
    axe.addEventListener("change", () => { p.p.axe = +axe.value; sale = true; rendu(); valider(); });
    blocs.push(el("label", {}, "Axe du cylindre", axe));
  }
  if (g.contour) {
    const zone = el("textarea", { value: p.pts.map((q) => q.join(", ")).join("\n"), spellcheck: false });
    const erreur = el("p", { className: "erreur", role: "alert" });
    zone.addEventListener("input", () => {
      const pts = [];
      for (const ligne of zone.value.split("\n")) {
        if (!ligne.trim()) continue;
        const [x, y] = ligne.split(/[,; ]+/).map(Number);
        if (!Number.isFinite(x) || !Number.isFinite(y) || Math.abs(x) > BORNE || Math.abs(y) > BORNE) { erreur.textContent = `Ligne ignorée : « ${ligne.trim()} » (deux nombres attendus, x puis y).`; continue; }
        pts.push([x, y]);
      }
      if (pts.length < 3) { erreur.textContent = "Il faut au moins 3 points."; return; }
      if (pts.length > 64) { erreur.textContent = "64 points au plus."; return; }
      erreur.textContent = ""; p.pts = pts; sale = true; rendu();
    });
    zone.addEventListener("change", valider);
    blocs.push(el("label", {}, "Contour (un point « x, y » par ligne, convexe)", zone), erreur);
  }
  blocs.push(el("label", {}, "Couleur", couleur), el("label", {}, "Arêtes tracées", aretes));
  blocs.push(sectionDecors(p));
  form.replaceChildren(...blocs);
}
// ---------- Décors de la pièce sélectionnée ----------
const couleurNum = (h) => (h ? parseInt(h.slice(1), 16) : -1);
/** Case à cocher + couleur (« aucun » quand décochée). */
function couleurFacultative(libelle, lire, ecrire, defaut) {
  const case_ = el("input", { type: "checkbox", checked: !!lire() }); const c = el("input", { type: "color", value: lire() || defaut });
  const maj = () => { ecrire(case_.checked ? c.value : null); c.disabled = !case_.checked; sale = true; rendu(); };
  c.disabled = !case_.checked;
  case_.addEventListener("change", () => { maj(); valider(); }); c.addEventListener("input", maj); c.addEventListener("change", valider);
  return el("label", {}, el("span", { className: "coche" }, case_, libelle), c);
}
function sectionDecors(p) {
  const caras = CARAS[p.genre];
  const d = p.decors.find((x) => x.id === selDecor) ?? null;
  const refaire = () => { dessinerForm(); sale = true; rendu(); valider(); };
  const liste = el("div", { className: "liste", id: "liste-decors", role: "group", "aria-label": "Décors de la pièce" }, ...p.decors.map((x, i) => {
    const b = el("button", { type: "button" }, el("span", {}, `${i + 1}. ${TYPES_DECOR[x.genre][0]}${x.genre === "texte" ? " « " + x.texte.slice(0, 14) + " »" : ""}`), el("small", {}, caras.find(([c]) => c === x.cara)?.[1] ?? "?"));
    b.setAttribute("aria-pressed", String(x.id === selDecor));
    b.addEventListener("click", () => { selDecor = x.id; dessinerForm(); surligner(); });
    return b;
  }));
  const ajouts = el("div", { className: "rang" }, ...Object.entries(TYPES_DECOR).map(([g, [nom]]) => {
    const b = el("button", { type: "button" }, "+ " + nom);
    b.addEventListener("click", () => { const n = decor(g, caras[0][0], 0, 0); n.id = nouvelleId(); p.decors.push(n); selDecor = n.id; refaire(); });
    return b;
  }));
  const blocs = [el("p", { className: "aide" }, "Décors : dessins à plat sur les faces de la pièce. Le dernier de la liste est dessus. Ils suivent la pièce et ne dépassent pas de leur face."), liste, ajouts];
  if (d) blocs.push(formDecor(p, d, refaire));
  return el("fieldset", {}, el("legend", {}, "Décors"), ...blocs);
}
function formDecor(p, d, refaire) {
  const caras = CARAS[p.genre];
  const face = el("select", {}, ...caras.map(([v, t]) => el("option", { value: v, selected: d.cara === v }, t)));
  const indice = el("p", { className: "aide" });
  const tailleFace = () => {
    if (sale) envoyer();
    const w = moteur.modeleFace(p.id, d.cara, 0), h = moteur.modeleFace(p.id, d.cara, 1);
    indice.textContent = w > 0 ? `Face : ${w.toFixed(1)} × ${h.toFixed(1)} (origine au centre, x à droite, y vers le bas).` : "Cette face n'existe pas pour cette pièce.";
  };
  tailleFace();
  face.addEventListener("change", () => { d.cara = +face.value; tailleFace(); sale = true; rendu(); valider(); });
  const blocs = [el("label", {}, "Face", face), indice,
    champNombre("Position x", () => d.x, (v) => { d.x = v; }, -500, 500, 0.5),
    champNombre("Position y", () => d.y, (v) => { d.y = v; }, -500, 500, 0.5),
    champNombre("Rotation (degrés)", () => d.rot, (v) => { d.rot = v; }, -180, 180, 1)];
  if (d.genre === "rect") blocs.push(champNombre("Largeur", () => d.p.w, (v) => { d.p.w = v; }, 0.5, 500, 0.5), champNombre("Hauteur", () => d.p.h, (v) => { d.p.h = v; }, 0.5, 500, 0.5), champNombre("Rayon des coins", () => d.p.r, (v) => { d.p.r = v; }, 0, 100, 0.5));
  if (d.genre === "cercle") blocs.push(champNombre("Rayon", () => d.p.r, (v) => { d.p.r = v; }, 0.5, 300, 0.5));
  if (d.genre === "poly") {
    const zone = el("textarea", { value: d.pts.map((q) => q.join(", ")).join("\n"), spellcheck: false });
    const ferme = el("input", { type: "checkbox", checked: d.p.ferme > 0.5 });
    zone.addEventListener("input", () => {
      const pts = zone.value.split("\n").map((l) => l.split(/[,; ]+/).map(Number)).filter((q) => q.length >= 2 && q.every(Number.isFinite) && q.every((v) => Math.abs(v) <= BORNE)).map((q) => [q[0], q[1]]).slice(0, 64);
      if (pts.length >= 2) { d.pts = pts; sale = true; rendu(); }
    });
    zone.addEventListener("change", valider);
    ferme.addEventListener("change", () => { d.p.ferme = ferme.checked ? 1 : 0; sale = true; rendu(); valider(); });
    blocs.push(el("label", {}, "Points (un « x, y » par ligne, relatifs à la position)", zone), el("label", { className: "coche" }, ferme, "Polygone fermé"));
  }
  if (d.genre === "texte") {
    const t = el("input", { type: "text", value: d.texte, maxLength: 120 });
    t.addEventListener("input", () => { d.texte = t.value; sale = true; rendu(); });
    t.addEventListener("change", () => { valider(); dessinerForm(); });
    const choix = (cle, options) => { const sel = el("select", {}, ...options.map(([v, n]) => el("option", { value: v, selected: d.p[cle] === v }, n))); sel.addEventListener("change", () => { d.p[cle] = +sel.value; sale = true; rendu(); valider(); }); return sel; };
    blocs.push(el("label", {}, "Texte", t), champNombre("Taille", () => d.p.taille, (v) => { d.p.taille = v; }, 1, 200, 0.5),
      el("label", {}, "Graisse", choix("graisse", [[300, "Fine"], [400, "Normale"], [700, "Grasse"]])),
      el("label", {}, "Alignement", choix("ancre", [[0, "Début au point"], [1, "Centré sur le point"], [2, "Fin au point"]])),
      el("label", {}, "Police", choix("police", [[0, "Sans empattement"], [1, "Chasse fixe"]])));
  }
  blocs.push(couleurFacultative("Remplissage", () => d.fill, (v) => { d.fill = v; }, "#4d5565"));
  blocs.push(couleurFacultative("Trait", () => d.trait, (v) => { d.trait = v; }, "#9aa3b2"));
  if (d.genre !== "texte" || d.trait) blocs.push(champNombre("Épaisseur du trait", () => d.ep, (v) => { d.ep = v; }, 0, 20, 0.1));
  const i = p.decors.indexOf(d);
  const act = (nom, f, off = false) => { const b = el("button", { type: "button", disabled: off }, nom); b.addEventListener("click", f); return b; };
  blocs.push(el("div", { className: "rang" },
    act("Monter", () => { [p.decors[i - 1], p.decors[i]] = [p.decors[i], p.decors[i - 1]]; refaire(); }, i === 0),
    act("Descendre", () => { [p.decors[i + 1], p.decors[i]] = [p.decors[i], p.decors[i + 1]]; refaire(); }, i === p.decors.length - 1),
    act("Dupliquer", () => { const c = JSON.parse(JSON.stringify(d)); c.id = nouvelleId(); c.x += 5; c.y += 5; p.decors.push(c); selDecor = c.id; refaire(); }),
    act("Supprimer", () => { p.decors.splice(i, 1); selDecor = p.decors[Math.min(i, p.decors.length - 1)]?.id ?? null; refaire(); })));
  return el("div", {}, ...blocs);
}

function toutRedessiner() { dessinerListe(); dessinerForm(); $("nom-modele").value = modele.nom; sale = true; rendu(); }

// ---------- Actions ----------
function ajouter(genre) {
  const n = modele.pieces.filter((p) => p.genre === genre).length + 1;
  const k = modele.pieces.length;
  const p = piece(genre, `${GENRES[genre].nom} ${n}`, [(k % 4) * 15, -(k % 4) * 10, 0], [0, 0, 0], {});
  modele.pieces.push(p); selection = p.id; sale = true;
  dessinerListe(); dessinerForm(); rendu(); valider();
}
function supprimer() {
  if (selection === null) return;
  const i = modele.pieces.findIndex((p) => p.id === selection);
  modele.pieces.splice(i, 1);
  selection = modele.pieces[Math.min(i, modele.pieces.length - 1)]?.id ?? null;
  sale = true; dessinerListe(); dessinerForm(); rendu(); valider();
}
function dupliquer() {
  const p = courante(); if (!p) return;
  const c = JSON.parse(JSON.stringify(p)); c.id = nouvelleId(); c.nom = p.nom + " copie"; c.pos[0] += 12; c.decors.forEach((d) => { d.id = nouvelleId(); });
  modele.pieces.push(c); selection = c.id; sale = true;
  dessinerListe(); dessinerForm(); rendu(); valider();
}
function remplacer(m) { modele = m; selection = m.pieces[0]?.id ?? null; sale = true; toutRedessiner(); valider(); }

// ---------- Fichiers ----------
const num = (v, min, max, defaut) => (typeof v === "number" && Number.isFinite(v) ? Math.min(Math.max(v, min), max) : defaut);
/** Lit un fichier de modèle ; ne garde que des valeurs valides et borne tout (un fichier n'est jamais cru sur parole). */
function lireModele(texte) {
  const d = JSON.parse(texte);
  if (d?.format !== "perspective-studio-modele" || !Array.isArray(d.pieces)) throw new Error("Ce fichier n'est pas un modèle Perspective Studio.");
  if (d.pieces.length > 200) throw new Error("Trop de pièces (200 au plus).");
  const sauve = prochainId; prochainId = 1;
  try {
    const pieces = d.pieces.map((q, i) => {
      const g = GENRES[q?.genre];
      if (!g) throw new Error(`Pièce ${i + 1} : type inconnu.`);
      const p = {};
      for (const [cle, , min, max] of g.champs) p[cle] = num(q.p?.[cle], min, max, g.defaut[cle]);
      if (g.axe) p.axe = [0, 1, 2].includes(q.p?.axe) ? q.p.axe : 2;
      const pts = g.contour && Array.isArray(q.pts) ? q.pts.slice(0, 64).map((s) => [num(s?.[0], -BORNE, BORNE, 0), num(s?.[1], -BORNE, BORNE, 0)]) : null;
      return {
        id: nouvelleId(), nom: String(q.nom ?? g.nom).slice(0, 40), genre: q.genre,
        pos: [0, 1, 2].map((k) => num(q.pos?.[k], -BORNE, BORNE, 0)), rot: [0, 1, 2].map((k) => num(q.rot?.[k], -360, 360, 0)),
        p: { ...g.defaut, ...p }, couleur: /^#[0-9a-f]{6}$/i.test(q.couleur) ? q.couleur.toLowerCase() : "#8a93a3",
        aretes: [0, 1, 2].includes(q.aretes) ? q.aretes : 0, pts: g.contour ? (pts && pts.length >= 3 ? pts : CONTOUR_DEFAUT.map((s) => [...s])) : [],
        decors: (Array.isArray(q.decors) ? q.decors : []).slice(0, 60).map((x) => lireDecor(x, q.genre)).filter(Boolean),
      };
    });
    return { nom: String(d.nom ?? "Sans titre").slice(0, 40), pieces };
  } catch (e) { prochainId = sauve; throw e; }
}
const couleurOuNul = (c) => (typeof c === "string" && /^#[0-9a-f]{6}$/i.test(c) ? c.toLowerCase() : null);
/** Un décor lu dans un fichier : type connu, face existante, tout borné ; sinon ignoré. */
function lireDecor(x, genrePiece) {
  const b = DEFAUT_DECOR[x?.genre];
  if (!b) return null;
  const cara = CARAS[genrePiece].some(([c]) => c === x.cara) ? x.cara : CARAS[genrePiece][0][0];
  const d = { id: nouvelleId(), genre: x.genre, cara, x: num(x.x, -BORNE, BORNE, 0), y: num(x.y, -BORNE, BORNE, 0), rot: num(x.rot, -360, 360, 0),
    p: { ...b.p }, fill: couleurOuNul(x.fill), trait: couleurOuNul(x.trait), ep: num(x.ep, 0, 20, b.ep) };
  const lim = { w: [0.5, 500], h: [0.5, 500], r: [0, 500], taille: [1, 200], graisse: [100, 900], ancre: [0, 2], police: [0, 1], ferme: [0, 1] };
  for (const k of Object.keys(b.p)) d.p[k] = num(x.p?.[k], ...lim[k], b.p[k]);
  if (x.genre === "poly") { const pts = (Array.isArray(x.pts) ? x.pts : []).slice(0, 64).map((q) => [num(q?.[0], -BORNE, BORNE, 0), num(q?.[1], -BORNE, BORNE, 0)]); d.pts = pts.length >= 2 ? pts : b.pts.map((q) => [...q]); }
  if (x.genre === "texte") d.texte = String(x.texte ?? "").slice(0, 120);
  return d;
}
function telecharger(nom, texte, type) {
  const url = URL.createObjectURL(new Blob([texte], { type }));
  const a = Object.assign(document.createElement("a"), { href: url, download: nom });
  document.body.appendChild(a); a.click(); a.remove(); setTimeout(() => URL.revokeObjectURL(url), 5000);
}
const nomFichier = () => (modele.nom || "modele").normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^a-z0-9]+/gi, "-").replace(/^-|-$/g, "").toLowerCase() || "modele";

// ---------- Plein écran, panneau, taille de fenêtre ----------
const ajuste = () => { const r = document.documentElement.style; r.setProperty("--h", innerHeight + "px"); r.setProperty("--w", innerWidth + "px"); };
const majBoutons = () => {
  $("plein").textContent = document.fullscreenElement ? "Quitter le plein écran (F)" : "Plein écran (F)";
  const cache = document.body.classList.contains("sans-panneau");
  $("panneau").textContent = cache ? "Afficher le panneau (H)" : "Masquer le panneau (H)"; $("panneau").setAttribute("aria-pressed", String(cache));
};
const basculePlein = async () => { try { if (document.fullscreenElement) await document.exitFullscreen(); else await document.documentElement.requestFullscreen(); } catch { dire("Plein écran refusé par le navigateur."); } };
const basculePanneau = () => { document.body.classList.toggle("sans-panneau"); majBoutons(); };
if (!document.fullscreenEnabled) $("plein").hidden = true;
$("plein").addEventListener("click", basculePlein); $("panneau").addEventListener("click", basculePanneau);
document.addEventListener("fullscreenchange", () => { majBoutons(); for (const ms of [0, 150, 400, 900, 1800]) setTimeout(ajuste, ms); });
addEventListener("resize", ajuste); window.visualViewport?.addEventListener("resize", ajuste); ajuste(); majBoutons();

document.addEventListener("keydown", (e) => {
  const saisie = /^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement?.tagName) && document.activeElement.type !== "range" && document.activeElement.type !== "checkbox";
  const mod = e.ctrlKey || e.metaKey;
  if (mod && !e.altKey && e.key.toLowerCase() === "z" && !saisie) { e.preventDefault(); e.shiftKey ? refaire() : annuler(); return; }
  if (mod && !e.altKey && e.key.toLowerCase() === "y" && !saisie) { e.preventDefault(); refaire(); return; }
  if (mod || e.altKey || saisie) return;
  if (e.key === "f" || e.key === "F") { e.preventDefault(); basculePlein(); }
  if (e.key === "h" || e.key === "H") { e.preventDefault(); basculePanneau(); }
  if (e.key === "Delete" || e.key === "Backspace") { e.preventDefault(); supprimer(); }
});

// ---------- Démarrage ----------
try {
  moteur = await chargerMoteur();
} catch (e) { alerte.textContent = "Moteur introuvable : " + e.message; throw e; }

const NOMS_EXEMPLES = { telecommande: "Télécommande", traversent: "Pièces qui se traversent" };
for (const [cle, nom] of Object.entries(NOMS_EXEMPLES)) $("exemple").append(el("option", { value: cle }, nom));
$("exemple").addEventListener("change", (e) => { if (e.target.value) { remplacer(EXEMPLES[e.target.value]()); dire("Exemple chargé."); } e.target.value = ""; });
document.querySelectorAll("[data-ajout]").forEach((b) => b.addEventListener("click", () => ajouter(b.dataset.ajout)));
$("supprimer").addEventListener("click", supprimer); $("dupliquer").addEventListener("click", dupliquer);
$("annuler").addEventListener("click", annuler); $("refaire").addEventListener("click", refaire);
$("nouveau").addEventListener("click", () => { remplacer({ nom: "Sans titre", pieces: [] }); dire("Nouveau modèle."); });
$("nom-modele").addEventListener("input", (e) => { modele.nom = e.target.value; }); $("nom-modele").addEventListener("change", valider);
$("enregistrer").addEventListener("click", () => {
  const d = { format: "perspective-studio-modele", version: 1, nom: modele.nom, pieces: modele.pieces.map(({ id, decors, ...reste }) => ({ ...reste, decors: decors.map(({ id: _i, ...dd }) => dd) })) };
  telecharger(nomFichier() + ".json", JSON.stringify(d, null, 2), "application/json"); dire(`${nomFichier()}.json enregistré.`);
});
$("svg").addEventListener("click", () => {
  const s = scene.innerHTML; if (!s) { dire("Rien à exporter."); return; }
  const propre = s.replace(/ sel"/g, '"').replace(/^(<svg[^>]*>)/, '$1<style>#mod path{vector-effect:non-scaling-stroke;stroke-linejoin:round;stroke-linecap:round}#mod .m{stroke-width:.6}#mod .e{fill:none;stroke:#6b6f78;stroke-width:1}</style><rect width="736" height="736" fill="#0a0b0d"/>');
  telecharger(nomFichier() + ".svg", `<?xml version="1.0" encoding="UTF-8"?>\n` + propre, "image/svg+xml"); dire(`${nomFichier()}.svg enregistré.`);
});
$("charger").addEventListener("change", async (e) => {
  const f = e.target.files?.[0]; if (!f) return;
  try { remplacer(lireModele(await f.text())); dire(`Modèle « ${modele.nom} » chargé (${modele.pieces.length} pièces).`); } catch (err) { dire("Fichier refusé : " + err.message); }
  e.target.value = "";
});
for (const c of [cam.az, cam.el, cam.focale, cam.dist]) c.addEventListener("input", rendu);
cam.auto.addEventListener("change", rendu);
scene.addEventListener("click", (e) => {
  const dec = e.target.closest?.("[data-d]"), t = e.target.closest?.("[data-p]");
  selectionner(t ? +t.dataset.p : null, dec ? +dec.dataset.d : null);
});

// reprise de la dernière session, sinon un exemple
let repris = false;
try { const s = localStorage.getItem("perspective-studio-modele"); if (s) { restaurer(s); historique = [s]; repris = true; } } catch { /* ignoré */ }
if (!repris) { modele = EXEMPLES.telecommande(); selection = modele.pieces[0].id; toutRedessiner(); historique = [instantane()]; }
majBoutonsHistorique();
