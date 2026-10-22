# User guide

## First use

Open Codex Accounts and choose **Create vault**. On later launches choose **Open vault**.
Closing the vault hides saved accounts and releases its ownership; it does not delete them.
Only one Accounts window can own the vault at a time.

Use the **+** button to open the account form. Enter a label, such as Personal, Work,
or any name that helps you recognize the account. Choose **Save current account**
to capture the existing Codex sign-in, or **Sign in to another account** to open the
official browser login. Finish authentication yourself in that browser.

Adding another account uses a temporary isolated Codex home. The currently installed
account remains in place until you explicitly switch. Do not sign out of Desktop
just to add another account.

## Switching and account menus

Save work and fully quit Codex Desktop, any ChatGPT host running Codex, CLI sessions
and editor integrations before changing sign-in. Accounts checks for active clients
and never force-closes them. Keep the Accounts window open.

Choose **Switch** on the account you want. After success, Codex opens with the installed
sign-in. The small illuminated indicator shows Accounts' last known active profile.
Changes made outside Accounts may require a refresh to be reflected here.

The account menu lets you rename or remove the saved profile, or **Sign in again**.
Removal deletes the saved copy, not the service account or shared conversation history.
For reauthentication, choose the same account and workspace in the browser. An inactive
profile is updated in the vault. Repairing the installed account also installs its new
sign-in and reopens Desktop.

## Usage and resets

Usage is read once a minute while Accounts is visible and unlocked, using each profile's
existing access token. The header refresh requests an update sooner. Codex can remain
open. Monitoring does not refresh tokens, switch accounts or consume banked resets.

Bars show remaining allowance for the windows the service actually returns. Reset
countdowns use those observations; tooltips expose exact times. When a reset time passes,
the app clears the expired window and checks again instead of assuming a full balance.
A positive available-reset count is shown beside usage when reported by the service.
Missing values are unknown, not zero.

An expired token clears that profile's usage. Use **Sign in again** for an inactive
account that needs renewal. For the current account, monitoring can pick up credentials
that Codex itself has renewed. Authentication, offline and service failures do not create
invented balances. Service response formats can change independently of this app.

## Interrupted operations

Cancellation waits for the owned helper to exit and its temporary files to be cleaned.
If an operation is interrupted, reopen Accounts and follow the recovery action shown.
Keep the vault and the executable that performed the operation intact until it finishes.
Do not manually delete a pending journal or temporary sign-in directory to dismiss an error.

A current-account reauthentication can leave a **Finish** action. It keeps the new sign-in
protected and finishes installing it; it does not restore superseded credentials.
Completed recovery records are retired on a later verified vault opening.

## Troubleshooting

| Symptom | Next action |
| --- | --- |
| Another instance owns the vault | Close the older Accounts window, then open it again. |
| Account change is blocked by running clients | Quit Codex clients normally, then retry. |
| Installation or storage unsupported | Refresh Settings > Diagnostics and check the supported setup in the README. |
| Usage disappeared | Refresh, check connectivity, or sign in again if credentials expired. |
| Cleanup or recovery is pending | Preserve the vault and use the displayed recovery control. |

Settings > Diagnostics can refresh installation information and export a sanitized report.
Reports exclude account labels, IDs, credentials, local paths and authentication URLs.
Review any additional screenshots or text before sharing. See [Security](../SECURITY.md).
