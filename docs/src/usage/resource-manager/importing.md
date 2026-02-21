# 📥 Importing Resources

Flux Launcher provides several ways to import existing resource collections or Minecraft folders, allowing you to quickly set up new lists without starting from scratch. All import options are accessible via the **"Import"** button in the sidebar.

---

## 💾 Import from File

This method allows you to import resource lists from various file formats. The behavior after selecting a file depends on its extension.

### Supported File Types & Behavior

*   **Flux List Files (`.toml`, `.mmd`):**
    *   These are Flux Launcher's native or compatible formats for comprehensive list exports.
    *   Importing these files will directly create a new list based on the file's configuration and projects, preserving all details.

*   **Legacy Mod List Files (`.mods`, `.all-mods`, `.queue-mods`):**
    *   These are older, simpler text-based formats often used to list mods by their names or IDs.
    *   When importing these, a **"Legacy Import Modal"** will appear. This modal allows you to define the properties of the *new list* that will be created from these legacy entries.

*   **Unsupported File Types:**
    *   Attempting to import any other file type will result in an **"Unsupported file type for import" notification**.

### How to Import
1.  Click the **"Import" button** in the sidebar.
2.  Select **"From File"** from the options.
3.  A file dialog will open. Navigate to and select your `.toml`, `.mmd`, `.mods`, `.all-mods`, or `.queue-mods` file.

    *   **For Flux List Files (`.toml`, `.mmd`):** The list will be processed and added to your sidebar automatically.
    *   **For Legacy Mod List Files (`.mods`, `.all-mods`, `.queue-mods`):**
        *   A modal window will appear, pre-filling a suggested **List Name** (usually derived from the file name).
        *   You *must* configure the **Resource Type** (e.g., Mod), **Game Version**, **Game Loader**, and the **Download Directory** for this new list.
        *   Click **"Import"** within the modal to proceed.

### Import Process & Results (for Legacy Mod List Files)
*   After configuring a legacy import, a progress modal may appear, showing the status of matching projects from the file to Modrinth's database.
*   If any entries from the legacy file cannot be found or matched on Modrinth, they will be listed as **"Unresolved"** in the import results. You can then confirm the import (creating a list with only the resolvable items) or cancel the operation. Unresolved items will not be added to your new list.

---

## 🌐 Import Modrinth Collection

Importing a Modrinth collection allows you to easily set up a new resource list based on a collection curated on Modrinth.

### How to Import
1.  Click the **"Import" button** in the sidebar.
2.  Select **"Modrinth Collection"**.
3.  A modal window will prompt you to enter the **Modrinth Collection URL** or its unique **ID**.
    *   Example URL: `https://modrinth.com/collection/ZCxg7r1U`
    *   Example ID: `ZCxg7r1U`
4.  Click **"Import"**.

### Configuration & Finalization
*   Flux Launcher will first load the collection's data from Modrinth. During this time, a **"Loading collection..."** message will be displayed.
*   Once loaded, if the collection contains projects, you will enter a "finalizing" step:
    *   **List Name:** A default name based on the collection will be suggested.
    *   **Resource Type:** If the collection contains multiple resource types (e.g., both mods and resource packs), Flux Launcher will alert you. You can select one resource type for the current import, and then re-import the collection for other types if desired.
    *   **Game Version / Game Loader / Download Directory:** Configure these settings for your new list.
*   Click **"Import"** again to create the new list with all matched projects from the Modrinth collection.

---

## 📦 Import from Minecraft Folder

This feature allows you to scan an existing Minecraft folder (e.g., your `.minecraft` directory or a specific instance folder) for installed resources and import them into a new Flux Launcher list.

The import process guides you through a few steps:

### 1. Select Folder

1.  Click the **"Import"** button in the sidebar.
2.  Select **"From Folder"**.
3.  Choose the **Resource Type** you want to detect (e.g., `Mod`, `Shader`, `Resource Pack`). This helps Flux Launcher focus its scan.
4.  Specify the **Folder** you want to scan. You can type the path directly or use the **"Browse..."** button to select it via a file dialog.
5.  Click **"Import"** to begin the scan.

### 2. Scanning

Flux Launcher will now scan the specified folder. This step involves:
*   Identifying files matching the selected resource type.
*   Attempting to resolve these files against the Modrinth database to find corresponding projects.
*   Reporting progress on detected files and matches.
*   Any errors encountered during the scan will be displayed.

Once the scan is complete, the modal automatically transitions to the "Review" step.

### 3. Review Detected Files

In this step, you can review the files Flux Launcher detected and their potential matches on Modrinth.

*   **File List:** Each detected file is listed with its original filename, a cleaned-up name for easier identification, and the detected Minecraft version and loader.
*   **Match Status:**
    *   **Exact Match:** If Flux Launcher finds a unique, strong match for a file, it will be automatically selected and marked "Exact Match" (green).
    *   **Selected:** If you manually choose a match, it will be marked "Selected."
    *   **No Matches Found:** If no matches are found, you'll see a "No matches found" (red) message.
*   **Actions for Each File:**
    *   **Change / Select Match:** For files with multiple potential matches or to change an existing selection, click "Change" or the dropdown to select a different match from the list.
    *   **🔍 Search Manually...:** If you don't find the correct match in the dropdown or if no matches were found, you can manually search Modrinth for the project. This will open a new search modal, pre-filled with the detected file name. Once a selection is made, you'll return to the review screen.
    *   **⊗ Skip this file:** If you don't want to include a particular file in your new list, you can skip it. Skipped files are marked "Skipped" (gray). You can "Undo" skipping later.
*   **Show Unresolved:** A filter button allows you to quickly view only the files that still require a decision (not skipped or selected).

Once all files are either skipped or have a selected match, the "Continue" button will become active, allowing you to proceed.

### 4. New List Settings

In the final step, you configure the settings for your new list, similar to creating a fresh list.

1.  **List Name:** Enter a name for your new resource list. By default, this will be the name of the folder you imported from.
2.  **Resource Type:** This will be pre-selected based on your choice in the first step.
3.  **Minecraft Version:** Select the target Minecraft version for this list. Flux Launcher may suggest a version based on its scan.
4.  **Game Loader:** Select the desired game loader (e.g., Fabric, Forge, Quilt). Flux Launcher may suggest a loader.
5.  **Download Directory:** Specify where resources for this list should be downloaded.

After configuring these settings, click **"Import"**. Flux Launcher will create your new list, add all the selected projects, and then close the modal.