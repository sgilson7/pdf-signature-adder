// The page draws what the core describes and decides nothing.
//
// The core (through the shim's Tool) holds the PDF and the signatures and
// returns a description of the whole screen. This file reads files, the
// clipboard and pointer events, hands them to the core as text or bytes, and
// draws the description back: each region is redrawn only when its part of
// the description changed. Positions arrive as CSS percentages and sizes as
// CSS lengths, so this file holds no number but 0 and 1 (BUILDER-SPEC D3).
// pdf.js only paints pages; the core writes the file (PS-7).
import init, { Tool } from "./pkg/tool_shim.js?v=a6af7a09aba5";
import * as pdfjs from "./vendor/pdfjs/pdf.mjs";
import * as pdfjsWorker from "./vendor/pdfjs/pdf.worker.mjs";

// With its worker module already loaded here, pdf.js runs on this page's own
// thread: it starts no Worker and fetches nothing after the page is ready.
globalThis.pdfjsWorker = pdfjsWorker;

const SVG = "http://www.w3.org/2000/svg";
const el = (id) => document.getElementById(id);
const make = (tag, cls, text) => {
  const node = document.createElement(tag);
  if (cls) node.className = cls;
  if (text !== undefined) node.textContent = text;
  return node;
};
const svg = (tag, attrs) => {
  const node = document.createElementNS(SVG, tag);
  Object.entries(attrs || {}).forEach(([k, v]) => node.setAttribute(k, String(v)));
  return node;
};

let tool = null;
let ui = null;
let layout = {};
let pdf = null;
let loadedDocument = 0;
let pictureVersion = -1;
let lastScroll = 0;
let fileUrl = null;
const drawn = {};
const artUrls = new Map();
const painted = new Map();
const tasks = new Map();
const visible = new Set();

function apply(text) {
  const result = JSON.parse(text);
  if (result.error) {
    el("message").textContent = result.error;
    return;
  }
  ui = result.ok;
  render();
}

const act = (name, value = "") => apply(tool.act(name, value));

// ---- Regions ---------------------------------------------------------------

function region(name, value, draw) {
  const text = JSON.stringify(value === undefined ? null : value);
  if (drawn[name] === text) return;
  drawn[name] = text;
  draw(value);
}

function render() {
  document.title = ui.title;
  document.body.dataset.screen = ui.screen;
  document.body.dataset.placing = String(ui.placing);
  // While a dialog is open the page behind it is locked, so a stroke on the
  // pad can never scroll it (T1).
  document.body.dataset.dialog = String(Boolean(ui.sheet || ui.saved));
  region("bar", [ui.file, ui.toolbar, ui.zoom, ui.nav], drawBar);
  region("status", [ui.status, ui.status_actions, ui.message, ui.note], drawStatus);
  region("start", [ui.screen, ui.title, ui.tagline, ui.start, ui.drop_hint, ui.toolbar], drawStart);
  region("frames", [ui.document, ui.pages.map(frameOf)], drawFrames);
  ui.pages.forEach((p) => {
    const selection = ui.selection && ui.selection.page === p.index ? ui.selection : null;
    region("page-" + p.index, [p.label, p.overlays, ui.arts, selection], () => drawOverlays(p, selection));
  });
  const sheet = ui.sheet;
  region("sheet", sheet && { ...sheet, pad: sheet.pad && { ...sheet.pad, paths: null } }, drawSheet);
  region("pad-paths", sheet && sheet.pad && [sheet.pad.paths, sheet.pad.stroke, sheet.pad.stroke_width], drawPadPaths);
  if (sheet && sheet.picture && sheet.picture.version !== pictureVersion) drawPicture(sheet.picture);
  region("saved", ui.saved, drawSaved);
  if (ui.scroll !== lastScroll) {
    lastScroll = ui.scroll;
    const target = document.querySelector(`[data-page="${ui.scroll_page}"]`);
    if (target) target.scrollIntoView({ block: "start" });
  }
}

