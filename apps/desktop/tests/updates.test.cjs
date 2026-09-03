const { test } = require("node:test");
const assert = require("node:assert/strict");
const vm = require("node:vm");
const fs = require("node:fs");

async function fixture(response) {
  const elements = new Map();
  const calls = [], messages = [];
  const context = {
    document: { querySelector(id) {
      if (!elements.has(id)) elements.set(id, {
        disabled: false, textContent: "", handlers: {}, hidden: true,
        addEventListener(event, handler) { this.handlers[event] = handler; },
        classList: {
          toggle(name, hidden) { elements.get(id).hidden = hidden; },
          add() { elements.get(id).hidden = true; },
        },
      });
      return elements.get(id);
    } },
    window: {}, state: { busy: false },
    setBusy(busy) { context.state.busy = busy; },
    showToast(message) { messages.push(message); },
    async invoke(command) {
      calls.push(command);
      if (command === "check_for_updates") {
        if (response instanceof Error) throw response;
        return response;
      }
      if (command === "install_update") throw Error("Test download failure");
    },
  };
  vm.runInNewContext(fs.readFileSync(`${__dirname}/../ui/updates.js`, "utf8"), context);
  await new Promise(setImmediate);
  return { elements, calls, messages, context };
}

test("startup failures stay quiet, manual failures are visible", async () => {
  const f = await fixture(Error("offline"));
  assert.equal(f.messages.length, 0);
  await f.elements.get("#check-updates").handlers.click();
  assert.match(f.messages[0], /offline/);
});
test("new releases show actions; Later is session-only and manual check restores notice", async () => {
  const f = await fixture({ version: "1.2.3", canInstall: true, message: "Restart required" });
  assert.equal(f.elements.get("#update-banner").hidden, false);
  assert.equal(f.elements.get("#install-update").disabled, false);
  f.elements.get("#dismiss-update").handlers.click();
  assert.equal(f.elements.get("#update-banner").hidden, true);
  await f.elements.get("#check-updates").handlers.click();
  assert.equal(f.elements.get("#update-banner").hidden, false);
  await f.elements.get("#install-update").handlers.click();
  assert.equal(f.context.state.busy, false);
  assert.match(f.elements.get("#update-message").textContent, /download failure/);
});
test("missing packages cannot be installed; current release has no notice", async () => {
  const missing = await fixture({ version: "1.2.3", canInstall: false, message: "Missing ZIP" });
  assert.equal(missing.elements.get("#install-update").disabled, true);
  const current = await fixture(null);
  assert.equal(current.elements.get("#update-banner").hidden, true);
  await current.elements.get("#check-updates").handlers.click();
  assert.match(current.messages[0], /latest stable/);
});
