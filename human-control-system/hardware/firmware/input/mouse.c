#include "mouse.h"
#include "hid_reports.h"
#include "config.h"
#include "util.h"

static int16_t mouse_x = 0;
static int16_t mouse_y = 0;
static int8_t mouse_wheel = 0;
static int8_t mouse_pan = 0;
static uint8_t mouse_buttons = 0;
static bool mouse_absolute = false;

void mouse_init(void) {
    mouse_x = 0;
    mouse_y = 0;
    mouse_wheel = 0;
    mouse_pan = 0;
    mouse_buttons = 0;
    mouse_absolute = false;
}

void mouse_move(int16_t x, int16_t y, bool absolute) {
    if (absolute) {
        mouse_x = x;
        mouse_y = y;
        mouse_absolute = true;
    } else {
        mouse_x += x;
        mouse_y += y;
    }
    mouse_send_report();
}

void mouse_press(uint8_t button) {
    if (button < 5) {
        mouse_buttons |= (1 << button);
        mouse_send_report();
    }
}

void mouse_release(uint8_t button) {
    if (button < 5) {
        mouse_buttons &= ~(1 << button);
        mouse_send_report();
    }
}

void mouse_click(uint8_t button) {
    mouse_press(button);
    delay_ms(10);
    mouse_release(button);
}

void mouse_wheel(int8_t wheel, int8_t pan) {
    mouse_wheel = wheel;
    mouse_pan = pan;
    mouse_send_report();
    mouse_wheel = 0;
    mouse_pan = 0;
}

void mouse_send_report(void) {
    extern bool hid_send_mouse_report(const hid_mouse_report_t* report);

    hid_mouse_report_t report = {
        .report_id = HID_MOUSE_REPORT_ID,
        .buttons = mouse_buttons,
        .x = (int8_t)(mouse_absolute ? 0 : (mouse_x > 127 ? 127 : (mouse_x < -127 ? -127 : mouse_x))),
        .y = (int8_t)(mouse_absolute ? 0 : (mouse_y > 127 ? 127 : (mouse_y < -127 ? -127 : mouse_y))),
        .wheel = mouse_wheel,
        .pan = mouse_pan
    };

    if (mouse_absolute) {
        report.x = (int8_t)(mouse_x > 127 ? 127 : (mouse_x < -127 ? -127 : mouse_x));
        report.y = (int8_t)(mouse_y > 127 ? 127 : (mouse_y < -127 ? -127 : mouse_y));
    }

    hid_send_mouse_report(&report);

    if (!mouse_absolute) {
        mouse_x = 0;
        mouse_y = 0;
    }
    mouse_wheel = 0;
    mouse_pan = 0;
}

void mouse_get_position(int16_t* x, int16_t* y) {
    if (x) *x = mouse_x;
    if (y) *y = mouse_y;
}

uint8_t mouse_get_buttons(void) {
    return mouse_buttons;
}

bool mouse_is_button_pressed(uint8_t button) {
    if (button < 5) {
        return (mouse_buttons & (1 << button)) != 0;
    }
    return false;
}

void mouse_set_absolute_mode(bool absolute) {
    mouse_absolute = absolute;
}

bool mouse_is_absolute_mode(void) {
    return mouse_absolute;
}