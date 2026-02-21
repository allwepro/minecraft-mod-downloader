# Managing Resources

This section details how to add, manage, update, and remove individual resources (mods, shaders, etc.) within your active list. Flux Launcher provides intuitive tools and visual indicators to keep your game content organized and up-to-date.

## Adding Resources

To add new resources to your list, use the "Add [Resource Type]" button prominently displayed at the top of the main content area.

*   **"➕ Add [Resource Type]" Button:** Click this button (e.g., "➕ Add Mod") to open the **Search Modal**. This modal allows you to browse and select resources directly from Modrinth, filtering by compatibility with your list's configured game version and loader.
*   **Offline Mode:** This button will be disabled and grayed out when Flux Launcher is in offline mode, as it requires an internet connection to search Modrinth.

*   **Keyboard Shortcut:** After typing your query in the Search Modal, you can simply press **`Enter`** to trigger the search without clicking the button.
*   **Smart Filtering:** Keep **"Match version/loader"** checked to ensure you only see projects compatible with your current list's Minecraft version and loader (Fabric/NeoForge/etc.).

## The Reload Button

The "Reload" button (🔄) is a vital tool for ensuring Flux Launcher's view of your files is current with your disk.

*   **Standard Click (🔄):** A regular click on this button triggers a re-indexing of files in your list's download directory. Use this if you manually add or remove files, or if you suspect the displayed status of your resources is out of sync.
*   **Shift+Click (🔄 + Shift):** Holding `Shift` while clicking "Reload" forces a full recalculation of all project dependencies. This can resolve issues where dependency relationships might appear incorrect or if a mod's requirements have recently changed.
*   **Loading Indicator:** While files are being scanned or re-indexed, the button will change to a spinner icon (⏳) to indicate activity.

## Auto-Update Mode

Flux Launcher offers an "Auto-Update" mode, configurable in the List Settings. This mode affects how updates are handled across your list.

*   **If Auto-Update is enabled:** Flux Launcher will automatically select and download the latest compatible versions for your resources whenever available, ensuring your list is always current without manual intervention.
*   **If Auto-Update is disabled:** A "🔄 Update All" button will appear if any resources in your list have a newer version available. You will also see individual "🔄 Update" buttons next to each outdated resource, allowing for granular control over updates.

## Individual Resource Entries

Each resource in your list is displayed as an entry with its icon, name, version, author, and various status indicators and action buttons.

### Resource Details

*   **Icon:** A small icon representing the resource (e.g., a gear for Mod Menu, a cube for Sodium).
*   **Name:** The name of the resource, often displayed as a hyperlink. Clicking the name will open the resource's page on Modrinth in your web browser.
*   **Version & Author:** Displays the currently selected version (e.g., `v17.0.0-beta.2`) and the author of the project (e.g., `by Prospector`).

### Status Indicators

Next to each resource, you may see one or more of these indicators:

*   **`✅` (Downloaded):** Indicates the resource's currently selected version is present and matches its expected hash on disk.
*   **`⏳ Downloading...` (with Progress Bar):** Shown when the resource is actively being downloaded or is in the download queue, along with a percentage progress.
*   **`❌` (Failed):** Appears if a download or metadata retrieval for the resource failed. Clicking this button allows you to clear cached metadata and version data, prompting a re-attempt to load.
*   **`📁 Missing`:** Indicates that the resource is part of your list and compatible, but its file is not currently present in the download directory. You will see a "Download" button to obtain it.
*   **`🔄 Update Available`:** If auto-update is disabled and a newer version of the resource is found online, this badge will appear.
*   **`❌ Incompatible`:** This resource is not compatible with the current game version and loader settings of your list.
    *   **"🔒 Overrule" Button:** You can choose to force-enable the resource despite incompatibility warnings. Use this with caution.
*   **`⚠ Incompatible Overruled`:** Indicates that you have previously overruled an incompatibility warning for this resource.
    *   **"🔓 Revoke" Button:** Click to revert the override, which will re-enable compatibility checks.

### Dependency Management

Flux Launcher helps you manage resource dependencies:

*   **`+N Dependencies` Badge:** If a resource requires other mods to function, this badge will show the count of its required dependencies. Clicking this badge will expand the entry to display the dependent resources, indented below the main resource.
*   **Dependent Resources:** When expanded, dependencies are shown with smaller icons and indented, indicating their subordinate relationship. They generally inherit the status of their parent.
*   **Dependency Impact on Actions:** Certain actions (like archiving or deleting) may be restricted if a resource is required by another. A tooltip will explain why the action is disabled.

### Action Buttons (Per Resource)

These buttons appear on the right side of each resource entry, allowing you to manage individual items.

*   **Download / Update Button:**
    *   If the resource is missing, a "Download" button (blue) will appear.
    *   If a newer version is available and auto-update is off, an "🔄 Update" button (light blue) will appear.
    *   These buttons are disabled if the resource is incompatible (unless overruled) or in offline mode.
*   **Archive / Unarchive Button:**
    *   **`📁 Archive` (light yellow):** Moves the resource to an "Archived Projects" section at the bottom of the list. Archived resources are not downloaded or loaded by the game but remain in your list for easy restoration.
    *   **`📂 Unarchive` (light yellow):** Appears for archived resources, moving them back into the active list.
    *   **Restrictions:** This action is disabled if the resource has other projects that depend on it. A tooltip will inform you of the blocking dependencies.
*   **Delete Button (🗑):**
    *   **Light Red `🗑`:** For resources without dependents, clicking this button permanently removes the resource from your list and deletes its file from disk.
    *   **Orange `🗑`:** If the resource has other projects that depend on it, clicking this button will "demote" it to an auto-managed state (meaning it's kept in the list because others need it, but it's no longer considered a manually added project). Its file will remain on disk.

## Unknown Files

Below your active and archived projects, Flux Launcher may display an "Unknown Projects" section.

*   **Purpose:** This section lists files found in your list's download directory that are not linked to any known project in your list. These could be leftover files, manually placed mods, or files Flux couldn't identify.
*   **Expandable Section:** Click the expand button (▶ / 🔽) next to "Unknown Projects ([Count])" to show or hide these files.
*   **"🗑 Delete All" Button:** Available within the expanded "Unknown Projects" section, this button allows you to delete all unknown files with a single click.
*   **Individual Unknown Entries:** Each unknown file is shown with a generic icon (❓), its filename, and a "No metadata available" label. An individual `🗑` button is provided to delete that specific file.