#include "config.h"
#include "hid_reports.h"
#include "protocol.h"

static hid_keyboard_report_t keyboard_report = {
    .report_id = HID_KEYBOARD_REPORT_ID,
    .modifiers = 0,
    .reserved = 0,
    .keys = {0}
};

static hid_mouse_report_t mouse_report = {
    .report_id = HID_MOUSE_REPORT_ID,
    .buttons = 0,
    .x = 0,
    .y = 0,
    .wheel = 0,
    .pan = 0
};

static hid_consumer_report_t consumer_report = {
    .report_id = HID_CONSUMER_REPORT_ID,
    .usage = 0
};

static hid_system_report_t system_report = {
    .report_id = HID_SYSTEM_REPORT_ID,
    .usage = 0
};

static hid_raw_report_t raw_hid_tx_report = {
    .report_id = HID_RAW_REPORT_ID,
    .data = {0}
};

static hid_raw_report_t raw_hid_rx_report = {
    .report_id = HID_RAW_REPORT_ID,
    .data = {0}
};

static bool keyboard_report_ready = false;
static bool mouse_report_ready = false;
static bool consumer_report_ready = false;
static bool system_report_ready = false;
static bool raw_hid_tx_ready = false;
static bool raw_hid_rx_received = false;

static uint8_t usb_state = 0;
static uint32_t last_heartbeat = 0;

extern void protocol_handle_raw_data(const uint8_t* data, uint16_t length);

void usb_callbacks_init(void) {
    keyboard_report_ready = false;
    mouse_report_ready = false;
    consumer_report_ready = false;
    system_report_ready = false;
    raw_hid_tx_ready = false;
    raw_hid_rx_received = false;
    usb_state = 1;
    last_heartbeat = 0;
}

void usb_callback_reset(void) {
    usb_callbacks_init();
    protocol_init();
}

void usb_callback_set_configuration(uint8_t config) {
    if (config > 0) {
        usb_state = 2;
    } else {
        usb_state = 1;
    }
}

void usb_callback_suspend(void) {
    usb_state |= 0x80;
}

void usb_callback_resume(void) {
    usb_state &= ~0x80;
}

bool usb_callback_control_request(uint8_t request, uint8_t request_type,
                                   uint16_t value, uint16_t index,
                                   uint16_t length, uint8_t* data) {
    uint8_t report_type = (value >> 8) & 0xFF;
    uint8_t report_id = value & 0xFF;
    uint8_t interface = index & 0xFF;

    switch (request) {
        case 0x01: // GET_REPORT
            if (request_type == 0xA1) {
                switch (report_type) {
                    case 0x01: // Input
                        switch (report_id) {
                            case HID_KEYBOARD_REPORT_ID:
                                if (length >= sizeof(hid_keyboard_report_t)) {
                                    memcpy(data, &keyboard_report, sizeof(hid_keyboard_report_t));
                                    return true;
                                }
                                break;
                            case HID_MOUSE_REPORT_ID:
                                if (length >= sizeof(hid_mouse_report_t)) {
                                    memcpy(data, &mouse_report, sizeof(hid_mouse_report_t));
                                    return true;
                                }
                                break;
                            case HID_CONSUMER_REPORT_ID:
                                if (length >= sizeof(hid_consumer_report_t)) {
                                    memcpy(data, &consumer_report, sizeof(hid_consumer_report_t));
                                    return true;
                                }
                                break;
                            case HID_SYSTEM_REPORT_ID:
                                if (length >= sizeof(hid_system_report_t)) {
                                    memcpy(data, &system_report, sizeof(hid_system_report_t));
                                    return true;
                                }
                                break;
                            case HID_RAW_REPORT_ID:
                                if (length >= sizeof(hid_raw_report_t)) {
                                    memcpy(data, &raw_hid_tx_report, sizeof(hid_raw_report_t));
                                    return true;
                                }
                                break;
                        }
                        break;
                    case 0x02: // Output
                        break;
                    case 0x03: // Feature
                        break;
                }
            }
            break;

        case 0x09: // SET_REPORT
            if (request_type == 0x21) {
                switch (report_type) {
                    case 0x02: // Output
                        if (report_id == HID_RAW_REPORT_ID && length >= sizeof(hid_raw_report_t)) {
                            memcpy(&raw_hid_rx_report, data, sizeof(hid_raw_report_t));
                            raw_hid_rx_received = true;
                            protocol_handle_raw_data(raw_hid_rx_report.data, RAW_HID_EPSIZE - 1);
                            return true;
                        }
                        break;
                    case 0x03: // Feature
                        break;
                }
            }
            break;

        case 0x0A: // SET_IDLE
            return true;

        case 0x0B: // SET_PROTOCOL
            return true;
    }
    return false;
}

