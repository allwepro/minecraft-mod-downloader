# Toolbar & Search

The toolbar at the top of the Resource Manager's main view provides quick access to essential actions for managing your current resource list, including refresh options, search functionality, and bulk download/update actions.

---

## ➕ Add Resource

The **"Add [Resource Type]"** button (e.g., "➕ Add Mod", "➕ Add Shader") is the primary way to introduce new resources to your list.

-   Clicking this button opens a **Search Modal**.
-   **Search Modal Usage:**
    -   Enter your search query in the provided text field.
    -   You can toggle **"Match version/loader"** to filter results more precisely to your list's configured Minecraft version and loader (e.g., Fabric for 1.21.1).
    -   Once you find the desired project, click the **"Add"** button next to it.
-   The selected project will then be added to your current list in the Resource Manager.
-   **Disabled in Offline Mode:** This feature requires an internet connection to search Modrinth and is therefore disabled when Flux Launcher is in offline mode.

---

## 🔄 Refresh Files

The **"Refresh"** button (🔄 icon) initiates a scan of the list's configured download directory.

-   **Standard Click:** Re-indexes the files on your disk to ensure Flux Launcher has the most up-to-date information on what's installed and its status. This is useful if you've manually added or removed files from the directory.
-   **Shift + Click:** Performs a deeper refresh, re-indexing files *and* forcing a recalculation of all project dependencies. This can resolve issues where dependency status appears incorrect after significant changes or if you suspect data corruption.

> 💡 **Spinner:** While a refresh or scan is active, the refresh button will transform into a spinning icon (⏳) to indicate that files are being processed.

---

## 🔍 Search Resources

The search bar, labeled "🔍 Search [Resource Type]...", allows you to filter the displayed resources within your current list.

-   **Keyword Search:** Type keywords to filter projects by their **name**. The search is case-insensitive.
-   **Include Dependencies:** To extend your search to include the names of *dependencies* that projects in your list rely on, prepend your search query with an ampersand (`&`).
    -   Example: `&fabric` might show mods that require Fabric API, even if "fabric" isn't in their name.
-   **Clear Search:** Click the **"❌"** button next to the search bar to quickly clear your query.

---

## ⬇ Download All

The **"Download All"** button streamlines the process of getting all missing resources for your list.

-   This button will appear as **"⬇ Download All"** when there are projects in your list that are not yet downloaded.
-   Clicking it will queue all currently missing, unarchived projects for download.
-   **"⏳ Downloading..."**: While downloads are in progress, the button will change to indicate the active download status.
-   **Disabled in Offline Mode:** This feature, like all Modrinth interaction, is disabled when Flux Launcher is in offline mode.

---

## 🔄 Update All

If your list has "Auto-Update Enabled" (configured in List Settings), this button will not appear. If auto-updates are disabled, the **"Update All"** button becomes visible when one or more projects in your list have a newer version available on Modrinth.

-   Clicking **"🔄 Update All"** will attempt to download the latest compatible versions for all projects that are currently outdated.
-   **Disabled in Offline Mode:** This feature is unavailable when Flux Launcher is in offline mode.

---

## 🗑 Delete X selected

This button appears when you have selected multiple projects in your list (using Ctrl/Cmd + click or Shift + click).

-   The text will indicate how many projects are selected (e.g., "🗑 Delete 3 selected").
-   Clicking this button will remove all selected projects from your list.