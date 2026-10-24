"use strict";

const byId = id => document.getElementById(id);
const invoke = (command, args = {}) => window.__TAURI__.core.invoke(command, args);
const usage = new Map();
const usageRequested = new Set();
const MAX_PAGE_SIZE = 5;
let loading = false;
let awaitingOpen = true;
let profiles = [];
let activeProfileId = null;
let page = 0;
let pageSize = MAX_PAGE_SIZE;
let viewEpoch = 0;
let pendingSwitch = null;
let pendingVaultOperation = null;
let emptyMessage = "Open Accounts to unlock your saved accounts.";
let showUnlock = true;

async function invokeVault(command, args = {}) {
  const work = invoke(command, args);
  pendingVaultOperation = work;
  try {
    return await work;
  } finally {
    if (pendingVaultOperation === work) pendingVaultOperation = null;
  }
}

async function awaitPendingVaultOperation() {
  const work = pendingVaultOperation;
  if (!work) return;
  try {
    await work;
  } catch {
    // A stale operation's own caller reports its failure. Reopening only waits for release.
  }
}

function remaining(window) {
  if (!window || !Number.isFinite(window.used_percent)) return null;
  return Math.max(0, Math.min(100, Math.round(100 - window.used_percent)));
}

function resetLabel(window) {
  if (!window || !Number.isFinite(window.resets_at)) return "";
  const seconds = Math.max(0, window.resets_at - Math.floor(Date.now() / 1000));
  if (seconds < 3600) return `Resets in ${Math.max(1, Math.ceil(seconds / 60))}m`;
  if (seconds < 86400) return `Resets in ${Math.ceil(seconds / 3600)}h`;
  return `Resets in ${Math.ceil(seconds / 86400)}d`;
}

function activeFirst(items, activeId) {
  return [...items].sort((left, right) => Number(right.id === activeId) - Number(left.id === activeId));
}

function accountRow(profile) {
  const active = profile.id === activeProfileId;
  const health = usage.get(profile.id);
  const percent = remaining(health?.primary);
  const row = document.createElement("article");
  row.className = `tray-account${active ? " active" : ""}`;
  row.dataset.profileId = profile.id;
  if (active) row.setAttribute("aria-current", "true");
  const led = document.createElement("span");
  led.className = "account-led";
  led.setAttribute("role", "img");
  led.setAttribute("aria-label", active ? "Active account, last observed" : "Saved account");
  led.title = active ? "Active account, last observed" : "Saved account";
  const name = document.createElement("span");
  name.className = "account-name";
  name.textContent = profile.label;
  const value = document.createElement("span");
  value.className = percent === null ? "usage-unknown" : "usage-percent";
  value.textContent = percent === null ? "Usage unavailable" : `${percent}%`;
  const detail = document.createElement("div");
  detail.className = "usage-detail";
  const meta = document.createElement("div");
  meta.className = "usage-meta";
  const track = document.createElement("span");
  track.className = "usage-track";
  const fill = document.createElement("span");
  fill.className = "usage-fill";
  fill.style.width = `${percent ?? 0}%`;
  track.append(fill);
  const reset = document.createElement("span");
  reset.className = "reset-time";
  reset.textContent = resetLabel(health?.primary);
  meta.append(track);
  detail.append(meta, reset);
  row.append(led, name, value, detail);
  if (!active) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "switch-button";
    button.textContent = "Switch";
    button.disabled = loading;
    button.setAttribute("aria-label", `Switch to ${profile.label}`);
    button.addEventListener("click", () => requestSwitch(profile, button));
    row.append(button);
  }
  return row;
}

function render() {
  const list = byId("account-list");
  const pages = Math.max(1, Math.ceil(profiles.length / pageSize));
  page = Math.min(Math.max(0, page), pages - 1);
  const visibleProfiles = profiles.slice(page * pageSize, (page + 1) * pageSize);
  list.replaceChildren(...visibleProfiles.map(accountRow));
  list.hidden = profiles.length === 0;
  byId("loading-view").hidden = !awaitingOpen;
  byId("locked-view").hidden = awaitingOpen || profiles.length !== 0;
  const paging = byId("account-pages");
  paging.hidden = profiles.length <= pageSize;
  byId("page-label").textContent = `${page + 1} / ${pages}`;
  byId("previous-page").disabled = loading || page === 0;
  byId("next-page").disabled = loading || page + 1 >= pages;
  byId("refresh-accounts").disabled = loading;
  byId("open-accounts").disabled = loading;
  list.setAttribute("aria-busy", String(loading));
  byId("locked-message").textContent = emptyMessage;
  byId("unlock-in-main").hidden = !showUnlock;
  byId("unlock-in-main").disabled = loading;
  void invoke("resize_tray_window", { rows: visibleProfiles.length, paged: profiles.length > pageSize }).catch(() => {});
}

function resetFlyout() {
  viewEpoch += 1;
  loading = false;
  awaitingOpen = true;
  profiles = [];
  activeProfileId = null;
  page = 0;
  pageSize = MAX_PAGE_SIZE;
  pendingSwitch = null;
  if (byId("trust-warning").open) byId("trust-warning").close("cancel");
  usage.clear();
  usageRequested.clear();
  emptyMessage = "Open Accounts to unlock your saved accounts.";
  showUnlock = true;
  byId("tray-status").textContent = "";
  render();
}