// Buttons: most send their action to the core; the few that need the
// browser (files, clipboard, downloads) are handled here.
const browserActions = {
  "file.open": () => el("pdf-input").click(),
  "file.save": () => apply(tool.save()),
  "picture.open": () => el("image-input").click(),
  "picture.paste": () => pasteFromClipboard(),
  "signature.save_image": () => saveSignatureImage(),
  "saved.download": () => download(tool.saved_bytes(), ui.saved.file_name, layout.pdfType),
  "saved.share": () => share(),
};

function button(a, choice) {
  const b = make("button", a.primary ? "primary" : "", a.label);
  b.type = "button";
  b.disabled = !a.enabled;
  b.dataset.action = a.name;
  if (a.value) b.dataset.value = a.value;
  if (choice) b.setAttribute("aria-pressed", String(a.pressed));
  b.addEventListener("click", () => (browserActions[a.name] ? browserActions[a.name]() : act(a.name, a.value)));
  return b;
}

function drawBar([file, toolbar, zoom, nav]) {
  el("file-actions").replaceChildren(...(file ? toolbar.slice(0, 1).map((a) => button(a)) : []));
  el("tools").replaceChildren(...(file ? [...zoom.map((a) => button(a, a.name === "zoom.fit")), ...toolbar.slice(1).map((a) => button(a))] : []));
  el("file").hidden = !file;
  if (file) {
    el("file-name").textContent = file.name;
    el("file-detail").textContent = file.detail;
  }
  el("nav").hidden = !nav;
  if (nav) {
    el("nav").replaceChildren(button(nav.prev), el("nav-label"), button(nav.next));
    el("nav-label").textContent = nav.label;
  }
}

function drawStatus([status, actions, message, note]) {
  el("status").textContent = status;
  el("status-actions").replaceChildren(...actions.map((a) => button(a)));
  el("status-bar").hidden = !status;
  el("message").textContent = message;
  el("note").textContent = note;
}

function drawStart([screen, title, tagline, start, dropHint, toolbar]) {
  el("start").hidden = screen !== "start";
  el("title").textContent = title;
  el("tagline").textContent = tagline;
  const [heading, text, privacy] = start;
  el("start-heading").textContent = heading;
  el("start-text").textContent = text;
  el("privacy").textContent = privacy;
  el("drop-hint").textContent = dropHint;
  el("start-actions").replaceChildren(button({ ...toolbar[0], primary: true }));
}

// ---- Pages -----------------------------------------------------------------

const frameOf = (p) => ({
  index: p.index,
  css_width: p.css_width,
  css_height: p.css_height,
  canvas_width: p.canvas_width,
  canvas_height: p.canvas_height,
  scale: p.scale,
});

function drawFrames([, frames]) {
  tasks.forEach((t) => t.cancel());
  tasks.clear();
  painted.clear();
  near.disconnect();
  onScreen.disconnect();
  visible.clear();
  el("pages").replaceChildren(
    ...frames.map((f) => {
      const page = make("section", "page");
      page.dataset.page = String(f.index);
      const label = make("h2", "page-label");
      const paper = make("div", "paper");
      paper.style.width = f.css_width;
      paper.style.height = f.css_height;
      const canvas = make("canvas");
      const layer = make("div", "layer");
      paper.append(canvas, layer);
      page.append(label, paper);
      watchPointer(paper, f.index);
      near.observe(paper);
      onScreen.observe(paper);
      paper.frame = f;
      return page;
    }),
  );
  // Region names for overlays were drawn into the old pages; draw them again.
  Object.keys(drawn)
    .filter((k) => k.startsWith("page-"))
    .forEach((k) => delete drawn[k]);
}

// Pages near the screen are painted; pages far from it give their pixels back,
// which keeps a long document inside an iPad's canvas memory. Both observers
// are made once data/layout.json has given the margin.
let near = null;
let onScreen = null;

