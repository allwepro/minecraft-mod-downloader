# 📂 Lists: Creating and Managing Your Collections

Lists are the fundamental building blocks within Flux Launcher's Resource Manager. Each list represents a self-contained collection of Minecraft resources (mods, shaders, resource packs, etc.) tailored for a specific Minecraft version and loader combination. Think of them as dedicated profiles for your different gameplay experiences.

---

### Understanding the Lists Sidebar

The left sidebar of the Resource Manager is where all your lists are displayed and managed.

![Flux Launcher Sidebar](../../images/resource-manager/first_list.png)

Each list entry provides a quick overview of its configuration:
-   **List Name:** The user-defined name for your collection (e.g., "New List").
-   **Resource Type Badge (e.g., `Mod`):** Indicates the primary type of content this list is designed for. While often `Mod`, Flux supports others like `Shader`, `Resource Pack`, etc., though the current UI primarily shows `Mod` for general resource lists.
-   **Minecraft Version Badge (e.g., `1.21.11`):** The target Minecraft version for this list.
-   **Loader Type Badge (e.g., `Fabric`):** The specific Minecraft loader (Fabric, Forge, Quilt, NeoForge, Vanilla) configured for this list.
-   **Resource Count Badge (e.g., `0` or `2`):** Shows the current number of resources installed in this list.

When a list is selected, it will be highlighted in the sidebar, and its contents will be displayed in the main view.

---

### Creating a New List

To start a fresh collection of resources:

1.  Click the green **`+ Create`** button in the top-left of the sidebar.
2.  A dialog will appear (not shown in screenshots, but implied) prompting you to:
    *   **Name your new list.** Choose a descriptive name.
    *   **Select the target Minecraft Version.** This ensures compatible resources are suggested.
    *   **Choose the desired Loader Type.**
    *   **If necessary, pick your preferred download destination.**
3.  After confirmation, your new list will appear in the sidebar and be automatically selected.

Newly created lists will initially show **"No items in this list"** in the main view, ready for you to add resources.

---

### Searching Your Lists

As your collection grows, you might have many lists. The **`Search Lists...`** input field at the top of the sidebar helps you quickly find a specific list:

-   Type keywords into the search bar. The sidebar will dynamically filter, showing only lists whose names match your input.
-   To clear the search and see all lists again, simply delete the text from the search bar.

---

### Deleting a List

To remove a list you no longer need:

1.  Select the list you wish to delete from the sidebar.
2.  In the top-right action bar of the main view, click the **`Delete`** button (red trash can icon).
3.  A confirmation dialog will appear. Confirm to permanently remove the list and all its associated resources and configurations.

**Warning:** Deleting a list is irreversible and will remove all downloaded mods and configuration specific to that list. Ensure you have backed up any important data if necessary.

---

### Next Steps

Now that you know how to create and manage lists, you can:
-   **[Organize them further with Groups & Instances](groups_instances.md)**
-   **[Import existing collections](importing.md)**
-   **[Add resources to your lists](managing_resources.md)**