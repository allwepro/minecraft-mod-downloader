# ⚡ Productivity & Shortcuts

The Flux Launcher's Resource Manager is designed to streamline your workflow with powerful productivity features, including multi-selection, context menus, and keyboard shortcuts.

---

## 🖱️ Multi-Selection

Managing many resources or lists can be tedious one-by-one. Flux Launcher provides familiar multi-selection methods to help you act on multiple items at once in both the main project list and the sidebar.

### In the Project List (Main Panel)

-   **Select Single Item:** Click on any project entry. This will deselect any previously selected items and highlight the clicked project.
-   **Select Multiple Items (Non-Contiguous):**
    -   Hold `Ctrl` (Windows/Linux) or `Command` (macOS) and click on individual projects. Each click will toggle the selection status of that project (add if not selected, remove if already selected).
-   **Select a Range of Items:**
    -   Click on the first project you want to select.
    -   Hold `Shift` and click on the last project in your desired range. All projects between the first and last clicked (inclusive) will be selected.
    -   If combined with `Ctrl`/`Command`, `Shift + Click` will add the range to your existing selection without clearing it first.
-   **Clear Selection:**
    -   Press the `Escape` key.
    -   Click on an empty area within the project list.

### In the Sidebar (Lists & Groups)

Multi-selection in the sidebar works identically to the project list, allowing you to select multiple `Lists` or `List Groups` for bulk actions (e.g., deleting several items at once).

-   **Select Single Item:** Click on any list or list group entry.
-   **Select Multiple Items (Non-Contiguous):** Hold `Ctrl` (Windows/Linux) or `Command` (macOS) and click on individual sidebar items.
-   **Select a Range of Items:** Hold `Shift` and click on the last item in your desired range.
-   **Clear Selection:** Click on an empty area within the sidebar.

---

## 📋 Context Menus (Right-Click)

Right-clicking on items in the Resource Manager reveals context-sensitive menus, offering quick access to relevant actions.

### Project Context Menu (Main Panel)

Right-click on one or more selected projects in the main panel to bring up options for those projects.

-   **Copy:** Copies the selected projects to the clipboard.
-   **Cut:** Cuts the selected projects, preparing them to be moved to another list.
-   **Paste:** Pastes projects from the clipboard into the current list.
    -   This option is only available if there are compatible projects in the clipboard and the resource types match (e.g., cannot paste a Mod into a Shader list).
    -   If you "Cut" projects, pasting them here will *move* them from their original list. If you "Copy" them, pasting will create duplicates.
-   **Download Selected:** Initiates the download of the latest compatible version for all selected projects that are currently missing.
-   **Update Selected:** Updates all selected projects to their latest compatible versions. Only available when automatic updates are disabled and updates are detected.
-   **Archive Selected:** Moves the selected projects to the "Archived Projects" section of the current list. Projects with active dependents cannot be archived.
-   **Unarchive Selected:** Moves selected archived projects back to the active project list.
-   **Delete Selected:** Permanently removes the selected projects from the current list and deletes their associated files from disk. Projects with active dependents cannot be deleted without first removing the dependents or making them auto-managed.

### List Context Menu (Sidebar)

Right-click on a single list in the sidebar to access options for that specific list. If you have multiple lists selected via multi-select, right-clicking on any of them will open the context menu for all selected lists.

-   **Move to Group:** A sub-menu appears, allowing you to move the list into an existing List Group or to remove it from any group ("No Group").
-   **Open Folder:** Opens the download directory for the selected list in your system's file explorer.
-   **Duplicate:** Creates an exact copy of the selected list, including all its projects and settings.
-   **Delete:** Removes the list and all its contained projects from Flux Launcher.

### List Group Context Menu (Sidebar)

Right-click on a single list group in the sidebar to access options for that group. If you have multiple list groups selected via multi-select, right-clicking on any of them will open the context menu for all selected groups.

-   **Create Subgroup:** Opens a modal to create a new, nested List Group directly within the selected parent group.
-   **Rename:** Opens a modal to change the name of the selected List Group.
-   **Duplicate:** Creates a duplicate of the selected List Group, including all its subgroups and associated lists.
-   **Delete:** Removes the List Group and all its contained subgroups and lists from Flux Launcher.

---

## ⌨️ Keyboard Shortcuts

Speed up your interactions with these handy keyboard shortcuts.

-   **`Delete`**: Deletes all currently selected items (Lists or List Groups) from the sidebar.
-   **`Enter`**: Confirms renaming actions when an input field is active (e.g., renaming a list or list group).