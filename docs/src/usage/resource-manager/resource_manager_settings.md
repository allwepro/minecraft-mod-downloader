# ⚙️ Resource Manager Settings

The **Resource Manager Settings** provide options to configure the overall behavior of the Resource Manager module, independent of any specific list or game instance. These settings help streamline your workflow, manage application data caching, and handle backups.

## Accessing Settings

To open the Resource Manager Settings modal:
1.  Locate the `Settings` button in the top-right corner of the Flux window. This is the **global application settings** button.
2.  Within the global settings, there should be a dedicated section or tab for "Resource Manager Settings." (The provided code snippet is for the modal itself, not its entry point in the main UI, but this is the logical user flow).

## General Settings

These settings are designed to customize default names for new lists and groups.

### Default List Name
-   **Purpose:** Sets the default name that will be automatically assigned to any new list you create. This can save time if you frequently create lists with a common prefix or theme.
-   **Usage:** Type your desired default name into the text field.

### Default List Group Name
-   **Purpose:** Sets the default name for any new list groups you create in the sidebar. Useful for organizing your groups consistently.
-   **Usage:** Enter your preferred default group name into the text field.

### Saving Changes
After making any adjustments to the General Settings, click the **💾 Save** button to apply your changes. These settings will persist across application restarts.

## Backup & Restore

This section allows you to export all your Resource Manager settings and lists into a single `.flux-rm` backup file, or import them from such a file. This is useful for migrating your data, sharing configurations, or creating restore points.

### Export Backup
-   **Purpose:** Creates a `.flux-rm` file containing all your current Resource Manager settings and lists.
-   **Usage:**
    1.  Click the **📤 Export Backup** button.
    2.  A file dialog will appear, pre-filling a suggested file name (`flux-resource-manager-backup.flux-rm`).
    3.  Choose your desired location and file name, then click `Save`.
    4.  A progress indicator will show while the backup is being created.

### Import Backup
-   **Purpose:** Loads settings and lists from a `.flux-rm` backup file, overwriting your current configuration.
-   **Usage:**
    1.  Click the **📥 Import Backup** button.
    2.  A file dialog will appear. Select the `.flux-rm` file you wish to import, then click `Open`.
    3.  A **Warning Modal** will appear, detailing the implications of importing a backup.
        -   **Important:** Importing a backup will **OVERWRITE** all existing Resource Manager settings and lists. Your current data will be replaced.
        -   The application will automatically reload all data after the import.
    4.  Review the warning carefully. If you wish to proceed, click **✔ Yes, Import**. If you change your mind, click **❌ Cancel**.
    5.  A progress indicator will show while the backup is being imported.

### Backup Progress
While an export or import operation is in progress, a spinner and a message indicating the current status (e.g., "Exporting items (1/5)") will be displayed. The export/import buttons will be disabled during this time.

## Advanced Options

The Advanced Options section offers more granular control, primarily focused on **Cache Management**. This section is collapsed by default to prevent accidental changes for most users.

To view and interact with advanced options:
-   Click the **▶ Advanced** button (or **🔽 Advanced** if already open) to expand this section.

### Cache Management

The Resource Manager extensively uses various caches to store downloaded search results, project metadata, icons, versions, and more. While this speeds up browsing and operations, sometimes a cache can become corrupted or outdated, leading to loading issues. Clearing specific caches can help resolve these problems, but be aware that the application will need to re-fetch that data, which may temporarily slow down operations.

#### Cache Types

You can selectively clear the following types of cache:

-   **Game Loader Cache:** Stores information about available game loaders (e.g., Fabric, Forge, Quilt).
-   **Game Version Cache:** Stores data on supported Minecraft game versions.
-   **Slug Cache:** Caches project "slugs" (short, unique identifiers for projects) used for API lookups.
-   **Metadata Cache:** Stores detailed information about Modrinth projects (descriptions, authors, tags, etc.).
-   **Versions Cache:** Caches the available file versions for specific projects, filtered by game version and loader.
-   **Icons Cache:** Stores project icons, both on disk and in GPU memory.
-   **Artifact Cache:** Contains the downloaded `.jar`, `.zip`, or other resource files themselves.
-   **File Index Cache:** An internal index of files found on your disk, used for fast lookup of installed resources.

#### Important Notes:
-   **Restart Required:** If you clear the **Game Loader Cache** or **Game Version Cache**, you will need to **restart Flux** for the changes to fully take effect. The application will indicate this if you select these options.
-   **Re-fetching Data:** Clearing any cache will require the application to re-download or re-index that specific data type when needed, which can cause a brief delay.

#### Clearing Cache
1.  **Select Caches:** Check the boxes next to the cache types you wish to clear.
2.  **Confirm:** Click the **🗑 Clear Cache** button.

Once clicked, the selected caches will be cleared from disk and/or memory.