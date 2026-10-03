#include "keyboard.h"
#include "hid_reports.h"
#include "config.h"
#include "util.h"

static uint8_t current_modifiers = 0;
static uint8_t pressed_keys[MAX_KEYS_PER_REPORT] = {0};
static uint8_t num_pressed_keys = 0;

#if NKRO_ENABLED
static bool nkro_keys[256] = {false};
#endif

void keyboard_init(void) {
    current_modifiers = 0;
    num_pressed_keys = 0;
    memset(pressed_keys, 0, sizeof(pressed_keys));
#if NKRO_ENABLED
    memset(nkro_keys, 0, sizeof(nkro_keys));
#endif
}

void keyboard_press(uint16_t usage_page, uint16_t usage, uint8_t modifiers) {
    if (usage_page != 0x07) {
        return;
    }

    if (modifiers) {
        current_modifiers |= modifiers;
    }

    if (usage == 0) {
        return;
    }

#if NKRO_ENABLED
    if (usage < 256) {
        nkro_keys[usage] = true;
    }
#endif

    if (num_pressed_keys < MAX_KEYS_PER_REPORT) {
        for (int i = 0; i < num_pressed_keys; i++) {
            if (pressed_keys[i] == usage) {
                return;
            }
        }
        pressed_keys[num_pressed_keys++] = usage;
    }
}

void keyboard_release(uint16_t usage_page, uint16_t usage) {
    if (usage_page != 0x07) {
        return;
    }

    if (usage == 0) {
        return;
    }

#if NKRO_ENABLED
    if (usage < 256) {
        nkro_keys[usage] = false;
    }
#endif

    for (int i = 0; i < num_pressed_keys; i++) {
        if (pressed_keys[i] == usage) {
            for (int j = i; j < num_pressed_keys - 1; j++) {
                pressed_keys[j] = pressed_keys[j + 1];
            }
            pressed_keys[--num_pressed_keys] = 0;
            break;
        }
    }
}

void keyboard_tap(uint16_t usage_page, uint16_t usage, uint16_t delay_ms) {
    keyboard_press(usage_page, usage, 0);
    keyboard_send_report();
    if (delay_ms > 0) {
        delay_ms(delay_ms);
    }
    keyboard_release(usage_page, usage);
    keyboard_send_report();
}

void keyboard_set_modifiers(uint8_t modifiers) {
    current_modifiers |= modifiers;
}

void keyboard_clear_modifiers(void) {
    current_modifiers = 0;
}

void keyboard_release_all(void) {
    current_modifiers = 0;
    num_pressed_keys = 0;
    memset(pressed_keys, 0, sizeof(pressed_keys));
#if NKRO_ENABLED
    memset(nkro_keys, 0, sizeof(nkro_keys));
#endif
}

void keyboard_send_report(void) {
    extern bool hid_send_keyboard_report(const hid_keyboard_report_t* report);

    hid_keyboard_report_t report = {
        .report_id = HID_KEYBOARD_REPORT_ID,
        .modifiers = current_modifiers,
        .reserved = 0,
        .keys = {0}
    };

#if NKRO_ENABLED
    uint8_t idx = 0;
    for (int i = 0; i < 256 && idx < MAX_KEYS_PER_REPORT; i++) {
        if (nkro_keys[i]) {
            report.keys[idx++] = i;
        }
    }
#else
    for (int i = 0; i < num_pressed_keys && i < MAX_KEYS_PER_REPORT; i++) {
        report.keys[i] = pressed_keys[i];
    }
#endif

    hid_send_keyboard_report(&report);
}

uint8_t keyboard_get_modifiers(void) {
    return current_modifiers;
}

uint8_t keyboard_get_num_pressed(void) {
    return num_pressed_keys;
}

const uint8_t* keyboard_get_pressed_keys(void) {
    return pressed_keys;
}

bool keyboard_is_key_pressed(uint16_t usage) {
#if NKRO_ENABLED
    if (usage < 256) {
        return nkro_keys[usage];
    }
#endif
    for (int i = 0; i < num_pressed_keys; i++) {
        if (pressed_keys[i] == usage) {
            return true;
        }
    }
    return false;
}