// A pointer for the sandbox. Headless sway has no input devices, so its
// seat offers no pointer and clients never see a click; this adds one
// through wlr-virtual-pointer for as long as it runs and plays the steps
// given as arguments:
//
//   move X Y     put the pointer at X,Y (output pixels)
//   by DX DY     move it by DX,DY in steps of at most 4 pixels
//   down / up    press / release the left button (rdown / rup: the right)
//   wheel N      turn the wheel N notches (negative: up)
//   wait MS      let MS milliseconds pass
//
// Built by tools/sandbox:
//   wayland-scanner client-header / private-code on the protocol XML here.
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <wayland-client.h>
#include "wlr-virtual-pointer-unstable-v1-client-protocol.h"

#define BTN_LEFT 0x110
#define BTN_RIGHT 0x111

static struct wl_seat *seat;
static struct zwlr_virtual_pointer_manager_v1 *manager;
static struct wl_display *display;
static struct zwlr_virtual_pointer_v1 *pointer;
static int width = 1400, height = 900;
static double x, y;

static void global(void *data, struct wl_registry *registry, uint32_t name, const char *interface, uint32_t version)
{
    (void)data;
    (void)version;
    if (strcmp(interface, wl_seat_interface.name) == 0 && !seat)
        seat = wl_registry_bind(registry, name, &wl_seat_interface, 1);
    else if (strcmp(interface, zwlr_virtual_pointer_manager_v1_interface.name) == 0)
        manager = wl_registry_bind(registry, name, &zwlr_virtual_pointer_manager_v1_interface, 1);
}

static void global_remove(void *data, struct wl_registry *registry, uint32_t name)
{
    (void)data;
    (void)registry;
    (void)name;
}

static const struct wl_registry_listener listener = { global, global_remove };

static uint32_t now(void)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (uint32_t)(t.tv_sec * 1000 + t.tv_nsec / 1000000);
}

// Waits while the compositor and the clients catch up.
static void pause_ms(long ms)
{
    wl_display_roundtrip(display);
    struct timespec t = { ms / 1000, (ms % 1000) * 1000000 };
    nanosleep(&t, NULL);
}

static void place(double nx, double ny)
{
    x = nx;
    y = ny;
    zwlr_virtual_pointer_v1_motion_absolute(pointer, now(), (uint32_t)x, (uint32_t)y, width, height);
    zwlr_virtual_pointer_v1_frame(pointer);
    pause_ms(15);
}

static void button(uint32_t which, uint32_t state)
{
    zwlr_virtual_pointer_v1_button(pointer, now(), which, state);
    zwlr_virtual_pointer_v1_frame(pointer);
    pause_ms(40);
}

int main(int argc, char **argv)
{
    const char *size = getenv("POINTER_EXTENT");
    if (size)
        sscanf(size, "%dx%d", &width, &height);
    display = wl_display_connect(NULL);
    if (!display) {
        fprintf(stderr, "pointer: no Wayland display\n");
        return 1;
    }
    struct wl_registry *registry = wl_display_get_registry(display);
    wl_registry_add_listener(registry, &listener, NULL);
    wl_display_roundtrip(display);
    if (!manager) {
        fprintf(stderr, "pointer: the compositor has no wlr-virtual-pointer\n");
        return 1;
    }
    pointer = zwlr_virtual_pointer_manager_v1_create_virtual_pointer(manager, seat);
    // The seat gains a pointer now; give the clients time to bind theirs.
    pause_ms(250);
    for (int i = 1; i < argc; i++) {
        const char *step = argv[i];
        if (strcmp(step, "move") == 0 && i + 2 < argc) {
            place(atof(argv[i + 1]), atof(argv[i + 2]));
            i += 2;
        } else if (strcmp(step, "by") == 0 && i + 2 < argc) {
            double dx = atof(argv[i + 1]), dy = atof(argv[i + 2]);
            double ax = dx < 0 ? -dx : dx, ay = dy < 0 ? -dy : dy;
            int n = (int)((ax > ay ? ax : ay) / 4) + 1;
            double sx = x, sy = y;
            for (int k = 1; k <= n; k++)
                place(sx + dx * k / n, sy + dy * k / n);
            i += 2;
        } else if (strcmp(step, "down") == 0) {
            button(BTN_LEFT, WL_POINTER_BUTTON_STATE_PRESSED);
        } else if (strcmp(step, "up") == 0) {
            button(BTN_LEFT, WL_POINTER_BUTTON_STATE_RELEASED);
        } else if (strcmp(step, "rdown") == 0) {
            button(BTN_RIGHT, WL_POINTER_BUTTON_STATE_PRESSED);
        } else if (strcmp(step, "rup") == 0) {
            button(BTN_RIGHT, WL_POINTER_BUTTON_STATE_RELEASED);
        } else if (strcmp(step, "wheel") == 0 && i + 1 < argc) {
            int n = atoi(argv[++i]);
            zwlr_virtual_pointer_v1_axis(pointer, now(), WL_POINTER_AXIS_VERTICAL_SCROLL, wl_fixed_from_int(15 * n));
            zwlr_virtual_pointer_v1_frame(pointer);
            pause_ms(40);
        } else if (strcmp(step, "wait") == 0 && i + 1 < argc) {
            pause_ms(atol(argv[++i]));
        } else {
            fprintf(stderr, "pointer: what is \"%s\"?\n", step);
            return 2;
        }
    }
    pause_ms(100);
    zwlr_virtual_pointer_v1_destroy(pointer);
    wl_display_roundtrip(display);
    wl_display_disconnect(display);
    return 0;
}
