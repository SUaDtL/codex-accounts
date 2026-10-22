"use strict";

const byId = (id) => document.getElementById(id);
const refresh = byId("refresh");
const activity = byId("activity");
let refreshing = false;
let vaultBusy = false;
let vaultState = "unopened";
let vaultExists = null;
let loginPending = false;
let loginCancelling = false;
let switchPending = false;
let switchCancelling = false;
let loginEpoch = 0;
let pendingAction = null;
let dialogTrigger = null;
const observedHealth = new Map();
const usageErrors = new Map();
const expiredUsage = new Set();
const refreshedResets = new Map();
let usageBusy = false;
let usageRequest = null;
let usageEpoch = 0;
let usageTimer = null;
let usageBackoffUntil = 0;
let displayedProfiles = [];
let activeProfileId = null;
const vaultControls = ["open-vault", "create-vault", "refresh-profiles", "close-vault", "recover-vault", "add-account"];

function updateControls() {
  refresh.disabled = vaultBusy || refreshing;
  for (const id of vaultControls) byId(id).disabled = vaultBusy;
  byId("setup-button").disabled = vaultBusy || (vaultState !== "open" && vaultExists === null);
  byId("add-account").disabled = vaultBusy || vaultState !== "open";
  byId("cancel-login").disabled = !loginPending || loginCancelling;
  byId("cancel-switch").disabled = !switchPending || switchCancelling;
  byId("export-diagnostics").disabled = vaultBusy || !["open", "recovery_required"].includes(vaultState);
  for (const button of document.querySelectorAll("[data-vault-action]")) {
    button.disabled = vaultBusy || (button.dataset.requiresOpen === "true" && vaultState !== "open");
  }
  byId("accounts-view").setAttribute("aria-busy", String(vaultBusy));
}

function openSettings() {
  byId("vault-message").textContent = vaultState === "open" ? `${displayedProfiles.length} saved account${displayedProfiles.length === 1 ? "" : "s"}.`
    : vaultState === "recovery_required" ? "Recovery required before account changes."
      : vaultExists === true ? "Your saved accounts are ready to open."
        : vaultExists === false ? "No saved accounts yet." : "Vault status unavailable.";
  byId("settings-dialog").showModal();
  byId("accounts-nav").focus();
}
function closeSettings() {
  byId("settings-dialog").close();
  byId("settings-button").focus();
}
byId("settings-button").addEventListener("click", openSettings);
byId("setup-button").addEventListener("click", () => {
  if (vaultBusy) return;
  if (vaultState === "open") openAddAccount();
  else if (vaultExists !== null) void vaultCommand(vaultExists ? "open_vault" : "create_vault");
});
byId("accounts-nav").addEventListener("click", closeSettings);

const inventoryErrors = Object.freeze({
  E_PLATFORM_UNSUPPORTED: "Installation discovery is not available on this platform.",
  E_ENUMERATION_UNAVAILABLE: "Windows could not list registered applications. Try refreshing inventory.",
  E_ACCESS_DENIED: "Windows denied access to installation metadata.",
  E_CANDIDATE_MISSING: "The detected installation is no longer available. Try refreshing inventory.",
  E_PATH_UNSAFE: "Installation metadata could not be read safely.",
  E_EXTERNAL_CHANGE: "The installation changed during inspection. Try refreshing inventory.",
  E_INPUT_LIMIT: "Installation metadata exceeded the supported size limit.",
  E_METADATA_INVALID: "Installation metadata could not be interpreted.",
  E_SHARING_VIOLATION: "Installation metadata is in use. Try refreshing inventory.",
  E_INVENTORY_IO: "Installation metadata could not be read. Try refreshing inventory."
});

function unavailable(message) {
  byId("selected-installation").textContent = "Unavailable";
  byId("inventory-detail").textContent = message;
}

function renderInventory(status) {
  const inventory = status.inventory;
  if (inventory.state === "unavailable") {
    unavailable(inventoryErrors[inventory.code] || "Installation metadata could not be read.");
    return;
  }
  const installations = inventory.installations;
  byId("selected-installation").textContent = installations.length === 1
    ? `${installations[0].version.join(".")} (${installations[0].architecture})`
    : installations.length === 0 ? "Not found" : "Multiple installations";
  byId("inventory-detail").textContent = installations.length === 1 ? ""
    : installations.length === 0 ? "Install Codex, then refresh."
      : "A single supported Codex installation could not be selected.";
}

function renderActiveAccount(state, profile) {
  byId("active-account-status").hidden = Boolean(profile) || ["unopened", "closed"].includes(state);
  byId("active-account-heading").textContent = state === "recovery_required" ? "Recovery required"
    : state === "pending" ? "Account action in progress" : "Current account not identified";
  if (!profile) document.querySelectorAll(".active-profile").forEach((row) => row.classList.remove("active-profile"));
  if (!profile) document.querySelectorAll(".account-indicator.on").forEach((light) => {
    light.classList.remove("on"); light.setAttribute("aria-label", "Status not confirmed"); light.title = "Status not confirmed";
  });
}