async function refreshVisibleUsage(epoch, force = false) {
  const visibleProfiles = profiles.slice(page * pageSize, (page + 1) * pageSize);
  for (const profile of visibleProfiles) {
    if (epoch !== viewEpoch) return;
    if (!force && usageRequested.has(profile.id)) continue;
    usageRequested.add(profile.id);
    try {
      const result = await invokeVault("refresh_usage", { profileId: profile.id });
      if (epoch !== viewEpoch) return;
      if (result.health?.profile === profile.id) usage.set(profile.id, result.health);
    } catch {
      usage.delete(profile.id);
      usageRequested.delete(profile.id);
    }
    render();
  }
}

async function changePage(offset) {
  if (loading) return;
  const epoch = viewEpoch;
  page += offset;
  loading = true;
  render();
  try {
    await refreshVisibleUsage(epoch);
  } finally {
    if (epoch === viewEpoch) {
      loading = false;
      render();
    }
  }
}

async function load() {
  if (loading) return;
  const epoch = ++viewEpoch;
  loading = true;
  awaitingOpen = true;
  profiles = [];
  activeProfileId = null;
  page = 0;
  usage.clear();
  usageRequested.clear();
  byId("refresh-accounts").disabled = true;
  byId("tray-status").textContent = "";
  render();
  try {
    await awaitPendingVaultOperation();
    if (epoch !== viewEpoch) return;
    const capacity = await invoke("tray_page_capacity");
    if (epoch !== viewEpoch) return;
    pageSize = Number.isSafeInteger(capacity) ? Math.max(1, Math.min(MAX_PAGE_SIZE, capacity)) : MAX_PAGE_SIZE;
    let result = await invokeVault("list_profiles");
    if (epoch !== viewEpoch) return;
    if (result.state !== "open" && result.vault_exists === true) result = await invokeVault("open_vault");
    if (epoch !== viewEpoch) return;
    if (result.state !== "open") throw new Error("vault_not_open");
    activeProfileId = result.last_known_active_profile;
    profiles = activeFirst(result.profiles, activeProfileId);
    awaitingOpen = false;
    render();
    await refreshVisibleUsage(epoch);
    if (epoch !== viewEpoch) return;
  } catch (error) {
    if (epoch !== viewEpoch) return;
    profiles = [];
    activeProfileId = null;
    usage.clear();
    const busy = ["vault_busy", "E_BUSY"].includes(error) || ["vault_busy", "E_BUSY"].includes(error?.message);
    emptyMessage = busy ? "Accounts are busy. Try again." : "Open Accounts to unlock your saved accounts.";
    showUnlock = !busy;
  } finally {
    if (epoch === viewEpoch) {
      loading = false;
      awaitingOpen = false;
      byId("refresh-accounts").disabled = false;
      render();
    }
  }
}

function requestSwitch(profile, button) {
  const active = profiles.find(candidate => candidate.id === activeProfileId);
  if (active?.domain && profile.domain && active.domain !== profile.domain) {
    pendingSwitch = { profile, button };
    byId("trust-warning").showModal();
    return;
  }
  void switchTo(profile, button);
}

async function switchTo(profile, button) {
  if (loading || profile.id === activeProfileId) return;
  const epoch = viewEpoch;
  loading = true;
  button.disabled = true;
  button.textContent = "Switching";
  byId("tray-status").textContent = `Switching to ${profile.label}`;
  try {
    const result = await invokeVault("switch_profile", { profileId: profile.id });
    if (epoch !== viewEpoch) return;
    activeProfileId = result.last_known_active_profile;
    profiles = activeFirst(result.profiles, activeProfileId);
    page = 0;
    byId("tray-status").textContent = `${profile.label} is ready`;
    render();
    await refreshVisibleUsage(epoch, true);
    if (epoch !== viewEpoch) return;
  } catch {
    if (epoch !== viewEpoch) return;
    byId("tray-status").textContent = "Close Codex, then try again";
  } finally {
    if (epoch === viewEpoch) {
      loading = false;
      render();
    }
  }
}

function openMain() {
  if (loading) return;
  resetFlyout();
  void invoke("show_main_window");
}
byId("refresh-accounts").addEventListener("click", () => void load());
byId("open-accounts").addEventListener("click", openMain);
byId("unlock-in-main").addEventListener("click", openMain);
byId("previous-page").addEventListener("click", () => void changePage(-1));
byId("next-page").addEventListener("click", () => void changePage(1));
byId("trust-warning").addEventListener("close", () => {
  if (byId("trust-warning").returnValue === "confirm" && pendingSwitch) {
    const selected = pendingSwitch;
    pendingSwitch = null;
    void switchTo(selected.profile, selected.button);
  } else {
    pendingSwitch = null;
  }
});
document.addEventListener("keydown", event => {
  if (event.key === "Escape" && !byId("trust-warning").open) {
    resetFlyout();
    void invoke("hide_tray_window");
  }
});
const eventApi = window.__TAURI__?.event;
if (eventApi?.listen) {
  void eventApi.listen("tray-opened", () => void load());
  void eventApi.listen("tray-hidden", resetFlyout);
}
resetFlyout();
