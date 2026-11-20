/* Наблюдатель теста, не часть MeshPilot. Читает доступное имя карточки из
 * AT-SPI только в приложении с ожидаемым PID. Нет hooks в испытуемом окне. */
#include <atspi/atspi.h>
#include <stdio.h>
#include <string.h>

static gboolean saw_app, saw_card;

static gboolean find_card(AtspiAccessible *node, unsigned depth, unsigned *budget) {
    if (!node || depth > 32 || !*budget) return FALSE;
    --*budget;
    gchar *name = atspi_accessible_get_name(node, NULL);
    gboolean card = name && g_str_has_prefix(name, "Занято RAM");
    if (card) saw_card = TRUE;
    gboolean found = card && strlen(name) < 256 &&
                     g_strstr_len(name, -1, "/proc/meminfo") && g_str_has_suffix(name, "UTC");
    if (found) puts(name);
    g_free(name);
    if (found) return TRUE;
    int count = atspi_accessible_get_child_count(node, NULL);
    for (int i = 0; i < count && i < 512 && *budget; i++) {
        AtspiAccessible *child = atspi_accessible_get_child_at_index(node, i, NULL);
        found = find_card(child, depth + 1, budget);
        g_clear_object(&child);
        if (found) return TRUE;
    }
    return FALSE;
}

int main(int argc, char **argv) {
    if (argc != 2) return 2;
    guint pid = (guint)g_ascii_strtoull(argv[1], NULL, 10);
    if (!pid || atspi_init() != 0) { fputs("GUI_RAM_BUS\n", stderr); return 1; }
    atspi_set_timeout(800, 3000);
    AtspiAccessible *desktop = atspi_get_desktop(0);
    if (!desktop) return 1;
    gboolean found = FALSE;
    for (int attempt = 0; !found && attempt < 45; attempt++) {
        while (g_main_context_iteration(NULL, FALSE)) {}
        int count = atspi_accessible_get_child_count(desktop, NULL);
        for (int i = 0; !found && i < count && i < 128; i++) {
            AtspiAccessible *app = atspi_accessible_get_child_at_index(desktop, i, NULL);
            if (app && atspi_accessible_get_process_id(app, NULL) == pid) {
                saw_app = TRUE;
                unsigned budget = 512;
                found = find_card(app, 0, &budget);
            }
            g_clear_object(&app);
        }
        if (!found) g_usleep(350000);
    }
    g_object_unref(desktop);
    atspi_exit();
    if (!found) fprintf(stderr, "%s\n", !saw_app ? "GUI_RAM_APP" : saw_card ? "GUI_RAM_EMPTY" : "GUI_RAM_TREE");
    return found ? 0 : 1;
}