function renderProfiles(result) {
  const focusedProfile = document.activeElement?.closest?.(".account-row")?.dataset.profileId;
  vaultState = result.state;
  vaultExists = typeof result.vault_exists === "boolean" ? result.vault_exists : null;
  const open = result.state === "open";
  const recovery = result.state === "recovery_required";
  const profiles = result.profiles.slice(0, 50);
  byId("account-count").textContent = String(profiles.length);
  const active = open ? profiles.find((profile) => profile.id === result.last_known_active_profile) : null;
  renderActiveAccount(result.state, active);
  if (profiles.length === 0) byId("active-account-status").hidden = true;
  if (!open && !recovery) { observedHealth.clear(); usageErrors.clear(); expiredUsage.clear(); refreshedResets.clear(); }
  const ids = new Set(profiles.map((profile) => profile.id));
  for (const id of observedHealth.keys()) if (!ids.has(id)) observedHealth.delete(id);
  for (const id of usageErrors.keys()) if (!ids.has(id)) usageErrors.delete(id);
  for (const id of expiredUsage) if (!ids.has(id)) expiredUsage.delete(id);
  for (const key of refreshedResets.keys()) if (!ids.has(key.split(":")[0])) refreshedResets.delete(key);
  if (result.health && ids.has(result.health.profile)) {
    const prior = observedHealth.get(result.health.profile);
    if (!prior || result.health.checked_at > prior.checked_at) {
      observedHealth.set(result.health.profile, result.health);
      usageErrors.delete(result.health.profile);
      expiredUsage.delete(result.health.profile);
    }
  }
  displayedProfiles = profiles;
  activeProfileId = active?.id || null;
  const ordered = [...profiles].sort((a, b) => Number(b === active) - Number(a === active));
  const rows = ordered.map((profile) => {
    const row = document.createElement("li");
    row.className = profile === active ? "account-row active-profile" : "account-row";
    row.dataset.profileId = profile.id;
    const person = document.createElement("div"); person.className = "account-person";
    const indicator = document.createElement("span");
    indicator.className = profile === active ? "account-indicator on" : "account-indicator";
    indicator.setAttribute("role", "img");
    const status = profile === active ? "Active account, last observed" : active ? "Inactive account" : "Account status unknown";
    indicator.setAttribute("aria-label", status); indicator.title = status;
    const identity = document.createElement("div");
    const label = document.createElement("h2"); label.className = "account-name"; label.textContent = profile.label;
    const detail = document.createElement("p"); detail.className = "account-subtitle";
    detail.textContent = profile.domain; detail.title = `Last saved ${localDate(profile.captured_at)}`;
    identity.append(label, detail); person.append(indicator, identity);
    const quotas = document.createElement("div"); quotas.className = "quota-pair"; quotas.dataset.quotaProfile = profile.id;
    const actions = document.createElement("div"); actions.className = "row-actions";
    const primary = profile === active
      ? actionButton("Open Codex", () => void vaultCommand("open_codex"))
      : actionButton("Switch", () => void vaultCommand("switch_profile", {profileId: profile.id}));
    if (profile !== active) primary.className = "primary";
    primary.setAttribute("aria-label", profile === active ? `Open Codex, last observed ${profile.label}` : `Switch to ${profile.label}`);
    const menu = document.createElement("details"); menu.className = "account-menu";
    const toggle = document.createElement("summary"); toggle.textContent = "⋯"; toggle.setAttribute("aria-label", `Actions for ${profile.label}`);
    const items = document.createElement("div"); items.className = "menu-items";
    const menuAction = (name, callback) => actionButton(name, () => { menu.open = false; callback(); });
    items.append(menuAction("Rename", () => confirmAction("Rename account", "Change this account's label in your vault.", "Save label", "rename_profile", {profileId: profile.id}, true, profile.label)));
    items.append(menuAction("Sign in again", () => void vaultCommand("reauthenticate_profile", {profileId: profile.id})));
    items.append(menuAction("Remove account", () => confirmAction(`Remove ${profile.label}?`, "Remove the saved copy from your vault. Active accounts and accounts needed for recovery cannot be removed.", "Remove account", "remove_profile", {profileId: profile.id})));
    menu.append(toggle, items); actions.append(primary, menu); row.append(person, quotas, actions); return row;
  });
  byId("profiles").replaceChildren(...rows);
  byId("profiles").hidden = rows.length === 0;
  byId("profile-state").hidden = rows.length > 0 || recovery;
  byId("table-heading").hidden = rows.length === 0;
  byId("usage-controls").hidden = !open || rows.length === 0;
  byId("account-count").hidden = !open;
  byId("profile-heading").textContent = recovery ? "Let's finish this up." : open ? "Your first account." : vaultExists === false ? "Your accounts, together." : "Welcome back.";
  byId("profile-description").textContent = open ? "Save your current account to get started." : "";
  byId("profile-description").hidden = !open;
  byId("setup-button").hidden = recovery;
  byId("setup-button").textContent = open ? "Save current account" : vaultExists === true ? "Open vault" : vaultExists === false ? "Create vault" : "Checking vault...";
  byId("vault-message").textContent = recovery ? "Vault opened; recovery is required." : open
    ? `Vault opened. ${profiles.length} saved account${profiles.length === 1 ? "" : "s"}.` : "Vault closed.";
  byId("open-vault").hidden = open || recovery || vaultExists !== true;
  byId("create-vault").hidden = open || recovery || vaultExists !== false;
  byId("close-vault").hidden = !open && !recovery;
  byId("recovery-panel").hidden = !recovery;
  byId("recover-vault").hidden = !["control_repair", "commit_pending", "cleanup_pending"].includes(result.recovery);
  byId("recovery-detail").textContent = result.recovery === "blocked"
    ? "The vault cannot safely choose a recovery path. Keep the existing vault intact and seek assistance. Account changes are blocked."
    : result.recovery === "switch_pending" ? "Use the available recovery actions below to complete the unfinished operation."
      : "Recover the interrupted protected storage update before making another account change.";
  renderOperations(result.operations || [], profiles);
  refreshQuotaDisplays();
  updateControls();
  if (focusedProfile) [...byId("profiles").children].find((row) => row.dataset.profileId === focusedProfile)?.querySelector("summary")?.focus();
}

