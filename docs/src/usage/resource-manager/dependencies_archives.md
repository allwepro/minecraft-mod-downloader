# 🧩 Dependencies & Archives

Flux provides robust tools to manage project dependencies, archive unused resources, and clean up unassociated files, ensuring your Minecraft installation remains organized and efficient.

## Managing Dependencies

Dependencies are other resources that a specific mod, shader, or pack requires to function correctly. Flux automatically detects and manages these relationships, simplifying the installation process.

### Displaying Dependencies
When a resource in your list has required dependencies, a badge like `+N Dependencies` will appear below its name and author. The "N" indicates the number of other projects that are necessary for this resource to work.

### Expanding and Viewing Dependencies
Clicking the `+N Dependencies` badge will expand the entry, revealing the list of required projects. These dependent projects are displayed with a slightly indented visual style and simplified interaction options, making it clear they are managed as part of their parent resource.

### Dependency Handling During Actions
*   **Downloading & Updating:** When you download or update a resource, Flux automatically attempts to download or update its required dependencies to compatible versions.
*   **Archiving:** A resource that is required by another project **cannot be archived**. If you try, a tooltip will inform you which projects depend on it. This prevents you from accidentally breaking your mod setup.
*   **Deleting:** If you attempt to delete a project that is required by others, the delete button's tooltip will change, indicating that deleting it will "demote" it to an auto-managed state. This means it will remain in your list (as an auto-managed project) because another active project still requires it, but it will no longer be considered a manually added item. To fully remove it, all dependent projects must first be removed or archived.

### Identifying Missing Dependencies
If a listed project (or one of its dependencies) is not present in your download directory despite being enabled and compatible, it will display a `📁 Missing` badge. This helps you quickly identify and resolve any missing files by prompting you to download them.

## Archiving Resources

Archiving allows you to temporarily disable and hide resources without permanently deleting them. This is useful for testing different mod combinations or reducing clutter in your active list.

### Archiving & Unarchiving Individual Resources
Each resource entry has "📁 Archive" and "📂 Unarchive" buttons.
*   Clicking **📁 Archive** moves the selected resource(s) to the "Archived Projects" section. This hides them from your main view and prevents them from being loaded into Minecraft.
*   Clicking **📂 Unarchive** restores the resource(s) to your active list. When unarchiving, Flux will attempt to scroll the restored item(s) back into view.

**Important:** You cannot archive a project that has active dependents. You must first remove or archive all projects that depend on it.

### Archiving & Unarchiving Multiple Resources (Context Menu)
To manage multiple resources at once:
1.  **Select** one or more resources (use `Ctrl`/`Cmd` + click for multiple, `Shift` + click for a range).
2.  **Right-click** on any selected resource to open the context menu.
3.  Choose **📁 Archive Selected** or **📂 Unarchive Selected** to apply the action to all selected items.

### Viewing Archived Resources
Archived resources are kept in a separate, collapsible section below your active projects.
*   Click the **▶ Archived Projects (N)** button to expand this section and view all currently archived resources. The "N" indicates the total count of archived projects.
*   Clicking **🔽 Archived Projects (N)** will collapse the section again.