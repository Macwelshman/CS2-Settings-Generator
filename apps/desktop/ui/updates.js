// Kept independent of scan state: update checks never alter asset settings.
(() => {
  const check = document.querySelector("#check-updates");
  const banner = document.querySelector("#update-banner");
  const title = document.querySelector("#update-title");
  const message = document.querySelector("#update-message");
  const install = document.querySelector("#install-update");
  const view = document.querySelector("#view-update");
  const later = document.querySelector("#dismiss-update");
  let checking = false;
  let installing = false;
  let available = null;

  async function checkUpdates(manual) {
    if (!invoke || checking || installing) return;
    checking = true;
    check.disabled = true;
    install.disabled = true;
    check.textContent = "Checking…";
    try {
      available = await invoke("check_for_updates");
      banner.classList.toggle("is-hidden", !available);
      if (available) {
        title.textContent = `Version ${available.version} is available`;
        message.textContent = available.message;
      } else if (manual) {
        showToast("You’re using the latest stable version.");
      }
    } catch (error) {
      if (manual) showToast(String(error));
    } finally {
      checking = false;
      check.disabled = false;
      install.disabled = !available?.canInstall;
      check.textContent = "Check for Updates…";
    }
  }
  check.addEventListener("click", () => checkUpdates(true));
  later.addEventListener("click", () => banner.classList.add("is-hidden"));
  view.addEventListener("click", async () => {
    try { await invoke("view_update_release"); } catch (error) { showToast(String(error)); }
  });
  install.addEventListener("click", async () => {
    if (state.busy || installing || checking) {
      showToast("Wait for the current operation to finish before updating.");
      return;
    }
    installing = true;
    check.disabled = install.disabled = view.disabled = later.disabled = true;
    install.textContent = "Downloading and verifying…";
    setBusy(true, "Preparing software update…");
    try {
      await invoke("install_update");
    } catch (error) {
      message.textContent = String(error);
    } finally {
      installing = false;
      check.disabled = view.disabled = later.disabled = false;
      install.disabled = !available?.canInstall;
      install.textContent = "Update Now";
      setBusy(false);
    }
  });
  window.__TAURI__?.event?.listen("check-for-updates", () => checkUpdates(true));
  checkUpdates(false);
})();
