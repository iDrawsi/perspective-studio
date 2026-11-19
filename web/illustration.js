// Page « Illustration » : charge un modèle (JSON), le montre avec ses commandes, la caméra et la démonstration, et l'exporte.
import { chargerMoteur } from "./glue.js";
import { lireModele, creerLecteur, lireStyle, cssIllustration, fondDe, STYLE_DEFAUT } from "./modele.js";
import { construireHtmlModele, svgDeLaScene, telecharger } from "./export.js";

const $ = (id) => document.getElementById(id);
const scene = $("scene"), msg = $("msg"), alerte = $("alerte");
const dire = (t) => { msg.textContent = t; };
const cam = { az: $("az"), el: $("el"), focale: $("focale"), zoom: $("zoom") };

let moteur, modele, texteModele, lecteur;
const sliders = new Map();

// distance = distance de cadrage automatique (la même quelle que soit la focale) divisée par le zoom
const vue = () => ({ az: +cam.az.value, el: +cam.el.value, focale: +cam.focale.value, dist: moteur.modeleDistanceAuto(+cam.focale.value) / +cam.zoom.value });
const nomFichier = () => (modele?.nom || "illustration").normalize("NFD").replace(/[̀-ͯ]/g, "").replace(/[^a-z0-9]+/gi, "-").replace(/^-|-$/g, "").toLowerCase() || "illustration";

function libelles() {
  const mm = +cam.focale.value;
  $("o-az").textContent = cam.az.value + "°"; $("o-el").textContent = cam.el.value + "°";
  $("o-focale").textContent = `${mm} mm · ${((2 * Math.atan(18 / mm) * 180) / Math.PI).toFixed(0)}°`;
  $("o-zoom").textContent = "× " + (+cam.zoom.value).toFixed(2);
  $("o-vitesse").textContent = "× " + $("vitesse").value;
  for (const c of modele.commandes) { const s = sliders.get(c.id); s.out.textContent = `${(+lecteur.valeurs[c.id]).toFixed(c.pas < 1 ? 1 : 0)} ${c.unite}`; if (document.activeElement !== s.input) s.input.value = lecteur.valeurs[c.id]; }
}
function dessiner() {
  if (!lecteur.rendre()) { alerte.textContent = "Rendu impossible : vérifie les dimensions des pièces."; return; }
  alerte.textContent = ""; libelles();
}

// ----- style (contours / couleurs, fond, teintes) : apparence seulement, le moteur n'est pas rappelé -----
const st = { mode: $("s-mode"), fond: $("s-fond"), trait: $("s-trait"), accent: $("s-accent"), teinte: $("s-teinte"), ep: $("s-ep") };
function styleCourant() { return { mode: st.mode.value, fond: modele.style.fond, trait: st.trait.value, accent: st.accent.value, teinte: st.teinte.checked, ep: +st.ep.value }; }
function appliquerStyle() {
  const s = modele.style = styleCourant();
  $("style-illustration").textContent = cssIllustration(modele, s);
  document.documentElement.style.setProperty("--fond", fondDe(modele, s));
  const contours = s.mode === "contours";
  for (const el of [st.trait, st.accent, st.teinte, st.ep]) el.disabled = !contours;
  st.fond.value = fondDe(modele, s); $("o-ep").textContent = s.ep.toFixed(1) + " px";
}
function montrerStyle() {
  const s = modele.style;
  st.mode.value = s.mode; st.trait.value = s.trait; st.accent.value = s.accent; st.teinte.checked = s.teinte; st.ep.value = s.ep;
  appliquerStyle();
}
st.fond.addEventListener("input", () => { if (st.mode.value === "contours") modele.style.fond = st.fond.value; else modele.fond = st.fond.value; appliquerStyle(); });
for (const el of [st.mode, st.trait, st.accent, st.teinte, st.ep]) el.addEventListener("input", appliquerStyle);
$("s-defaut").addEventListener("click", () => { modele.style = lireStyle(STYLE_DEFAUT); montrerStyle(); });
/** Texte du modèle avec le style en cours (pour les exports). */
const texteCourant = () => JSON.stringify({ ...JSON.parse(texteModele), fond: modele.fond, style: modele.style, camera: { az: +cam.az.value, el: +cam.el.value, focale: +cam.focale.value, zoom: +cam.zoom.value } }, null, 1);