function observe() {
  near = new IntersectionObserver(
    (entries) =>
      entries.forEach((e) => {
        if (e.isIntersecting) paint(e.target);
        else unpaint(e.target);
      }),
    { rootMargin: layout.paintMargin },
  );
  onScreen = new IntersectionObserver((entries) => {
    entries.forEach((e) => {
      const k = e.target.frame.index;
      if (e.isIntersecting) visible.add(k);
      else visible.delete(k);
    });
    if (visible.size) act("view.visible", [...visible].join(","));
  });
}

async function paint(paper) {
  const f = paper.frame;
  const key = JSON.stringify(f);
  if (!pdf || painted.get(f.index) === key) return;
  painted.set(f.index, key);
  const canvas = paper.querySelector("canvas");
  canvas.width = f.canvas_width;
  canvas.height = f.canvas_height;
  try {
    const page = await pdf.getPage(f.index + 1);
    const task = page.render({ canvasContext: canvas.getContext("2d"), viewport: page.getViewport({ scale: f.scale }) });
    tasks.set(f.index, task);
    await task.promise;
  } catch (e) {
    // A cancelled paint is replaced by the next one; nothing to report.
    if (!(e instanceof pdfjs.RenderingCancelledException)) painted.delete(f.index);
  } finally {
    tasks.delete(f.index);
  }
}

function unpaint(paper) {
  const k = paper.frame.index;
  if (!painted.has(k)) return;
  painted.delete(k);
  const t = tasks.get(k);
  if (t) t.cancel();
  const canvas = paper.querySelector("canvas");
  canvas.width = 0;
  canvas.height = 0;
}

async function loadPdf(bytes) {
  if (pdf) await pdf.destroy();
  pdf = null;
  pdf = await pdfjs.getDocument({
    data: bytes.slice(),
    isEvalSupported: false,
    disableAutoFetch: true,
    disableStream: true,
    disableRange: true,
  }).promise;
  delete drawn.frames;
  render();
  document.querySelectorAll(".paper").forEach((paper) => {
    const r = paper.getBoundingClientRect();
    if (r.top < innerHeight && r.bottom > 0) paint(paper);
  });
}

function pointerText(e, node) {
  const r = node.getBoundingClientRect();
  return [e.pointerId, e.pointerType, e.clientX, e.clientY, r.left, r.top, r.width, r.height].join("|");
}

// iPad Safari starts a scroll from touch events (the Pencil's included) unless
// they are cancelled, and a scroll cancels the pointer: one dot of ink, then
// the page moves (T1). touch-action alone was not enough on the SVG pad.
function holdStill(node, only) {
  const stop = (e) => {
    if (!only || e.target.closest(only)) e.preventDefault();
  };
  node.addEventListener("touchstart", stop, { passive: false });
  node.addEventListener("touchmove", stop, { passive: false });
}

function capture(node, e) {
  try {
    node.setPointerCapture(e.pointerId);
  } catch (err) {
    // A refused capture still lets the drag work while the pointer stays on.
  }
}

function watchPointer(paper, k) {
  let pressed = null;
  // A finger on a placed signature drags it; elsewhere it scrolls the page.
  holdStill(paper, ".overlay");
  paper.addEventListener("pointerdown", (e) => {
    if (e.button > 0 || e.target.closest(".selection-actions")) return;
    pressed = e.pointerId;
    capture(paper, e);
    act("page.down", k + "|" + pointerText(e, paper));
  });
  paper.addEventListener("pointermove", (e) => {
    if (pressed === e.pointerId) act("page.move", k + "|" + pointerText(e, paper));
  });
  const up = (e) => {
    if (pressed !== e.pointerId) return;
    pressed = null;
    act("page.up", k + "|" + pointerText(e, paper));
  };
  paper.addEventListener("pointerup", up);
  paper.addEventListener("pointercancel", up);
}

