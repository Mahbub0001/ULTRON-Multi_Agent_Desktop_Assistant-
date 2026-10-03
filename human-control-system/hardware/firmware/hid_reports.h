#pragma once

#include <stdint.h>
#include "config.h"

#pragma pack(push, 1)

typedef struct {
    uint8_t report_id;
    uint8_t modifiers;
    uint8_t reserved;
    uint8_t keys[MAX_KEYS_PER_REPORT];
} hid_keyboard_report_t;

typedef struct {
    uint8_t report_id;
    uint8_t buttons;
    int8_t x;
    int8_t y;
    int8_t wheel;
    int8_t pan;
} hid_mouse_report_t;

typedef struct {
    uint8_t report_id;
    uint16_t usage;
} hid_consumer_report_t;

typedef struct {
    uint8_t report_id;
    uint8_t usage;
} hid_system_report_t;

typedef struct {
    uint8_t report_id;
    uint8_t data[RAW_HID_EPSIZE - 1];
} hid_raw_report_t;

#pragma pack(pop)

typedef struct {
    uint16_t usage_page;
    uint16_t usage;
    bool pressed;
} key_event_t;

typedef struct {
    int16_t x;
    int16_t y;
    int8_t wheel;
    int8_t pan;
    uint8_t buttons;
    bool absolute;
} mouse_event_t;

typedef enum {
    HID_REPORT_KEYBOARD = HID_KEYBOARD_REPORT_ID,
    HID_REPORT_MOUSE = HID_MOUSE_REPORT_ID,
    HID_REPORT_CONSUMER = HID_CONSUMER_REPORT_ID,
    HID_REPORT_SYSTEM = HID_SYSTEM_REPORT_ID,
    HID_REPORT_RAW = HID_RAW_REPORT_ID,
} hid_report_type_t;

#define HID_KEYBOARD_MODIFIER_LEFT_CTRL   (1 << 0)
#define HID_KEYBOARD_MODIFIER_LEFT_SHIFT  (1 << 1)
#define HID_KEYBOARD_MODIFIER_LEFT_ALT    (1 << 2)
#define HID_KEYBOARD_MODIFIER_LEFT_GUI    (1 << 3)
#define HID_KEYBOARD_MODIFIER_RIGHT_CTRL  (1 << 4)
#define HID_KEYBOARD_MODIFIER_RIGHT_SHIFT (1 << 5)
#define HID_KEYBOARD_MODIFIER_RIGHT_ALT   (1 << 6)
#define HID_KEYBOARD_MODIFIER_RIGHT_GUI   (1 << 7)

#define HID_MOUSE_BUTTON_LEFT   (1 << 0)
#define HID_MOUSE_BUTTON_RIGHT  (1 << 1)
#define HID_MOUSE_BUTTON_MIDDLE (1 << 2)
#define HID_MOUSE_BUTTON_BACK   (1 << 3)
#define HID_MOUSE_BUTTON_FORWARD (1 << 4)

#define HID_CONSUMER_VOLUME_UP     0xE9
#define HID_CONSUMER_VOLUME_DOWN   0xEA
#define HID_CONSUMER_MUTE          0xE2
#define HID_CONSUMER_PLAY_PAUSE    0xCD
#define HID_CONSUMER_NEXT_TRACK    0xB5
#define HID_CONSUMER_PREV_TRACK    0xB6
#define HID_CONSUMER_STOP          0xB7

#define HID_SYSTEM_POWER_DOWN      0x81
#define HID_SYSTEM_SLEEP           0x82
#define HID_SYSTEM_WAKE_UP         0x83

#endif