function charger(texte) {
  const m = lireModele(texte);                       // lève une erreur claire si le fichier est refusé
  modele = m; texteModele = texte;
  $("titre").textContent = `${m.nom} — ${m.pieces.length} pièces, ${m.commandes.length} commande${m.commandes.length > 1 ? "s" : ""}`;
  cam.az.value = m.camera.az; cam.el.value = m.camera.el; cam.focale.value = m.camera.focale; cam.zoom.value = m.camera.zoom;
  lecteur?.demo(false);
  lecteur = creerLecteur({ scene, moteur, modele, vue, vitesse: () => +$("vitesse").value });
  lecteur.surChangement(libelles);
  lecteur.surveiller(scene);
  $("demo").checked = false; $("demo").disabled = lecteur.mouvementReduit;
  const zone = $("commandes"); zone.replaceChildren(); sliders.clear();
  for (const c of m.commandes) {
    const out = document.createElement("output"), ligne = document.createElement("span"), nom = document.createElement("span");
    ligne.className = "ligne"; nom.textContent = c.nom; ligne.append(nom, out);
    const input = Object.assign(document.createElement("input"), { type: "range", min: c.min, max: c.max, step: c.pas, value: c.defaut });
    input.addEventListener("input", () => { $("demo").checked = false; lecteur.demo(false); lecteur.regler(c.id, +input.value); libelles(); });
    const label = document.createElement("label"); label.append(ligne, input); zone.append(label);
    sliders.set(c.id, { input, out });
  }
  montrerStyle();
  dessiner();
}

// ----- démarrage -----
try { moteur = await chargerMoteur(); } catch (e) { alerte.textContent = "Moteur introuvable : " + e.message; throw e; }
try {
  const r = await fetch("modeles/television.json", { cache: "no-cache" });
  if (!r.ok) throw new Error(`modèle introuvable (${r.status})`);
  charger(await r.text());
} catch (e) { alerte.textContent = "Chargement impossible : " + e.message; throw e; }

for (const c of Object.values(cam)) c.addEventListener("input", dessiner);
$("vitesse").addEventListener("input", libelles);
$("demo").addEventListener("change", (e) => { e.target.checked = lecteur.demo(e.target.checked); });
$("charger").addEventListener("change", async (e) => {
  const f = e.target.files?.[0]; if (!f) return;
  try {
    if (f.size > 2_000_000) throw new Error("fichier trop gros");
    charger(await f.text()); dire(`Modèle « ${modele.nom} » chargé.`);
  } catch (err) { dire("Fichier refusé : " + err.message); }
  e.target.value = "";
});
$("export-json").addEventListener("click", () => { telecharger(nomFichier() + ".json", texteCourant(), "application/json"); dire(`${nomFichier()}.json enregistré.`); });
$("export-svg").addEventListener("click", async () => { telecharger(nomFichier() + ".svg", await svgDeLaScene(scene, fondDe(modele, modele.style), $("style-illustration").textContent), "image/svg+xml"); dire(`${nomFichier()}.svg enregistré (image figée à cet instant).`); });
$("export-html").addEventListener("click", async () => {
  try {
    telecharger(nomFichier() + ".html", await construireHtmlModele({ texteModele: texteCourant(), vue: vue(), vitesse: +$("vitesse").value, demo: true, titre: modele.nom }), "text/html");
    dire(`${nomFichier()}.html enregistré : page autonome, démonstration animée.`);
  } catch (err) { dire("Export impossible : " + err.message); }
});

// ----- plein écran, panneau -----
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
  if (e.ctrlKey || e.metaKey || e.altKey || /^(INPUT|SELECT|TEXTAREA)$/.test(document.activeElement?.tagName) && document.activeElement.type !== "range" && document.activeElement.type !== "checkbox") return;
  if (e.key === "f" || e.key === "F") { e.preventDefault(); basculePlein(); }
  if (e.key === "h" || e.key === "H") { e.preventDefault(); basculePanneau(); }
});