function quotaLabel(window, fallback) {
  const minutes = window?.duration_mins;
  if (minutes === 300) return "5 hours";
  if (minutes === 10080) return "Weekly";
  if (Number.isSafeInteger(minutes) && minutes > 0) return minutes % 60 === 0 ? `${minutes / 60} hours` : `${minutes} min`;
  return fallback;
}
function quotaRemaining(window, now = Date.now() / 1000) {
  if (!window || !Number.isInteger(window.used_percent) || window.used_percent < 0 || window.used_percent > 100) return null;
  if (Number.isSafeInteger(window.resets_at) && window.resets_at <= now) return null;
  return 100 - window.used_percent;
}
function renderQuotaPair(container, health, error) {
  container.replaceChildren();
  const expired = expiredUsage.has(container.dataset.quotaProfile);
  const visibleHealth = expired ? null : health;
  const windows = [["Usage", visibleHealth?.primary], ["Usage", visibleHealth?.secondary]].filter(([, value]) => value);
  container.classList.toggle("single-window", windows.length === 1);
  for (const [fallback, value] of windows) {
    const cell = document.createElement("div");
    const top = document.createElement("div"); top.className = "quota-top";
    const label = document.createElement("span"); label.className = "quota-label"; label.textContent = quotaLabel(value, fallback);
    const remaining = quotaRemaining(value);
    const amount = document.createElement("span"); amount.className = "quota-value"; amount.textContent = remaining === null ? "N/A" : `${remaining}%`;
    const bar = document.createElement("div"); bar.className = "quota-bar";
    bar.setAttribute("aria-label", `${label.textContent}: ${remaining === null ? "unknown" : `${remaining}% remaining`}`);
    if (remaining !== null) {
      bar.setAttribute("role", "meter"); bar.setAttribute("aria-valuemin", "0"); bar.setAttribute("aria-valuemax", "100"); bar.setAttribute("aria-valuenow", String(remaining));
      const fill = document.createElement("div"); fill.className = "quota-fill"; fill.style.width = `${remaining}%`; bar.append(fill);
    }
    const reset = Number.isSafeInteger(value?.resets_at) ? `Resets ${localDate(value.resets_at)}` : "Reset time unavailable";
    top.title = reset;
    bar.title = reset;
    top.append(label, amount); cell.append(top, bar); container.append(cell);
  }
  const now = Date.now() / 1000;
  const resets = [visibleHealth?.primary, visibleHealth?.secondary].map((window) => window?.resets_at).filter((reset) => Number.isSafeInteger(reset));
  const elapsed = resets.some((reset) => reset <= now);
  const future = resets.filter((reset) => reset > now);
  const freshness = document.createElement("span");
  freshness.className = error ? "usage-freshness stale" : "usage-freshness";
  freshness.textContent = error || (elapsed ? "Updating usage..."
    : future.length ? `Resets in ${shortInterval(Math.min(...future) - now)}` : visibleHealth ? "Reset time unavailable" : "Waiting for usage");
  const banked = visibleHealth?.available_resets;
  if (!error && Number.isSafeInteger(banked) && banked > 0) {
    freshness.textContent += ` · ${banked} reset${banked === 1 ? "" : "s"} available`;
  }
  freshness.title = health ? `Last successful update: ${localDate(health.checked_at)}` : "No usage observation available";
  container.setAttribute("aria-label", `Usage. ${freshness.textContent}`);
  container.append(freshness);
}

function shortInterval(seconds) {
  const minutes = Math.max(1, Math.ceil(seconds / 60));
  if (minutes < 60) return `${minutes}m`;
  if (minutes < 1440) return `${Math.ceil(minutes / 60)}h`;
  return `${Math.ceil(minutes / 1440)}d`;
}
function refreshQuotaDisplays() {
  document.querySelectorAll("[data-quota-profile]").forEach((cell) => renderQuotaPair(cell, observedHealth.get(cell.dataset.quotaProfile), usageErrors.get(cell.dataset.quotaProfile)));
  byId("usage-note").textContent = Date.now() < usageBackoffUntil
    ? `Refresh paused for ${shortInterval((usageBackoffUntil - Date.now()) / 1000)}`
    : usageBusy ? "Refreshing usage..." : "Refreshes every minute";

}

function pauseUsagePolling() {
  usageEpoch += 1;
  clearTimeout(usageTimer);
  usageTimer = null;
}

function scheduleUsagePolling(delay = 60000) {
  clearTimeout(usageTimer);
  usageTimer = null;
  if (document.hidden || vaultState !== "open" || vaultBusy || usageBusy || displayedProfiles.length === 0) return;
  usageTimer = setTimeout(() => void refreshUsage(), Math.max(delay, usageBackoffUntil - Date.now()));
}

