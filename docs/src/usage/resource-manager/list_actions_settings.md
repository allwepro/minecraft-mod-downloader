# List Actions & Settings

When you have a list open in the Resource Manager's main view, a dedicated set of tools appears at the top right of the panel. These allow you to manage the list itself, including its configuration, duplication, and export.

---

## ⚙ List Settings

The **"List Settings"** button opens a modal window specific to the currently active list. Here, you can define critical parameters that affect how resources are discovered, downloaded, and managed for this particular list.

**Configurable Options:**

-   **Minecraft Game Version:** The specific Minecraft version this list is designed for (e.g., `1.21.1`).
-   **Game Loader:** The mod loader required for this list (e.g., `Fabric`, `Forge`, `Quilt`).
-   **Resource Type:** Defines what kind of resources this list primarily manages (e.g., `Mod`, `Shader`, `Resource Pack`).
-   **Download Directory:** The local folder where resources for this list will be downloaded. This can be configured as a custom path or set to inherit from an instance group if the list is part of one.
-   **Auto-Update Enabled:** Toggle whether installed resources in this list should automatically check for and apply updates.

After making changes, click **"Save"**. The Resource Manager will then re-evaluate the list's compatibility and download status based on the new settings. You will also see the list's unique `ID` at the bottom of this modal.

---

## 📤 Export

The **"Export"** button allows you to save your current list of resources to a file, which can then be shared with others or used for backup.

-   Upon clicking, a file save dialog will appear, prompting you to choose a location and filename for your export.
-   **Export Formats:**
    -   **Flux Launcher (`.mmd`):** This is the native export format for Flux Launcher, containing detailed information about your list and its resources.
    -   **Legacy Mod List (for `Mods` only):** If you are exporting a `Mod` list and choose a non-Flux format, Flux can export a simpler list that might be compatible with other mod managers.

---

## 📂 Open Folder

Clicking **"Open Folder"** will instantly open the configured download directory for the current list in your operating system's file explorer. This provides quick access to your installed resources for manual inspection or management.

---

## 👥 Duplicate

The **"Duplicate"** button creates an exact copy of the active list. The new list will appear in your sidebar with a similar name (e.g., "New List (Copy)"), containing all the same projects and settings as the original. This is useful for creating variations of a modpack or for testing changes without affecting your primary list.

---

## ✏ Rename

To change the name of your open list, click the **"Rename"** button.

-   An inline text input field will appear, pre-filled with the current list name.
-   Type in your desired new name.
-   Press `Enter` or click the **"✔"** button to confirm the rename.
-   Click the **"❌"** button to cancel the renaming process.

---

## 🗑 Delete

The **"Delete"** button, prominently colored in red, allows you to permanently remove the current list.

-   Deleting a list removes it from Flux Launcher and unassigns any associated resource files, but it generally **does not delete the physical files from your disk**. This ensures you don't lose your mods if you simply want to remove the list from the launcher.