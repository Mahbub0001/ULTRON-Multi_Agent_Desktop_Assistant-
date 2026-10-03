#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "config.h"

#pragma pack(push, 1)

typedef enum {
    CMD_KEY_DOWN        = 0x01,
    CMD_KEY_UP          = 0x02,
    CMD_KEY_TAP         = 0x03,
    CMD_MODIFIER_SET    = 0x04,
    CMD_MODIFIER_CLEAR  = 0x05,
    CMD_MOUSE_MOVE      = 0x10,
    CMD_MOUSE_MOVE_ABS  = 0x11,
    CMD_MOUSE_CLICK     = 0x12,
    CMD_MOUSE_DOWN      = 0x13,
    CMD_MOUSE_UP        = 0x14,
    CMD_MOUSE_WHEEL     = 0x15,
    CMD_MOUSE_PAN       = 0x16,
    CMD_CONSUMER_PRESS  = 0x20,
    CMD_CONSUMER_RELEASE = 0x21,
    CMD_CONSUMER_TAP    = 0x22,
    CMD_SYSTEM_PRESS    = 0x30,
    CMD_SYSTEM_RELEASE  = 0x31,
    CMD_SYSTEM_TAP      = 0x32,
    CMD_RAW_HID_SEND    = 0x40,
    CMD_RAW_HID_RECV    = 0x41,
    CMD_HEARTBEAT       = 0xF0,
    CMD_RESET           = 0xF1,
    CMD_GET_VERSION     = 0xF2,
    CMD_GET_STATUS      = 0xF3,
    CMD_SET_LED         = 0xF4,
    CMD_BOOTLOADER      = 0xFF,
} protocol_cmd_t;

typedef enum {
    RESP_OK             = 0x00,
    RESP_ERROR          = 0x01,
    RESP_INVALID_CMD    = 0x02,
    RESP_INVALID_PARAM  = 0x03,
    RESP_TIMEOUT        = 0x04,
    RESP_BUSY           = 0x05,
    RESP_NOT_SUPPORTED  = 0x06,
    RESP_ACK            = 0x10,
    RESP_NACK           = 0x11,
    RESP_VERSION        = 0x20,
    RESP_STATUS         = 0x21,
} protocol_resp_t;

typedef struct {
    uint32_t magic;
    uint8_t version;
    uint8_t cmd;
    uint16_t sequence;
    uint16_t length;
    uint8_t payload[0];
} protocol_header_t;

typedef struct {
    uint16_t usage_page;
    uint16_t usage;
    uint8_t modifiers;
    uint8_t reserved;
} cmd_key_down_t;

typedef struct {
    uint16_t usage_page;
    uint16_t usage;
} cmd_key_up_t;

typedef struct {
    uint16_t usage_page;
    uint16_t usage;
    uint16_t delay_ms;
} cmd_key_tap_t;

typedef struct {
    uint8_t modifiers;
} cmd_modifier_set_t;

typedef struct {
    int16_t x;
    int16_t y;
    bool absolute;
} cmd_mouse_move_t;

typedef struct {
    uint8_t button;
    bool down;
} cmd_mouse_click_t;

typedef struct {
    int8_t wheel;
    int8_t pan;
} cmd_mouse_wheel_t;

typedef struct {
    uint16_t usage;
} cmd_consumer_t;

typedef struct {
    uint8_t usage;
} cmd_system_t;

typedef struct {
    uint8_t data[RAW_HID_EPSIZE - 1];
} cmd_raw_hid_t;

typedef struct {
    uint8_t major;
    uint8_t minor;
    uint8_t patch;
    uint32_t build_timestamp;
} resp_version_t;

typedef struct {
    uint32_t uptime_ms;
    uint32_t commands_processed;
    uint32_t commands_failed;
    uint32_t bytes_received;
    uint32_t bytes_sent;
    uint16_t pending_acks;
    uint8_t usb_state;
    uint8_t reserved[5];
} resp_status_t;

typedef struct {
    uint8_t state;
} cmd_set_led_t;

typedef struct {
    uint16_t sequence;
    uint8_t cmd;
    uint8_t status;
} ack_packet_t;

#pragma pack(pop)

#define PROTOCOL_HEADER_SIZE sizeof(protocol_header_t)
#define PROTOCOL_MAX_PAYLOAD (RAW_HID_EPSIZE - PROTOCOL_HEADER_SIZE)

typedef void (*protocol_handler_t)(const protocol_header_t* header, uint8_t* response, uint16_t* response_len);

bool protocol_init(void);
bool protocol_process(const uint8_t* data, uint16_t length, uint8_t* response, uint16_t* response_len);
bool protocol_send_ack(uint16_t sequence, uint8_t cmd, uint8_t status);
bool protocol_send_response(uint16_t sequence, uint8_t resp_code, const void* data, uint16_t length);
void protocol_tick(void);
void protocol_register_handler(uint8_t cmd, protocol_handler_t handler);

#define PROTOCOL_MAGIC_VAL 0x48435300
#define PROTOCOL_VERSION_VAL 1

#endif