async function refreshUsage(manual = false) {
  if (usageBusy || vaultBusy || vaultState !== "open" || document.hidden || Date.now() < usageBackoffUntil) return;
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) return;
  clearTimeout(usageTimer);
  usageTimer = null;
  const epoch = usageEpoch;
  const ids = displayedProfiles.map((profile) => profile.id).filter((id) => manual || id === activeProfileId || !expiredUsage.has(id));
  const valid = () => epoch === usageEpoch && !vaultBusy && vaultState === "open" && !document.hidden;
  usageBusy = true;
  updateControls();
  try {
    for (const id of ids) {
      if (!valid()) break;
      try {
        const request = Promise.resolve(invoke("refresh_usage", {profileId: id}));
        usageRequest = request;
        let snapshot;
        try {
          snapshot = await request;
        } finally {
          if (usageRequest === request) usageRequest = null;
        }
        if (!valid()) break;
        // A background response supplies usage only. It must never restore stale
        // accounts, active status or recovery controls after an explicit action.
        const health = snapshot.health;
        if (health?.profile === id && displayedProfiles.some((profile) => profile.id === id)) {
          const previous = observedHealth.get(id);
          if (!previous || health.checked_at >= previous.checked_at) observedHealth.set(id, health);
          usageErrors.delete(id);
          expiredUsage.delete(id);
        } else {
          usageErrors.set(id, "Usage unavailable");
        }
      } catch (error) {
        if (!valid()) break;
        if (["vault_busy", "E_BUSY", "vault_closing", "vault_not_open"].includes(error)) break;
        if (error === "E_USAGE_THROTTLED") {
          usageBackoffUntil = Date.now() + 300000;
          usageErrors.set(id, "Refresh paused, try again shortly");
          break;
        }
        if (error === "E_USAGE_EXPIRED") expiredUsage.add(id);
        usageErrors.set(id, error === "E_USAGE_EXPIRED" ? "Sign in to refresh"
          : error === "E_EXTERNAL_CHANGE" ? "Save current account to update" : "Usage unavailable");
      }
      if (valid()) refreshQuotaDisplays();
    }
  } finally {
    usageBusy = false;
    if (valid()) refreshQuotaDisplays();
    updateControls();
    scheduleUsagePolling();
  }
}

function checkUsageResets() {
  if (document.hidden || vaultState !== "open" || vaultBusy || usageBusy || Date.now() < usageBackoffUntil) return;
  const now = Date.now() / 1000;
  let due = false;
  for (const profile of displayedProfiles) {
    if (expiredUsage.has(profile.id)) continue;
    const health = observedHealth.get(profile.id);
    for (const name of ["primary", "secondary"]) {
      const reset = health?.[name]?.resets_at;
      const key = `${profile.id}:${name}`;
      if (Number.isSafeInteger(reset) && reset <= now && refreshedResets.get(key) !== reset) {
        refreshedResets.set(key, reset);
        due = true;
      }
    }
  }
  // One attempt per observed reset, even if the service returns the old window.
  if (due) scheduleUsagePolling(0);
}

function localDate(seconds) {
  if (!Number.isSafeInteger(seconds) || seconds <= 0 || seconds > 253402300799) return "Not recorded";
  return new Date(seconds * 1000).toLocaleString();
}

const vaultErrors = Object.freeze({
  vault_missing: "No vault was found. Use Create vault to set one up.",
  vault_exists: "A vault already exists. Use Open vault; it will not be overwritten.",
  vault_busy: "The vault is busy or open in another application window. Close that window or try again after the current operation finishes.",
  vault_closing: "Window closure was requested. Review the last operation result, then close this window again. Reopen Codex Accounts to start another operation.",
  vault_access_denied: "Windows denied access to the protected vault.",
  vault_unsafe_path: "The vault location could not be opened safely.",
  vault_key_unavailable: "The vault encryption key is unavailable. Existing data has not been replaced.",
  vault_corrupt: "The vault could not be authenticated. Existing data has not been replaced.",
  vault_recovery_required: "The vault needs recovery. Use the recovery controls below.",
  login_cleanup_required: "An interrupted sign-in still needs temporary-file cleanup. Quit Codex clients normally, then open this vault again. Keep the protected vault intact.",
  vault_later_startup_required: "Close and reopen the vault to finish cleanup.",
  vault_changed: "Vault storage changed during the operation. Close the vault and reopen it.",
  vault_input_limit: "Close and reopen the vault to clear completed recovery records. Unfinished operations are kept.",
  vault_platform_unsupported: "Protected vault storage is currently available only on Windows x64.",
  vault_switch_pending: "An unfinished switch needs recovery before another account change.",
  vault_not_open: "Open or create your protected vault first.",
  vault_active_unknown: "The active account is unknown, so this saved account cannot safely be removed.",
  vault_active_profile: "The currently installed account cannot be removed from the vault.",
  vault_profile_referenced: "Recovery still needs this account. Complete recovery before removing it.",
  invalid_label: "Enter a label of 1–80 characters without control characters.",
  invalid_selection: "This account or operation is no longer selectable. Refresh the account list.",
  E_COMPAT_UNKNOWN: "The installed runtime or account storage could not be established. Unsupported configuration will not be changed.",
  E_BACKEND_UNSUPPORTED: "This credential store is unsupported. Only the official file store is supported; its configuration will not be changed.",
  E_POLICY_DENIED: "The operation is blocked by the current workspace or sign-in policy.",
  E_CLIENTS_RUNNING: "Codex clients are still running. Save your work, fully quit Codex Desktop and its ChatGPT host, and close CLI/editor sessions. Keep this Accounts window open, then try again.",
  E_CONSENT_REQUIRED: "Select the specific account action again to continue.",
  E_BUSY: "Another account operation is in progress. Wait for it to finish.",
  E_HELPER_STUCK: "The official helper has not stopped. Further changes are blocked; review recovery status.",
  E_WRITE_FAILED: "The account could not be installed. Review the recorded restoration or recovery result below.",
  E_LOGIN_REQUIRED: "This account needs sign-in. Choose Sign in again on the saved account to get a fresh official session without signing out of the installed account.",
  E_LOGIN_TIMEOUT: "Sign-in timed out. Start Add account again when you are ready to complete browser sign-in.",
  E_LOGIN_FAILED: "The official sign-in did not complete. Your current Desktop account is unchanged. Start Add account again to retry.",
  E_LOGIN_URL_REFUSED: "The official runtime returned an unsupported sign-in address. It was not opened. Your current Desktop account is unchanged.",
  E_BROWSER_OPEN_FAILED: "The official sign-in could not be opened in your browser. Check your default browser and try Add account again.",
  E_TEMP_CLEANUP_FAILED: "Sign-in did not finish because temporary files could not be cleaned up. The new sign-in was not saved. Quit Codex clients, close and reopen the vault, then retry.",
  login_not_running: "Sign-in is no longer running. Wait for its final result.",
  switch_not_running: "The switch is no longer running. Wait for its recorded result.",
  E_RECOVERY_REQUIRED: "An unfinished operation needs recovery. Use the controls below.",
  E_CANCELLED: "The operation was cancelled. Review its recorded result below.",
  E_LAUNCH_FAILED: "Desktop could not be reopened. Review the installed-account result, then open Codex manually and check the account and workspace.",
  E_OBSERVATION_UNAVAILABLE: "The official runtime could not verify the account. Review the recorded result before proceeding.",
  E_VAULT_CORRUPT: "The protected account data could not be authenticated. It has not been replaced.",
  E_CAPTURE_CURRENT_FIRST: "Save your current signed-in account before switching. This keeps your previous account available.",
  E_ALREADY_CURRENT: "This account is already installed. Open Codex and check its account and workspace.",
  E_AUTH_FORMAT_UNSUPPORTED: "The current sign-in format is unsupported. Use an ordinary ChatGPT sign-in through the official Codex app; API-key and agent credentials cannot be saved here.",
  E_STORAGE_UNSAFE: "The account or vault location has unsupported protection or links. It will not be modified.",
  E_KEY_UNAVAILABLE: "The vault encryption key is unavailable. Existing data has not been replaced.",
  E_STORAGE_LIMIT: "The account data exceeds a supported storage limit.",
  E_IDENTITY_MISMATCH: "The saved account does not match the identity observed by the official runtime. Review the operation result before trying again.",
  E_STORAGE_FAILED: "Protected storage could not complete the operation. Review recovery status before proceeding.",
  E_NOT_FOUND: "The selected account or operation no longer exists. Refresh the vault.",
  E_RECOVERY_CHOICE_UNAVAILABLE: "This recovery choice is unavailable for the recorded state. Refresh and review the available recovery actions.",
  E_ALREADY_COMMITTED: "This switch has already committed. Refresh the recorded result and check the account in Desktop.",
  E_UNSUPPORTED_PLATFORM: "Account operations are supported on Windows x64 only.",
  E_DIAGNOSTICS_EXISTS: "A file already exists at the selected location. Export again and choose a new filename.",
  E_DIAGNOSTICS_WRITE: "Diagnostics could not be saved. Export again and choose a writable location.",
  E_DIAGNOSTICS_DIALOG: "The native Save dialog is unavailable. Try exporting again after closing other dialogs."
});

