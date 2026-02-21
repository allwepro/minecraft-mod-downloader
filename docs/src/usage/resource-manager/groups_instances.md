# 👥 Groups & Instances

Flux provides powerful organizational features through **Groups** and **Instances** in the sidebar. These allow you to categorize your resource lists and manage dedicated Minecraft game setups more effectively.

---

## What are Groups?

A group is a customizable folder in your sidebar that can contain multiple individual resource lists. Groups help keep your workspace organized, especially when you have many modpacks or different game configurations.

## What are Instances?

An instance is a special type of group designed to manage a complete Minecraft game installation. When you convert a group into an instance, you can:
*   Define a specific **instance directory** (`.minecraft` folder location).
*   Pin a particular **Minecraft game version** for all lists within that instance.
*   All lists assigned to an instance will automatically use the instance's directory and game version as their default.

---

## 🏗️ Creating & Organizing Groups

1.  **Creating a New Group:**
    *   In the sidebar, click the "➕ **Create**" button.
    *   From the popup, select "New List Group" (this option will be available when a group creation feature is implemented).
    *   A new, empty group will appear in your sidebar.

2.  **Adding Lists to a Group:**
    *   You can drag and drop existing lists directly into a group in the sidebar.
    *   Lists can also be moved between groups or out of groups to the root level.

3.  **Viewing & Navigating Groups:**
    *   Groups appear with a "➕" (collapsed) or "➖" (expanded) icon.
    *   Click the arrow or double-click the group's name to toggle its expanded state.
    *   A badge next to the group name shows the total number of lists it contains.
    *   If a group is an instance, it will display a "🎮" icon next to its name.

## ⚙️ Managing Group Settings

When you click on a group in the sidebar, its settings will be displayed in the main content area.

### Group Header Actions

At the top of the main panel, you'll see the group's name and a set of action buttons:

*   **✏ Rename:** Click this button to rename the current group. An input field will appear, allowing you to type a new name. Press `Enter` or click the "✔" button to confirm, or "❌" to cancel.
*   **👥 Duplicate:** Creates an exact copy of the group, including all its assigned lists and their resources.
*   **🗑 Delete:** Permanently deletes the group and unassigns any lists within it (the lists themselves are not deleted, only their assignment to this group).
*   **⬇ Download All / ⬇ Download Instance:**
    *   If it's a regular group, this button is labeled "⬇ Download All."
    *   If it's an instance, it's labeled "⬇ Download Instance."
    *   Clicking this button will initiate the download of all missing resources across *all* lists assigned to this group (and recursively for any nested groups).
    *   While downloads are active, the button changes to "⏳ Downloading...".
*   **⚙ Group Settings:** (Not explicitly shown in code but implied by `ListGroupSettingsModal::new`) This button will open a dedicated modal for group-specific configurations, such as toggling "Instance Mode" (described below).

### Instance Configuration

If the selected group is an **Instance**, a dedicated "Instance Configuration" panel will be visible:

*   **Instance Directory:**
    *   This field specifies the `.minecraft` directory where all lists within this instance will download their resources.
    *   Click the "📁 **Browse**" button to open a file picker and select a directory on your system.
    *   **Note:** If not set, it may try to infer a default based on your system.
*   **Game Version:**
    *   A dropdown menu where you can select a specific Minecraft Java Edition version for this instance.
    *   All lists within this instance will default to this game version for compatibility checks and downloads.
*   **💾 Save:** Click this button to apply changes to the instance's directory and game version.
*   **Download Path Information:** Below the save button, Flux provides a helpful overview of where different resource types will be downloaded within the specified instance directory (e.g., `Mods ➡ <instance_directory>/mods`).

### Converting to/from an Instance

If a selected group is **not** an instance, you'll see a button:

*   **🎮 Convert to Instance:** Clicking this button transforms the current group into an instance, activating the "Instance Configuration" panel for it. All associated lists will then inherit its download directory and game version.

If it is an instance, the "⚙ Group Settings" modal would likely contain an option to revert it to a regular group.

---

## 🖱️ Productivity & Context Menus

### Drag & Drop (DnD)

You can easily reorganize your sidebar using drag and drop:
*   **Move Lists:** Click and drag a list to move it into or out of a group, or reorder it within the same level.
*   **Move Groups:** Groups can also be dragged to reorder them or nest them within other groups (creating subgroups).
*   **Visual Feedback:** While dragging, a blue line indicates the drop target and position (before/after an item or inside a group).
*   **Auto-Expand:** Hovering over a collapsed group while dragging will automatically expand it after a short delay, allowing you to drop items inside.

### Right-Click Context Menus

Right-clicking on a list group in the sidebar will open a context menu with quick actions, similar to those in the header, for that specific group (e.g., Duplicate, Delete).