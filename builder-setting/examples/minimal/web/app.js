// The page draws what the core returns and decides nothing.
// Every call goes through the shim, which returns {"ok": ...} or {"error": "..."}.
import init, { start, act } from "./pkg/tool_shim.js";

const el = (id) => document.getElementById(id);
let settingsText = "";
let state = null;

function draw(result) {
  if (result.error) {
    el("message").textContent = result.error;
    return;
  }
  const { settings, view } = result.ok;
  state = result.ok.state;
  document.title = settings.title;
  el("title").textContent = settings.title;
  el("intro").textContent = settings.intro;
  el("add").textContent = settings.add_label;
  el("remove").textContent = settings.remove_label;
  el("reset").textContent = settings.reset_label;
  el("count").textContent = view.count;
  el("add").disabled = !view.can_add;
  el("remove").disabled = !view.can_remove;
  el("note").textContent = view.note;
  el("message").textContent = "";
}

function press(action) {
  draw(JSON.parse(act(settingsText, JSON.stringify(state), action)));
}

async function main() {
  await init();
  settingsText = await (await fetch("data/counter.json")).text();
  draw(JSON.parse(start(settingsText)));
  for (const action of ["add", "remove", "reset"]) {
    el(action).addEventListener("click", () => press(action));
  }
  document.body.dataset.ready = "true";
}

main();