function operationError(error) {
  if (typeof error !== "string") return "The operation could not finish. Refresh the vault to inspect its recorded state.";
  return Object.hasOwn(vaultErrors, error) ? vaultErrors[error]
    : Object.hasOwn(inventoryErrors, error) ? inventoryErrors[error]
      : "The operation could not finish. Refresh the vault to inspect its recorded state.";
}

async function vaultCommand(command, args = {}, refreshUsageAfter = false) {
  if (vaultBusy) return;
  const previouslyOpen = vaultState === "open";
  pauseUsagePolling();
  if (byId("settings-dialog").open && command !== "export_sanitized_diagnostics") closeSettings();
  vaultBusy = true;
  updateControls();
  loginPending = command === "add_account" || command === "reauthenticate_profile";
  switchPending = command === "switch_profile";
  if (["switch_profile", "recover_switch", "recover_vault", "reauthenticate_profile"].includes(command)) renderActiveAccount("pending", null);
  const currentLogin = ++loginEpoch;
  loginCancelling = false;
  switchCancelling = false;
  byId("login-panel").hidden = !loginPending;
  byId("login-heading").textContent = command === "reauthenticate_profile" ? "Sign in again" : "Add an account";
  byId("login-impact").textContent = command === "reauthenticate_profile"
    ? "Keep Codex Accounts open. The fresh sign-in must match the selected account. If that account is currently installed, its sign-in will be repaired; otherwise the fresh session is saved in the vault."
    : "Keep Codex Accounts open. Your current Desktop account stays installed. The new sign-in is saved separately in your protected vault.";
  byId("switch-panel").hidden = !switchPending;
  const mutating = ["capture_current", "add_account", "reauthenticate_profile", "switch_profile", "recover_switch", "recover_vault", "remove_profile", "rename_profile"].includes(command);
  const message = loginPending ? "Preparing official sign-in. Complete sign-in in your browser when it opens, then wait here while the account is saved."
    : mutating ? "Applying account changes..." : ({
      open_vault: "Opening vault...", create_vault: "Creating vault...",
      close_vault: "Closing vault...", list_profiles: "Refreshing accounts...",
      open_codex: "Opening Codex...", export_sanitized_diagnostics: "Exporting report..."
    }[command] || "Working...");
  if (loginPending) byId("login-message").textContent = message;
  if (switchPending) byId("switch-message").textContent = "Checking and switching accounts. Wait for the recorded result before opening Codex…";
  updateControls();
  if (command !== "export_sanitized_diagnostics") byId("vault-message").textContent = message;
  if (command === "export_sanitized_diagnostics") byId("diagnostics-export-message").textContent = "Choose a destination in the native Save dialog.";
  activity.textContent = message;
  const invoke = window.__TAURI__?.core?.invoke;
  let succeeded = false;
  try {
    if (!invoke) throw "vault_connection_unavailable";
    // The epoch already stopped the cycle. Join only its in-flight request so
    // the worker can admit this explicit action without a transient busy error.
    const outstandingUsage = usageRequest;
    if (outstandingUsage) {
      try { await outstandingUsage; } catch { /* The explicit action owns its result. */ }
    }
    const pending = invoke(command, args);
    if (loginPending || switchPending) setTimeout(() => void updateLoginStatus(currentLogin), 750);
    const result = await pending;
    renderProfiles(result);
    if (command === "create_vault" && result.state === "open") {
      byId("vault-message").textContent = "Vault created.";
    }
    if (command === "open_codex") byId("vault-message").textContent = "Codex opened.";
    if (command === "capture_current") byId("vault-message").textContent = "Current account saved.";
    if (command === "add_account") byId("vault-message").textContent = "Account added. Choose Switch to use it.";
    if (command === "reauthenticate_profile") {
      const outcome = result.reauthentication;
      byId("vault-message").textContent = outcome && outcome.installation === null
        ? "Fresh sign-in saved for the inactive account. The currently installed account was not replaced."
        : ["awaiting_confirmation", "finished"].includes(outcome?.installation?.phase)
          ? "Current account sign-in updated."
          : "Sign-in needs attention. Review the result below.";
    }
    if (command === "remove_profile") byId("vault-message").textContent = "Inactive saved account removed.";
    if (command === "rename_profile") byId("vault-message").textContent = "Account renamed.";
    if (command === "export_sanitized_diagnostics") {
      const exported = result.diagnostics_exported === true;
      const cancelled = result.diagnostics_exported === false;
      const exportMessage = exported ? "Sanitized diagnostics exported."
        : cancelled ? "Export cancelled. No diagnostics file was saved."
          : "No export result was returned. Check the selected destination before trying again.";
      byId("diagnostics-export-message").textContent = exportMessage;
      byId("vault-message").textContent = exportMessage;
    }
    if (command === "switch_profile" || command === "recover_switch") byId("vault-message").textContent = "Switch finished.";
    activity.textContent = ["list_profiles", "open_vault", "close_vault"].includes(command) ? "" : byId("vault-message").textContent;
    succeeded = true;
  } catch (error) {
    // Read status once after a failed action, never retry a mutation.
    let refreshed = false;
    if (invoke && command !== "list_profiles") {
      try { renderProfiles(await invoke("list_profiles")); refreshed = true; } catch { /* fixed message below */ }
    }
    // Do not keep displaying stale labels after an unsuccessful read.
    if (!refreshed) {
    vaultState = "unknown";
    byId("account-count").textContent = "?";
    vaultExists = null;
    byId("setup-button").textContent = "Vault unavailable";
    byId("table-heading").hidden = true;
    byId("usage-controls").hidden = true;
    observedHealth.clear(); usageErrors.clear(); expiredUsage.clear(); refreshedResets.clear(); displayedProfiles = []; activeProfileId = null;
    refreshQuotaDisplays();
    byId("attention-operations").replaceChildren(); byId("attention-panel").hidden = true;
    renderActiveAccount("unknown", null);
    byId("profiles").replaceChildren();
    byId("profiles").hidden = true;
    byId("profile-state").hidden = false;
    byId("profile-heading").textContent = "Account list unavailable";
    byId("profile-description").hidden = false;
    byId("profile-description").textContent = "Try reopening from Settings.";
    // A failed metadata read may still hold an opened vault. Offer release.
    byId("close-vault").hidden = false;
    byId("recovery-panel").hidden = true;
    }
    byId("vault-message").textContent = error === "vault_connection_unavailable"
      ? "Open the Windows application to use protected vault storage."
      : command === "open_codex" && error === "E_BUSY"
        ? "Codex is already running. Use its existing window."
      : command === "add_account" && error === "E_CANCELLED"
        ? "Sign-in cancelled. Your current Desktop account is unchanged."
        : command === "reauthenticate_profile" && error === "E_CANCELLED"
          ? "Sign-in cancelled. Review the recorded operation result before continuing; any installation or recovery work is shown below."
        : command === "add_account" && error === "E_ALREADY_CURRENT"
          ? "This is already your current account. Choose Sign in again on that saved account to repair its sign-in."
        : command === "reauthenticate_profile" && error === "E_IDENTITY_MISMATCH"
          ? "The browser sign-in did not match the selected saved account. That sign-in was not saved. Choose Sign in again with the correct identity, or use Add account to keep a different account separately."
          : operationError(error);
    activity.textContent = byId("vault-message").textContent;
    if (command === "export_sanitized_diagnostics") byId("diagnostics-export-message").textContent = byId("vault-message").textContent;
  } finally {
    if ((loginPending || switchPending) && invoke) {
      try {
        if (await invoke(loginPending ? "read_add_account_status" : "read_switch_status") === "closing") {
          byId("vault-message").textContent += " The operation has ended. Review this result, then close the window again.";
          activity.textContent = byId("vault-message").textContent;
        }
      } catch { /* Preserve the original operation result. */ }
    }
    loginPending = false;
    loginCancelling = false;
    switchPending = false;
    switchCancelling = false;
    byId("login-panel").hidden = true;
    byId("switch-panel").hidden = true;
    vaultBusy = false;
    if (command === "export_sanitized_diagnostics" && vaultState === "open") {
      byId("vault-message").textContent = `${displayedProfiles.length} saved account${displayedProfiles.length === 1 ? "" : "s"}.`;
    }
    updateControls();
    const signInChanged = succeeded && ["capture_current", "reauthenticate_profile", "switch_profile", "recover_switch"].includes(command);
    if (signInChanged) {
      if (activeProfileId) usageErrors.delete(activeProfileId);
      if (command === "reauthenticate_profile") {
        usageErrors.delete(args.profileId);
        if (expiredUsage.delete(args.profileId)) observedHealth.delete(args.profileId);
      }
    }
    scheduleUsagePolling(succeeded && (!previouslyOpen || signInChanged || refreshUsageAfter) ? 0 : 60000);
    if (succeeded) {
      const message = activity.textContent;
      setTimeout(() => { if (!vaultBusy && activity.textContent === message) activity.textContent = ""; }, 5000);
    }
  }
}