function artNode(art) {
  if (art.kind === "picture") {
    const img = make("img");
    img.alt = "";
    img.src = artUrl(art);
    return img;
  }
  const s = svg("svg", { viewBox: art.view_box, preserveAspectRatio: "none", "aria-hidden": "true" });
  const g = svg("g", {
    fill: "none",
    stroke: art.stroke,
    "stroke-width": art.stroke_width,
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
  });
  art.paths.forEach((d) => g.append(svg("path", { d })));
  s.append(g);
  return s;
}

function pixelsToCanvas(bytes, width, height) {
  const c = make("canvas");
  c.width = width;
  c.height = height;
  c.getContext("2d").putImageData(new ImageData(new Uint8ClampedArray(bytes.buffer, bytes.byteOffset, bytes.length), width, height), 0, 0);
  return c;
}

function artUrl(art) {
  if (!artUrls.has(art.id)) artUrls.set(art.id, pixelsToCanvas(tool.art_rgba(art.id), art.width, art.height).toDataURL());
  return artUrls.get(art.id);
}

function drawOverlays(p, selection) {
  const page = document.querySelector(`[data-page="${p.index}"]`);
  if (!page) return;
  page.querySelector(".page-label").textContent = p.label;
  const layer = page.querySelector(".layer");
  const nodes = p.overlays.map((o) => {
    const box = make("div", o.selected ? "overlay selected" : "overlay");
    Object.assign(box.style, { left: o.left, top: o.top, width: o.width, height: o.height });
    box.append(artNode(ui.arts[o.art]));
    if (o.selected) ["tl", "tr", "bl", "br"].forEach((c) => box.append(make("span", "handle " + c)));
    return box;
  });
  if (selection) {
    const bar = make("div", "selection-actions");
    Object.assign(bar.style, { left: selection.left, top: selection.top });
    bar.append(...selection.actions.map((a) => button(a)));
    nodes.push(bar);
  }
  layer.replaceChildren(...nodes);
}

// ---- The signature sheet ---------------------------------------------------

function row(label, actions, choice) {
  if (!actions.length) return null;
  const r = make("div", "row");
  if (label) r.append(make("span", "row-label", label));
  const g = make("div", "group");
  g.append(...actions.map((a) => button(a, choice)));
  r.append(g);
  return r;
}

function drawSheet(sheet) {
  el("sheet-backdrop").hidden = !sheet;
  if (!sheet) return;
  pictureVersion = -1;
  const head = make("div", "dialog-head");
  const title = make("h2", "", sheet.title);
  title.id = "sheet-title";
  head.append(title, button(sheet.close));
  const tabs = make("div", "tabs group");
  tabs.append(...sheet.tabs.map((a) => button(a, true)));
  const body = make("div", "sheet-body");
  if (sheet.pad) {
    const box = make("div", "pad-box");
    // The pointer events land on a plain box around the drawing, where
    // WebKit honours touch-action; on the SVG itself it may not (T1).
    const surface = make("div", "pad-surface");
    surface.id = "pad-surface";
    const pad = svg("svg", { id: "pad", viewBox: sheet.pad.view_box, role: "img", "aria-label": sheet.pad.label });
    pad.append(
      svg("line", { x1: "0", y1: sheet.pad.baseline, x2: "100%", y2: sheet.pad.baseline, class: "baseline" }),
      svg("g", { id: "pad-ink", fill: "none", "stroke-linecap": "round", "stroke-linejoin": "round" }),
    );
    surface.append(pad);
    box.append(surface, make("p", "pad-hint", sheet.pad.hint));
    body.append(box);
    watchPad(surface);
  } else {
    if (sheet.hint) body.append(make("p", "hint", sheet.hint));
    const preview = make("div", "preview");
    preview.id = "preview";
    preview.append(make("p", "empty", sheet.empty));
    body.append(preview, make("p", "source"));
    if (sheet.fetch) body.append(row("", [sheet.fetch], false));
  }
  const controls = [
    row(sheet.ink_label, sheet.inks, true),
    row(sheet.pen_label, sheet.weights, true),
    row("", sheet.edits, false),
    row("", sheet.options, true),
  ].filter(Boolean);
  const actions = make("div", "dialog-actions group");
  actions.append(...sheet.actions.map((a) => button(a)));
  el("sheet").replaceChildren(head, tabs, body, ...controls, actions);
  el("sheet").dataset.tab = sheet.tab;
  delete drawn["pad-paths"];
}

