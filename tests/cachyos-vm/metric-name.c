/* Наблюдатель теста, не часть MeshPilot. Читает доступное имя карточки из
 * AT-SPI только в приложении с ожидаемым PID. Нет hooks в испытуемом окне. */
#include <atspi/atspi.h>
#include <stdio.h>
#include <string.h>

static gboolean saw_app, saw_card;
static const char *heading, *source, *incomplete = "GUI_METRIC_EMPTY";

static gboolean find_card(AtspiAccessible *node, unsigned depth, unsigned *budget) {
    if (!node || depth > 32 || !*budget) return FALSE;
    --*budget;
    gchar *name = atspi_accessible_get_name(node, NULL);
    gboolean card = name && g_str_has_prefix(name, heading);
    if (card) {
        saw_card = TRUE;
        // Только коды из закрытого списка. Никогда не печатать произвольное дерево окна.
        if (strstr(name, "Канал недоступен")) incomplete = "GUI_METRIC_IPC";
        else if (strstr(name, "Некорректный ответ")) incomplete = "GUI_METRIC_INVALID";
        else if (strstr(name, "Нет достоверного замера")) incomplete = "GUI_METRIC_UNAVAILABLE";
        else if (strstr(name, "Чтение")) incomplete = "GUI_METRIC_PENDING";
        else if (strstr(name, " мс")) incomplete = "GUI_METRIC_PARTIAL";
    }
    gboolean found = card && strlen(name) < 256 &&
                     g_strstr_len(name, -1, source) && g_str_has_suffix(name, "UTC");
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
    if (argc != 3) return 2;
    if (strcmp(argv[2], "ram") == 0) { heading = "Занято RAM"; source = "/proc/meminfo"; }
    else if (strcmp(argv[2], "cpu") == 0) { heading = "Загрузка CPU"; source = "/proc/stat"; }
    else if (strcmp(argv[2], "root_fs") == 0) { heading = "Занято на /"; source = "statvfs(/)"; }
    else if (strcmp(argv[2], "net") == 0) { heading = "Трафик сети"; source = "/proc/net/dev"; }
    else return 2;
    guint pid = (guint)g_ascii_strtoull(argv[1], NULL, 10);
    if (!pid || atspi_init() != 0) { fputs("GUI_METRIC_BUS\n", stderr); return 1; }
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
                // Не зачитывать кэшированную подпись загрузки после завершения замера.
                atspi_accessible_clear_cache(app);
                unsigned budget = 512;
                found = find_card(app, 0, &budget);
            }
            g_clear_object(&app);
        }
        if (!found) g_usleep(350000);
    }
    g_object_unref(desktop);
    atspi_exit();
    if (!found) fprintf(stderr, "%s\n", !saw_app ? "GUI_METRIC_APP" : saw_card ? incomplete : "GUI_METRIC_TREE");
    return found ? 0 : 1;
}