const loginStages = Object.freeze({
  preparing: "Preparing official sign-in. Keep this window open…",
  waiting_for_browser: "Complete sign-in to the account you want in your browser. Return here when finished.",
  saving: "Sign-in received. Saving the account in your protected vault…",
  cleaning: "Cleaning up temporary sign-in files. Keep this window open…",
  finished: "Finishing the operation. Wait for the final result…",
  closing: "Closing was requested. Stopping sign-in and cleaning up; keep this window open. Review the final result, then close the window again."
});

async function updateLoginStatus(epoch) {
  if ((!loginPending && !switchPending) || epoch !== loginEpoch) return;
  try {
    const stage = await window.__TAURI__.core.invoke(loginPending ? "read_add_account_status" : "read_switch_status");
    if ((!loginPending && !switchPending) || epoch !== loginEpoch) return;
    if (loginPending && !loginCancelling && Object.hasOwn(loginStages, stage)) byId("login-message").textContent = loginStages[stage];
    if (switchPending && stage === "closing") byId("switch-message").textContent = "Closing was requested. Cancelling the switch and finishing required recovery work. Review the final result, then close the window again.";
  } catch { /* The original request owns the final result; do not retry it. */ }
  if ((loginPending || switchPending) && epoch === loginEpoch) setTimeout(() => void updateLoginStatus(epoch), 750);
}

