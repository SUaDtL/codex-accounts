const { chromium } = require("playwright");
const fs = require("fs");
const path = require("path");
const assert = require("assert");

const root = process.cwd();
const out = path.join(root, "reports/ui/tray");
fs.mkdirSync(out, { recursive: true });

(async () => {
  const browser = await chromium.launch({ headless: true });
  try {
    const page = await browser.newPage({ viewport: { width: 380, height: 270 } });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.route("**/*", async route => {
      const name = new URL(route.request().url()).pathname.slice(1) || "tray.html";
      if (!["tray.html", "tray.css", "tray.js"].includes(name)) return route.abort();
      await route.fulfill({
        body: fs.readFileSync(path.join(root, "crates/desktop-app/assets", name)),
        contentType: name.endsWith("html") ? "text/html" : name.endsWith("css") ? "text/css" : "application/javascript",
      });
    });
    await page.addInitScript(() => {
      const now = Math.floor(Date.now() / 1000);
      window.calls = [];
      window.listeners = {};
      window.vaultBusy = false;
      window.fixture = {
        state: "open",
        vault_exists: true,
        protection: "windows_user",
        last_known_active_profile: "a",
        profiles: [
          { id: "a", label: "Erica", domain: "ChatGPT", captured_at: now },
          { id: "b", label: "Brennon", domain: "ChatGPT", captured_at: now },
        ],
        operations: [], recovery: "none", health: null,
      };
      window.__TAURI__ = {
        event: { listen: async (name, handler) => { listeners[name] = handler; return () => { delete listeners[name]; }; } },
        core: { invoke: async (command, args) => {
          calls.push({ command, args });
          if (command === "tray_page_capacity") return 5;
          if (["list_profiles", "open_vault", "refresh_usage", "switch_profile"].includes(command) && vaultBusy) throw "vault_busy";
          if (command === "list_profiles" && window.failList) throw window.failList;
          if (command === "refresh_usage") {
            vaultBusy = true;
            const response = { ...fixture, health: { profile: args.profileId, source: "usage", checked_at: now,
              available_resets: null, primary: { used_percent: args.profileId === "a" ? 18 : 100,
                duration_mins: 10080, resets_at: now + (args.profileId === "a" ? 604800 : 518400) }, secondary: null } };
            if (window.holdUsage) return new Promise(resolve => { window.releaseUsage = () => { window.holdUsage = false; vaultBusy = false; resolve(response); }; });
            vaultBusy = false;
            return response;
          }
          if (command === "switch_profile") {
            const response = { ...fixture, last_known_active_profile: args.profileId };
            if (window.holdSwitch) {
              vaultBusy = true;
              return new Promise(resolve => { window.releaseSwitch = () => { window.holdSwitch = false; vaultBusy = false; resolve(response); }; });
            }
            return response;
          }
          return structuredClone(fixture);
        } },
      };
    });
    await page.goto("http://ui.localhost/tray.html");
    await page.waitForFunction(() => Object.keys(listeners).length === 2);
    assert.equal(await page.evaluate(() => calls.filter(call => ["list_profiles", "open_vault", "refresh_usage", "switch_profile"].includes(call.command)).length), 0);
    await page.evaluate(() => { holdUsage = true; });
    await page.evaluate(() => listeners["tray-opened"]());
    await page.waitForFunction(() => document.querySelectorAll(".tray-account").length === 2);
    assert(await page.getByRole("button", { name: "Switch to Brennon", exact: true }).isDisabled());
    assert(await page.getByRole("button", { name: "Open Accounts", exact: true }).isDisabled());
    assert.equal(await page.evaluate(() => calls.filter(call => call.command === "show_main_window").length), 0);
    await page.evaluate(() => { listeners["tray-hidden"](); listeners["tray-opened"](); });
    assert.equal(await page.evaluate(() => calls.filter(call => call.command === "list_profiles").length), 1);
    await page.evaluate(() => releaseUsage());
    await page.waitForFunction(() => document.querySelectorAll(".tray-account").length === 2 && document.querySelector("#account-list").getAttribute("aria-busy") === "false");
    assert.equal(await page.locator("#locked-message").textContent(), "Open Accounts to unlock your saved accounts.");
    assert.equal(await page.evaluate(() => calls.filter(call => call.command === "switch_profile").length), 0);
    assert.equal(await page.locator(".tray-account.active .account-name").textContent(), "Erica");
    assert.equal(await page.locator(".tray-account.active").getAttribute("aria-current"), "true");
    assert.equal(await page.locator(".tray-account.active .account-led").getAttribute("aria-label"), "Active account, last observed");
    assert.equal(await page.locator(".tray-account.active .usage-percent").textContent(), "82%");
    assert.equal(await page.getByRole("button", { name: "Switch to Brennon", exact: true }).count(), 1);
    assert.equal(await page.evaluate(() => calls.filter(call => call.command === "resize_tray_window").some(call => call.args.rows === 2 && call.args.paged === false)), true);
    assert(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight && document.documentElement.scrollWidth <= innerWidth));
    await page.screenshot({ path: path.join(out, "tray.png") });
    await page.evaluate(() => { holdSwitch = true; });
    await page.getByRole("button", { name: "Switch to Brennon", exact: true }).click();
    await page.waitForFunction(() => calls.filter(call => call.command === "switch_profile").length === 1);
    assert.equal(await page.evaluate(() => calls.find(call => call.command === "switch_profile").args.profileId), "b");
    const listsBeforeSwitchReopen = await page.evaluate(() => calls.filter(call => call.command === "list_profiles").length);
    await page.evaluate(() => { listeners["tray-hidden"](); listeners["tray-opened"](); });
    assert.equal(await page.evaluate(() => calls.filter(call => call.command === "list_profiles").length), listsBeforeSwitchReopen);
    await page.evaluate(() => releaseSwitch());
    await page.waitForFunction(() => document.querySelectorAll(".tray-account").length === 2 && document.querySelector("#account-list").getAttribute("aria-busy") === "false");
    assert.equal(await page.locator("#locked-view").isHidden(), true);

    await page.setViewportSize({ width: 380, height: 594 });
    await page.evaluate(() => {
      const now = Math.floor(Date.now() / 1000);
      fixture = { ...fixture, last_known_active_profile: "p50", profiles: Array.from({ length: 50 }, (_, index) => ({ id: `p${index + 1}`, label: `Profile ${index + 1}`, domain: "ChatGPT", captured_at: now })) };
      calls = [];
      listeners["tray-hidden"]();
      listeners["tray-opened"]();
    });
    await page.waitForFunction(() => document.querySelectorAll(".tray-account").length === 5 && document.querySelector("#page-label").textContent === "1 / 10");
    await page.waitForFunction(() => document.querySelector("#account-list").getAttribute("aria-busy") === "false");
    assert.equal(await page.locator(".tray-account.active .account-name").textContent(), "Profile 50");
    await page.evaluate(() => { holdUsage = true; });
    await page.getByRole("button", { name: "Next accounts", exact: true }).click();
    await page.waitForFunction(() => document.querySelector("#page-label").textContent === "2 / 10");
    assert(await page.getByRole("button", { name: /^Switch to Profile/ }).first().isDisabled());
    assert(await page.getByRole("button", { name: "Next accounts", exact: true }).isDisabled());
    assert(await page.getByRole("button", { name: "Refresh accounts", exact: true }).isDisabled());
    assert.equal(await page.evaluate(() => calls.filter(call => call.command === "switch_profile").length), 0);
    await page.evaluate(() => releaseUsage());
    await page.waitForFunction(() => document.querySelector("#account-list").getAttribute("aria-busy") === "false");
    await page.getByRole("button", { name: "Previous accounts", exact: true }).click();
    await page.waitForFunction(() => document.querySelector("#page-label").textContent === "1 / 10" && document.querySelector("#account-list").getAttribute("aria-busy") === "false");
    const seen = [];
    for (let index = 0; index < 10; index += 1) {
      seen.push(...await page.locator(".tray-account .account-name").allTextContents());
      if (index < 9) await page.getByRole("button", { name: "Next accounts", exact: true }).click();
    }
    assert.equal(new Set(seen).size, 50);
    assert(seen.includes("Profile 50"));
    assert.equal(await page.evaluate(() => calls.filter(call => call.command === "switch_profile").length), 0);
    assert(await page.evaluate(() => document.documentElement.scrollHeight <= innerHeight && document.documentElement.scrollWidth <= innerWidth));

    await page.setViewportSize({ width: 380, height: 270 });
    await page.evaluate(() => {
      const now = Math.floor(Date.now() / 1000);
      fixture = { ...fixture, last_known_active_profile: "a", profiles: [
        { id: "a", label: "Erica", domain: "Personal", captured_at: now },
        { id: "b", label: "Brennon", domain: "Work", captured_at: now },
      ] };
      calls = [];
      listeners["tray-hidden"]();
      listeners["tray-opened"]();
    });
    await page.waitForFunction(() => document.querySelectorAll(".tray-account").length === 2);
    await page.getByRole("button", { name: "Switch to Brennon", exact: true }).click();
    await page.locator("#trust-warning").waitFor({ state: "visible" });
    assert.equal(await page.evaluate(() => calls.filter(call => call.command === "switch_profile").length), 0);
    await page.evaluate(() => listeners["tray-hidden"]());
    assert.equal(await page.locator("#trust-warning").evaluate(dialog => dialog.open), false);
    await page.evaluate(() => listeners["tray-opened"]());
    await page.waitForFunction(() => document.querySelectorAll(".tray-account").length === 2);
    await page.getByRole("button", { name: "Switch to Brennon", exact: true }).click();
    await page.locator("#trust-warning").waitFor({ state: "visible" });
    await page.locator("#confirm-switch").click();
    await page.waitForFunction(() => calls.filter(call => call.command === "switch_profile").length === 1);

    await page.evaluate(() => { fixture = { ...fixture, state: "locked", profiles: [] }; });
    await page.getByRole("button", { name: "Refresh accounts", exact: true }).click();
    await page.waitForFunction(() => document.querySelector("#locked-view") && !document.querySelector("#locked-view").hidden);
    assert.equal(await page.locator("input[type=password]").count(), 0);
    await page.evaluate(() => { failList = "vault_busy"; });
    await page.getByRole("button", { name: "Refresh accounts", exact: true }).click();
    await page.waitForFunction(() => document.querySelector("#locked-message").textContent.includes("busy"));
    assert.equal(await page.locator("#unlock-in-main").isVisible(), false);
    assert.deepEqual(errors, []);
    console.log("PASS compact paged tray flyout, deferred loading, explicit switching and trust warning. Synthetic IPC only.");
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exit(1); });
