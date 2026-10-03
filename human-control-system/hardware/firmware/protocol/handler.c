#include <string.h>
#include "protocol.h"
#include "config.h"
#include "hid_reports.h"
#include "input/keyboard.h"
#include "input/mouse.h"
#include "input/consumer.h"
#include "input/system.h"

#ifdef __ARMCC_VERSION
#include "cmsis_nvic.h"
#else
// CMSIS NVIC functions
static inline void NVIC_SystemReset(void) {
    // System reset via AIRCR register
    *((volatile uint32_t*)0xE000ED0C) = 0x05FA0004;
}
#endif

static protocol_handler_t handlers[256] = {0};
static uint16_t next_sequence = 1;

static uint32_t commands_processed = 0;
static uint32_t commands_failed = 0;
static uint32_t bytes_received = 0;
static uint32_t bytes_sent = 0;

extern void usb_callback_tick(void);
extern uint32_t usb_get_uptime_ms(void);
extern uint8_t usb_get_state(void);
extern void usb_set_led(bool on);

void protocol_register_handler(uint8_t cmd, protocol_handler_t handler) {
    if (cmd < 256) {
        handlers[cmd] = handler;
    }
}

static void handle_key_down(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                            uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_key_down_t)) {
        *response_len = 0;
        return;
    }
    const cmd_key_down_t* cmd = (const cmd_key_down_t*)payload;
    keyboard_press(cmd->usage_page, cmd->usage, cmd->modifiers);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_key_up(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                          uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_key_up_t)) {
        *response_len = 0;
        return;
    }
    const cmd_key_up_t* cmd = (const cmd_key_up_t*)payload;
    keyboard_release(cmd->usage_page, cmd->usage);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_key_tap(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                           uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_key_tap_t)) {
        *response_len = 0;
        return;
    }
    const cmd_key_tap_t* cmd = (const cmd_key_tap_t*)payload;
    keyboard_tap(cmd->usage_page, cmd->usage, cmd->delay_ms);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_modifier_set(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                                uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_modifier_set_t)) {
        *response_len = 0;
        return;
    }
    const cmd_modifier_set_t* cmd = (const cmd_modifier_set_t*)payload;
    keyboard_set_modifiers(cmd->modifiers);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_modifier_clear(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                                  uint8_t* response, uint16_t* response_len) {
    (void)payload;
    (void)payload_len;
    keyboard_clear_modifiers();
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_mouse_move(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                              uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_mouse_move_t)) {
        *response_len = 0;
        return;
    }
    const cmd_mouse_move_t* cmd = (const cmd_mouse_move_t*)payload;
    mouse_move(cmd->x, cmd->y, cmd->absolute);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_mouse_click(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                               uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_mouse_click_t)) {
        *response_len = 0;
        return;
    }
    const cmd_mouse_click_t* cmd = (const cmd_mouse_click_t*)payload;
    if (cmd->down) {
        mouse_press(cmd->button);
    } else {
        mouse_release(cmd->button);
    }
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_mouse_wheel(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                               uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_mouse_wheel_t)) {
        *response_len = 0;
        return;
    }
    const cmd_mouse_wheel_t* cmd = (const cmd_mouse_wheel_t*)payload;
    mouse_wheel(cmd->wheel, cmd->pan);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_consumer_press(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                                  uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_consumer_t)) {
        *response_len = 0;
        return;
    }
    const cmd_consumer_t* cmd = (const cmd_consumer_t*)payload;
    consumer_press(cmd->usage);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_consumer_release(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                                    uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_consumer_t)) {
        *response_len = 0;
        return;
    }
    const cmd_consumer_t* cmd = (const cmd_consumer_t*)payload;
    consumer_release(cmd->usage);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_consumer_tap(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                                uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_consumer_t)) {
        *response_len = 0;
        return;
    }
    const cmd_consumer_t* cmd = (const cmd_consumer_t*)payload;
    consumer_tap(cmd->usage);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_system_press(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                                uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_system_t)) {
        *response_len = 0;
        return;
    }
    const cmd_system_t* cmd = (const cmd_system_t*)payload;
    system_press(cmd->usage);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_system_release(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                                  uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_system_t)) {
        *response_len = 0;
        return;
    }
    const cmd_system_t* cmd = (const cmd_system_t*)payload;
    system_release(cmd->usage);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_system_tap(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                              uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_system_t)) {
        *response_len = 0;
        return;
    }
    const cmd_system_t* cmd = (const cmd_system_t*)payload;
    system_tap(cmd->usage);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_raw_hid_send(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                                uint8_t* response, uint16_t* response_len) {
    extern bool hid_send_raw_report(const hid_raw_report_t* report);
    hid_raw_report_t report = {.report_id = HID_RAW_REPORT_ID};
    uint16_t copy_len = (payload_len < RAW_HID_EPSIZE - 1) ? payload_len : RAW_HID_EPSIZE - 1;
    memcpy(report.data, payload, copy_len);
    hid_send_raw_report(&report);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_heartbeat(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                             uint8_t* response, uint16_t* response_len) {
    (void)payload;
    (void)payload_len;
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_reset(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                         uint8_t* response, uint16_t* response_len) {
    (void)payload;
    (void)payload_len;
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
    NVIC_SystemReset();
}

static void handle_get_version(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                               uint8_t* response, uint16_t* response_len) {
    (void)payload;
    (void)payload_len;
    resp_version_t version = {
        .major = FIRMWARE_VERSION_MAJOR,
        .minor = FIRMWARE_VERSION_MINOR,
        .patch = FIRMWARE_VERSION_PATCH,
        .build_timestamp = 0
    };
    protocol_build_response_packet(header->sequence, RESP_VERSION, &version, sizeof(version), response, response_len);
}

static void handle_get_status(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                              uint8_t* response, uint16_t* response_len) {
    (void)payload;
    (void)payload_len;
    resp_status_t status = {
        .uptime_ms = usb_get_uptime_ms(),
        .commands_processed = commands_processed,
        .commands_failed = commands_failed,
        .bytes_received = bytes_received,
        .bytes_sent = bytes_sent,
        .pending_acks = 0,
        .usb_state = usb_get_state(),
    };
    protocol_build_response_packet(header->sequence, RESP_STATUS, &status, sizeof(status), response, response_len);
}

static void handle_set_led(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                           uint8_t* response, uint16_t* response_len) {
    if (payload_len < sizeof(cmd_set_led_t)) {
        *response_len = 0;
        return;
    }
    const cmd_set_led_t* cmd = (const cmd_set_led_t*)payload;
    usb_set_led(cmd->state != 0);
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
}

static void handle_bootloader(const protocol_header_t* header, const uint8_t* payload, uint16_t payload_len,
                              uint8_t* response, uint16_t* response_len) {
    (void)payload;
    (void)payload_len;
    protocol_build_response_packet(header->sequence, RESP_OK, NULL, 0, response, response_len);
    __asm volatile ("bkpt #251");
}

void protocol_handlers_init(void) {
    protocol_register_handler(CMD_KEY_DOWN, handle_key_down);
    protocol_register_handler(CMD_KEY_UP, handle_key_up);
    protocol_register_handler(CMD_KEY_TAP, handle_key_tap);
    protocol_register_handler(CMD_MODIFIER_SET, handle_modifier_set);
    protocol_register_handler(CMD_MODIFIER_CLEAR, handle_modifier_clear);
    protocol_register_handler(CMD_MOUSE_MOVE, handle_mouse_move);
    protocol_register_handler(CMD_MOUSE_MOVE_ABS, handle_mouse_move);
    protocol_register_handler(CMD_MOUSE_CLICK, handle_mouse_click);
    protocol_register_handler(CMD_MOUSE_DOWN, handle_mouse_click);
    protocol_register_handler(CMD_MOUSE_UP, handle_mouse_click);
    protocol_register_handler(CMD_MOUSE_WHEEL, handle_mouse_wheel);
    protocol_register_handler(CMD_MOUSE_PAN, handle_mouse_wheel);
    protocol_register_handler(CMD_CONSUMER_PRESS, handle_consumer_press);
    protocol_register_handler(CMD_CONSUMER_RELEASE, handle_consumer_release);
    protocol_register_handler(CMD_CONSUMER_TAP, handle_consumer_tap);
    protocol_register_handler(CMD_SYSTEM_PRESS, handle_system_press);
    protocol_register_handler(CMD_SYSTEM_RELEASE, handle_system_release);
    protocol_register_handler(CMD_SYSTEM_TAP, handle_system_tap);
    protocol_register_handler(CMD_RAW_HID_SEND, handle_raw_hid_send);
    protocol_register_handler(CMD_HEARTBEAT, handle_heartbeat);
    protocol_register_handler(CMD_RESET, handle_reset);
    protocol_register_handler(CMD_GET_VERSION, handle_get_version);
    protocol_register_handler(CMD_GET_STATUS, handle_get_status);
    protocol_register_handler(CMD_SET_LED, handle_set_led);
    protocol_register_handler(CMD_BOOTLOADER, handle_bootloader);
}

bool protocol_process(const uint8_t* data, uint16_t length, uint8_t* response, uint16_t* response_len) {
    if (!data || !response || !response_len || length == 0) {
        return false;
    }

    bytes_received += length;

    protocol_header_t header;
    uint8_t payload[PROTOCOL_MAX_PAYLOAD];
    uint16_t payload_len = 0;

    if (!protocol_parse_packet(data, length, &header, payload, &payload_len)) {
        return false;
    }

    commands_processed++;

    protocol_handler_t handler = handlers[header.cmd];
    if (handler) {
        handler(&header, payload, payload_len, response, response_len);
    } else {
        commands_failed++;
        protocol_build_response_packet(header.sequence, RESP_INVALID_CMD, NULL, 0, response, response_len);
    }

    if (*response_len > 0) {
        bytes_sent += *response_len;
    }

    return true;
}

bool protocol_send_ack(uint16_t sequence, uint8_t cmd, uint8_t status) {
    uint8_t buffer[RAW_HID_EPSIZE];
    uint16_t buffer_len = 0;
    if (protocol_build_ack_packet(sequence, cmd, status, buffer, &buffer_len)) {
        extern bool hid_send_raw_report(const hid_raw_report_t* report);
        hid_raw_report_t report = {.report_id = HID_RAW_REPORT_ID};
        memcpy(report.data, buffer, buffer_len);
        bytes_sent += buffer_len;
        return hid_send_raw_report(&report);
    }
    return false;
}

bool protocol_send_response(uint16_t sequence, uint8_t resp_code, const void* data, uint16_t length) {
    uint8_t buffer[RAW_HID_EPSIZE];
    uint16_t buffer_len = 0;
    if (protocol_build_response_packet(sequence, resp_code, data, length, buffer, &buffer_len)) {
        extern bool hid_send_raw_report(const hid_raw_report_t* report);
        hid_raw_report_t report = {.report_id = HID_RAW_REPORT_ID};
        memcpy(report.data, buffer, buffer_len);
        bytes_sent += buffer_len;
        return hid_send_raw_report(&report);
    }
    return false;
}

void protocol_tick(void) {
    static uint32_t last_heartbeat_sent = 0;
    uint32_t now = usb_get_uptime_ms();

    if (now - last_heartbeat_sent >= HEARTBEAT_INTERVAL_MS) {
        last_heartbeat_sent = now;
        uint8_t buffer[RAW_HID_EPSIZE];
        uint16_t buffer_len = 0;
        if (protocol_build_packet(CMD_HEARTBEAT, next_sequence++, NULL, 0, buffer, &buffer_len)) {
            extern bool hid_send_raw_report(const hid_raw_report_t* report);
            hid_raw_report_t report = {.report_id = HID_RAW_REPORT_ID};
            memcpy(report.data, buffer, buffer_len);
            hid_send_raw_report(&report);
        }
    }
}

uint16_t protocol_get_next_sequence(void) {
    return next_sequence++;
}

uint32_t protocol_get_commands_processed(void) {
    return commands_processed;
}

uint32_t protocol_get_commands_failed(void) {
    return commands_failed;
}

uint32_t protocol_get_bytes_received(void) {
    return bytes_received;
}

uint32_t protocol_get_bytes_sent(void) {
    return bytes_sent;
}