function drawPadPaths(value) {
  const ink = document.getElementById("pad-ink");
  if (!ink || !value) return;
  const [paths, stroke, width] = value;
  ink.setAttribute("stroke", stroke);
  ink.setAttribute("stroke-width", String(width));
  ink.replaceChildren(...paths.map((d) => svg("path", { d })));
}

function watchPad(pad) {
  let pressed = null;
  holdStill(pad);
  pad.addEventListener("pointerdown", (e) => {
    if (e.button > 0) return;
    pressed = e.pointerId;
    capture(pad, e);
    e.preventDefault();
    act("pad.down", pointerText(e, pad));
  });
  pad.addEventListener("pointermove", (e) => {
    if (pressed !== e.pointerId) return;
    // A Pencil reports many samples between frames; send them all at once.
    const samples = e.getCoalescedEvents ? e.getCoalescedEvents() : [];
    act("pad.move", (samples.length ? samples : [e]).map((s) => pointerText(s, pad)).join(";"));
  });
  const up = (e) => {
    if (pressed !== e.pointerId) return;
    pressed = null;
    act("pad.up", pointerText(e, pad));
  };
  pad.addEventListener("pointerup", up);
  pad.addEventListener("pointercancel", up);
}

function drawPicture(p) {
  pictureVersion = p.version;
  const preview = el("preview");
  if (!preview) return;
  const c = pixelsToCanvas(tool.picture_rgba(), p.width, p.height);
  c.setAttribute("role", "img");
  c.setAttribute("aria-label", p.source);
  preview.replaceChildren(c);
  document.querySelector("#sheet .source").textContent = p.source;
}

// ---- Pictures in -------------------------------------------------------------

async function decodePicture(blob, name) {
  const isVector = blob.type === layout.svgType || name.toLowerCase().endsWith(".svg");
  const source = isVector ? new Blob([await blob.text()], { type: layout.svgType }) : blob;
  const url = URL.createObjectURL(source);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    const size = JSON.parse(tool.picture_size(String(img.naturalWidth), String(img.naturalHeight), isVector));
    if (size.error) return act("picture.unreadable");
    const [width, height] = size.ok;
    const c = make("canvas");
    c.width = width;
    c.height = height;
    const ctx = c.getContext("2d");
    ctx.drawImage(img, 0, 0, width, height);
    const data = ctx.getImageData(0, 0, width, height).data;
    apply(tool.picture(name, width, height, new Uint8Array(data.buffer)));
  } catch (e) {
    act("picture.unreadable");
  } finally {
    URL.revokeObjectURL(url);
  }
}

async function pasteFromClipboard() {
  try {
    const items = await navigator.clipboard.read();
    for (const item of items) {
      const type = item.types.find((t) => t.startsWith("image/"));
      if (type) return decodePicture(await item.getType(type), "clipboard");
    }
    act("clipboard.empty");
  } catch (e) {
    act("clipboard.refused");
  }
}

// ---- Files out ---------------------------------------------------------------

function download(bytes, name, type) {
  if (fileUrl) URL.revokeObjectURL(fileUrl);
  fileUrl = URL.createObjectURL(bytes instanceof Blob ? bytes : new Blob([bytes], { type }));
  const a = make("a");
  a.href = fileUrl;
  a.download = name;
  document.body.append(a);
  a.click();
  a.remove();
}