void usb_callback_endpoint_in_complete(uint8_t endpoint) {
    switch (endpoint) {
        case 0x81: // Keyboard
            keyboard_report_ready = false;
            break;
        case 0x82: // Mouse
            mouse_report_ready = false;
            break;
        case 0x84: // Raw HID TX
            raw_hid_tx_ready = false;
            break;
    }
}

void usb_callback_endpoint_out_complete(uint8_t endpoint) {
    if (endpoint == 0x03) { // Raw HID RX
        raw_hid_rx_received = true;
        protocol_handle_raw_data(raw_hid_rx_report.data, RAW_HID_EPSIZE - 1);
    }
}

void usb_callback_sof(uint16_t frame_number) {
    (void)frame_number;
}

uint32_t usb_get_uptime_ms(void);

void usb_callback_tick(void) {
    uint32_t now = usb_get_uptime_ms();
    if (now - last_heartbeat >= HEARTBEAT_INTERVAL_MS) {
        last_heartbeat = now;
        protocol_tick();
    }
}

bool hid_send_keyboard_report(const hid_keyboard_report_t* report) {
    if (keyboard_report_ready) return false;
    memcpy(&keyboard_report, report, sizeof(hid_keyboard_report_t));
    keyboard_report_ready = true;
    return true;
}

bool hid_send_mouse_report(const hid_mouse_report_t* report) {
    if (mouse_report_ready) return false;
    memcpy(&mouse_report, report, sizeof(hid_mouse_report_t));
    mouse_report_ready = true;
    return true;
}

bool hid_send_consumer_report(const hid_consumer_report_t* report) {
    if (consumer_report_ready) return false;
    memcpy(&consumer_report, report, sizeof(hid_consumer_report_t));
    consumer_report_ready = true;
    return true;
}

bool hid_send_system_report(const hid_system_report_t* report) {
    if (system_report_ready) return false;
    memcpy(&system_report, report, sizeof(hid_system_report_t));
    system_report_ready = true;
    return true;
}

bool hid_send_raw_report(const hid_raw_report_t* report) {
    if (raw_hid_tx_ready) return false;
    memcpy(&raw_hid_tx_report, report, sizeof(hid_raw_report_t));
    raw_hid_tx_ready = true;
    return true;
}

const hid_keyboard_report_t* hid_get_keyboard_report(void) {
    return &keyboard_report;
}

const hid_mouse_report_t* hid_get_mouse_report(void) {
    return &mouse_report;
}

const hid_consumer_report_t* hid_get_consumer_report(void) {
    return &consumer_report;
}

const hid_system_report_t* hid_get_system_report(void) {
    return &system_report;
}

const hid_raw_report_t* hid_get_raw_tx_report(void) {
    return &raw_hid_tx_report;
}

bool hid_is_keyboard_ready(void) {
    return keyboard_report_ready;
}

bool hid_is_mouse_ready(void) {
    return mouse_report_ready;
}

bool hid_is_consumer_ready(void) {
    return consumer_report_ready;
}

bool hid_is_system_ready(void) {
    return system_report_ready;
}

bool hid_is_raw_tx_ready(void) {
    return raw_hid_tx_ready;
}

bool hid_is_raw_rx_received(void) {
    return raw_hid_rx_received;
}

void hid_clear_raw_rx_flag(void) {
    raw_hid_rx_received = false;
}

uint8_t usb_get_state(void) {
    return usb_state;
}

void usb_set_led(bool on) {
#if LED_ACTIVE_HIGH
    if (on) {
        // GPIO_SET(LED_PIN);
    } else {
        // GPIO_CLEAR(LED_PIN);
    }
#else
    if (on) {
        // GPIO_CLEAR(LED_PIN);
    } else {
        // GPIO_SET(LED_PIN);
    }
#endif
}