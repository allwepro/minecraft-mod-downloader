// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="introduction.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="installation.html"><strong aria-hidden="true">2.</strong> Installation</a></li><li class="chapter-item expanded "><a href="usage.html"><strong aria-hidden="true">3.</strong> Getting Started</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="usage/launcher.html"><strong aria-hidden="true">3.1.</strong> Launcher (WIP)</a></li><li><ol class="section"><li class="chapter-item expanded "><div><strong aria-hidden="true">3.1.1.</strong> Technical Details</div></li><li><ol class="section"><li class="chapter-item expanded "><a href="usage/launcher/technical-details/tests.html"><strong aria-hidden="true">3.1.1.1.</strong> Testing</a></li></ol></li></ol></li><li class="chapter-item expanded "><a href="usage/resource-manager.html"><strong aria-hidden="true">3.2.</strong> Resource Manager</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="usage/resource-manager/get_started.html"><strong aria-hidden="true">3.2.1.</strong> Get Started</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/lists.html"><strong aria-hidden="true">3.2.2.</strong> Lists</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/groups_instances.html"><strong aria-hidden="true">3.2.3.</strong> Groups &amp; Instances</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/importing.html"><strong aria-hidden="true">3.2.4.</strong> Importing Resources</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/toolbar_search.html"><strong aria-hidden="true">3.2.5.</strong> Toolbar &amp; Search</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/list_actions_settings.html"><strong aria-hidden="true">3.2.6.</strong> List Actions &amp; Settings</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/managing_resources.html"><strong aria-hidden="true">3.2.7.</strong> Managing Resources</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/dependencies_archives.html"><strong aria-hidden="true">3.2.8.</strong> Dependencies &amp; Archives</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/productivity.html"><strong aria-hidden="true">3.2.9.</strong> Productivity &amp; Shortcuts</a></li><li class="chapter-item expanded "><a href="usage/resource-manager/resource_manager_settings.html"><strong aria-hidden="true">3.2.10.</strong> Resource Manager Settings</a></li><li class="chapter-item expanded "><div><strong aria-hidden="true">3.2.11.</strong> Technical Details</div></li><li><ol class="section"><li class="chapter-item expanded "><a href="usage/resource-manager/technical-details/tests.html"><strong aria-hidden="true">3.2.11.1.</strong> Testing</a></li></ol></li></ol></li></ol></li><li class="chapter-item expanded "><li class="spacer"></li><li class="chapter-item expanded "><a href="faq.html"><strong aria-hidden="true">4.</strong> FAQ</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