async function refreshInventory() {
  if (refreshing || vaultBusy) return;
  refreshing = true;
  refresh.disabled = true;
  refresh.textContent = "Checking…";
  try {
    const invoke = window.__TAURI__?.core?.invoke;
    if (!invoke) {
      unavailable("Open the Windows application to read local installation metadata.");
      return;
    }
    const status = await invoke("read_status");
    renderInventory(status);
  } catch (error) {
    // Errors are fixed categories; never print arbitrary IPC or system text.
    unavailable(error === "inventory_busy"
      ? "An inventory check is already running. Try again after it finishes."
      : "The application could not finish reading installation metadata. Try refreshing inventory.");
  } finally {
    refreshing = false;
    refresh.disabled = vaultBusy;
    refresh.textContent = "Refresh";
  }
}

function actionButton(label, action, requiresOpen = true) {
  const button = document.createElement("button");
  button.type = "button"; button.textContent = label;
  button.dataset.vaultAction = "true";
  button.dataset.requiresOpen = String(requiresOpen);
  button.addEventListener("click", action);
  return button;
}

function confirmAction(title, detail, submit, command, args = {}, label = false, initialLabel = "") {
  if (vaultBusy) return;
  dialogTrigger = byId("settings-dialog").open ? byId("settings-button") : document.activeElement;
  if (byId("settings-dialog").open) closeSettings();
  pendingAction = { command, args, label };
  byId("add-account-options").hidden = true;
  byId("dialog-heading").textContent = title;
  byId("dialog-detail").textContent = detail;
  byId("dialog-submit").textContent = submit;
  byId("dialog-submit").hidden = !command;
  byId("dialog-cancel").textContent = command ? "Cancel" : "Got it";
  byId("label-field").hidden = !label;
  byId("account-label").required = label;
  byId("account-label").value = initialLabel;
  byId("dialog-error").textContent = "";
  byId("action-dialog").showModal();
  (label ? byId("account-label") : byId("dialog-cancel")).focus();
}

function closeDialog() {
  pendingAction = null;
  byId("action-dialog").close();
  if (dialogTrigger?.isConnected) dialogTrigger.focus();
}

byId("dialog-cancel").addEventListener("click", closeDialog);
byId("action-dialog").addEventListener("cancel", () => { pendingAction = null; });
byId("action-form").addEventListener("submit", (event) => {
  event.preventDefault();
  if (!pendingAction?.command || vaultBusy) return;
  const {command, args, label} = pendingAction;
  if (label) {
    const value = byId("account-label").value.trim();
    if (!value || [...value].length > 80 || /[\u0000-\u001f\u007f-\u009f]/u.test(value)) {
      byId("dialog-error").textContent = vaultErrors.invalid_label;
      byId("account-label").focus(); return;
    }
    args.label = value;
  }
  closeDialog();
  void vaultCommand(command, args);
});

const phaseLabels = Object.freeze({awaiting_confirmation: "Account switched", finished: "Operation finished", restored: "Previous account restored", cancelled: "Switch cancelled", conflict: "Recovery conflict", recovery: "Recovery required", needs_login: "Fresh sign-in required"});
const acceptanceLabels = Object.freeze({accepted: "Accepted by official runtime", rejected: "Sign-in required", unknown: "Not checked", unavailable: "Could not verify"});
const launchLabels = Object.freeze({opened: "Opened", failed: "Could not open", not_requested: "Not reopened"});