async function share() {
  const file = new File([tool.saved_bytes()], ui.saved.file_name, { type: layout.pdfType });
  try {
    await navigator.share({ files: [file] });
  } catch (e) {
    // Closing the share sheet without choosing is not an error.
  }
}

async function saveSignatureImage() {
  const result = JSON.parse(tool.drawing_file());
  if (result.error) return;
  const { svg: text, width, height } = result.ok;
  const url = URL.createObjectURL(new Blob([text], { type: layout.svgType }));
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    const c = make("canvas");
    c.width = width;
    c.height = height;
    c.getContext("2d").drawImage(img, 0, 0, width, height);
    c.toBlob((blob) => download(blob, ui.sheet.image_file, layout.pngType), layout.pngType);
  } finally {
    URL.revokeObjectURL(url);
  }
}

function drawSaved(saved) {
  el("saved-backdrop").hidden = !saved;
  if (!saved) return;
  const title = make("h2", "", saved.title);
  title.id = "saved-title";
  const actions = make("div", "dialog-actions group");
  actions.append(...saved.actions.map((a) => button(a)));
  el("saved").replaceChildren(title, make("p", "saved-detail", saved.detail), make("p", "hint", saved.note), actions);
}

// ---- Files in ----------------------------------------------------------------

async function openPdf(file) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  apply(tool.open_pdf(file.name, bytes));
  if (ui.document !== loadedDocument) {
    loadedDocument = ui.document;
    artUrls.clear();
    await loadPdf(bytes);
  }
}

function openFile(file) {
  if (!file) return;
  const isPdf = file.type === layout.pdfType || file.name.toLowerCase().endsWith(".pdf");
  if (isPdf) openPdf(file);
  else decodePicture(file, file.name);
}

function canShareFiles() {
  try {
    const probe = new File([new Uint8Array()], "probe.pdf", { type: layout.pdfType });
    return Boolean(navigator.canShare && navigator.canShare({ files: [probe] }));
  } catch (e) {
    return false;
  }
}

async function main() {
  await init();
  const strings = await (await fetch("data/strings.json?v=a6af7a09aba5")).text();
  const glossary = await (await fetch("data/glossary.md?v=a6af7a09aba5")).text();
  layout = JSON.parse(await (await fetch("data/layout.json?v=a6af7a09aba5")).text());
  observe();
  tool = new Tool(strings, glossary);
  apply(tool.ui());
  act("view.dpr", String(devicePixelRatio));
  act("view.width", String(el("main").clientWidth));
  act("ability.share", String(canShareFiles()));
  act("ability.clipboard", String(Boolean(navigator.clipboard && navigator.clipboard.read)));
  new ResizeObserver(() => act("view.width", String(el("main").clientWidth))).observe(el("main"));

  el("pdf-input").addEventListener("change", (e) => {
    openFile(e.target.files[0]);
    e.target.value = "";
  });
  el("image-input").addEventListener("change", (e) => {
    const file = e.target.files[0];
    if (file) decodePicture(file, file.name);
    e.target.value = "";
  });
  document.addEventListener("dragover", (e) => e.preventDefault());
  document.addEventListener("drop", (e) => {
    e.preventDefault();
    openFile(e.dataTransfer.files[0]);
  });
  document.addEventListener("paste", (e) => {
    const item = [...e.clipboardData.items].find((i) => i.type.startsWith("image/"));
    if (!item) return;
    e.preventDefault();
    decodePicture(item.getAsFile(), "clipboard");
  });
  document.addEventListener("keydown", (e) => {
    if (e.target.closest("input, textarea")) return;
    const arrow = e.key.startsWith("Arrow");
    if (!arrow && !["Escape", "Delete", "Backspace"].includes(e.key)) return;
    if (arrow && ui.selection) e.preventDefault();
    act("key", e.shiftKey ? e.key + "+shift" : e.key);
  });
  document.body.dataset.ready = "true";
}

main();