function renderOperations(operations, profiles) {
  byId("attention-operations").replaceChildren();
  byId("attention-panel").hidden = true;
  [...operations].reverse().forEach((operation, index) => {
    const attention = operation.can_recover || (index === 0 && (operation.phase === "needs_login" || operation.primary_error || operation.restoration_error || operation.desktop_launch === "failed"));
    if (!attention) return;
    const row = document.createElement("li");
    const heading = document.createElement("h2");
    heading.textContent = phaseLabels[operation.phase] || "Unfinished switch";
    const installed = profiles.find((profile) => profile.id === operation.installed_profile);
    const account = document.createElement("p");
    account.textContent = installed ? `Installed account: ${installed.label}` : "Review the recorded operation result below.";
    const facts = document.createElement("dl"); facts.className = "operation-facts";
    for (const [name, value] of [["Credentials", acceptanceLabels[operation.credential_acceptance] || "Not checked"], ["Desktop", launchLabels[operation.desktop_launch] || "Not checked"]]) {
      const group = document.createElement("div");
      const term = document.createElement("dt"); const detail = document.createElement("dd");
      term.textContent = name; detail.textContent = value;
      group.append(term, detail); facts.append(group);
    }
    row.append(heading, account, facts);
    for (const code of [operation.primary_error, operation.restoration_error]) {
      if (!code) continue;
      const error = document.createElement("p"); error.className = "error";
      error.textContent = operationError(code); row.append(error);
    }
    const actions = document.createElement("div"); actions.className = "actions";
    if (operation.can_recover) {
      const note = document.createElement("p");
      note.textContent = operation.phase === "conflict"
        ? "Files have changed. Recovery will refuse to overwrite an unrecognized state. Close all Codex clients before choosing an action."
        : operation.can_restore === true
          ? "Close all Codex clients, then restore the previous account or finish the selected switch."
          : "Close all Codex clients, then finish this operation. The superseded sign-in cannot be restored.";
      row.append(note);
      for (const [choice, text] of [["restore", "Restore previous account"], ["finish", "Finish switch"]]) {
        if (choice === "restore" && operation.can_restore !== true) continue;
        actions.append(actionButton(text, () => void vaultCommand("recover_switch", {operationId: operation.id, choice}), false));
      }
    } else if (operation.phase === "needs_login") {
      const note = document.createElement("p");
      note.textContent = "The newest sign-in was retained, but the official runtime rejected it. Sign in again to this account; the superseded session will not be restored.";
      row.append(note);
      if (installed) actions.append(actionButton("Sign in again", () => void vaultCommand("reauthenticate_profile", {profileId: installed.id})));
    }
    row.append(actions);
    byId("attention-operations").append(row);
    byId("attention-panel").hidden = false;
  });
}

function openAddAccount() {
  if (vaultBusy || vaultState !== "open") return;
  confirmAction("Add account", "Close Codex before saving or signing in.", "Save account", "capture_current", {}, true);
  byId("add-account-options").hidden = false;
  document.querySelector('input[name="account-source"][value="capture_current"]').checked = true;
}
byId("add-account-options").addEventListener("change", (event) => {
  if (!pendingAction || event.target.name !== "account-source") return;
  pendingAction.command = event.target.value;
  byId("dialog-submit").textContent = event.target.value === "capture_current" ? "Save account" : "Sign in";
});
byId("add-account").addEventListener("click", openAddAccount);
byId("cancel-login").addEventListener("click", async () => {
  if (!loginPending || loginCancelling) return;
  loginCancelling = true;
  updateControls();
  byId("login-message").textContent = "Stopping sign-in and cleaning up. Keep this window open until the final result appears…";
  try {
    await window.__TAURI__.core.invoke("cancel_add_account");
  } catch (error) {
    if (!loginPending) return;
    loginCancelling = false;
    byId("login-message").textContent = operationError(error);
    updateControls();
  }
});
byId("cancel-switch").addEventListener("click", async () => {
  if (!switchPending || switchCancelling) return;
  switchCancelling = true;
  updateControls();
  byId("switch-message").textContent = "Cancellation requested. Wait while the app finishes any required recovery. A committed switch will stay installed; review the final result.";
  try {
    await window.__TAURI__.core.invoke("cancel_switch");
  } catch (error) {
    if (!switchPending) return;
    switchCancelling = false;
    byId("switch-message").textContent = operationError(error);
    updateControls();
  }
});
byId("recover-vault").addEventListener("click", () => confirmAction("Recover vault state?", "Recover the interrupted protected storage update using the last authenticated vault state. Existing account data will be preserved where recovery permits.", "Recover vault state", "recover_vault"));
byId("export-diagnostics").addEventListener("click", () => void vaultCommand("export_sanitized_diagnostics"));

refresh.addEventListener("click", refreshInventory);
byId("open-vault").addEventListener("click", () => void vaultCommand("open_vault"));
byId("create-vault").addEventListener("click", () => void vaultCommand("create_vault"));
byId("close-vault").addEventListener("click", () => void vaultCommand("close_vault"));
byId("refresh-profiles").addEventListener("click", () => void vaultCommand("list_profiles", {}, true));
// Native details menus remain keyboard accessible and dismiss like utility menus.
document.addEventListener("pointerdown", (event) => {
  document.querySelectorAll(".account-menu[open]").forEach((menu) => { if (!menu.contains(event.target)) menu.open = false; });
});
document.addEventListener("keydown", (event) => {
  if (event.key !== "Escape" || byId("action-dialog").open || byId("settings-dialog").open) return;
  const menu = document.querySelector(".account-menu[open]");
  if (menu) { menu.open = false; menu.querySelector("summary").focus(); event.preventDefault(); }
});
document.addEventListener("visibilitychange", () => {
  pauseUsagePolling();
  if (!document.hidden) { refreshQuotaDisplays(); updateControls(); scheduleUsagePolling(); }
});
setInterval(() => {
  if (!document.hidden) { refreshQuotaDisplays(); updateControls(); checkUsageResets(); }
}, 30000);
void refreshInventory();
void vaultCommand("list_profiles");
