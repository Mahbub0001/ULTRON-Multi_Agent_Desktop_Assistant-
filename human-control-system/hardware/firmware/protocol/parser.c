#include <string.h>
#include "protocol.h"
#include "config.h"

#define PARSER_STATE_IDLE       0
#define PARSER_STATE_HEADER     1
#define PARSER_STATE_PAYLOAD    2

typedef struct {
    uint8_t state;
    protocol_header_t header;
    uint16_t bytes_received;
    uint16_t payload_offset;
    uint8_t buffer[RAW_HID_EPSIZE];
} parser_state_t;

static parser_state_t parser;
static bool parser_initialized = false;

bool protocol_init(void) {
    memset(&parser, 0, sizeof(parser));
    parser.state = PARSER_STATE_IDLE;
    parser_initialized = true;
    return true;
}

static bool parse_header(const uint8_t* data, uint16_t length) {
    if (length < PROTOCOL_HEADER_SIZE) {
        return false;
    }

    memcpy(&parser.header, data, PROTOCOL_HEADER_SIZE);

    if (parser.header.magic != PROTOCOL_MAGIC_VAL) {
        parser.state = PARSER_STATE_IDLE;
        return false;
    }

    if (parser.header.version != PROTOCOL_VERSION_VAL) {
        parser.state = PARSER_STATE_IDLE;
        return false;
    }

    if (parser.header.length > PROTOCOL_MAX_PAYLOAD) {
        parser.state = PARSER_STATE_IDLE;
        return false;
    }

    parser.payload_offset = 0;
    if (parser.header.length > 0) {
        parser.state = PARSER_STATE_PAYLOAD;
    } else {
        parser.state = PARSER_STATE_IDLE;
        return true;
    }

    return true;
}

static bool parse_payload(const uint8_t* data, uint16_t length) {
    uint16_t remaining = parser.header.length - parser.payload_offset;
    uint16_t to_copy = (length < remaining) ? length : remaining;

    memcpy(parser.buffer + parser.payload_offset, data, to_copy);
    parser.payload_offset += to_copy;

    if (parser.payload_offset >= parser.header.length) {
        parser.state = PARSER_STATE_IDLE;
        return true;
    }

    return false;
}

bool protocol_parse_packet(const uint8_t* data, uint16_t length,
                           protocol_header_t* out_header,
                           uint8_t* out_payload, uint16_t* out_payload_len) {
    if (!parser_initialized) {
        protocol_init();
    }

    uint16_t offset = 0;

    while (offset < length) {
        switch (parser.state) {
            case PARSER_STATE_IDLE:
                if (length - offset >= PROTOCOL_HEADER_SIZE) {
                    if (parse_header(data + offset, length - offset)) {
                        offset += PROTOCOL_HEADER_SIZE;
                        if (parser.state == PARSER_STATE_IDLE) {
                            *out_header = parser.header;
                            *out_payload_len = 0;
                            return true;
                        }
                    } else {
                        offset++;
                    }
                } else {
                    return false;
                }
                break;

            case PARSER_STATE_PAYLOAD:
                if (parse_payload(data + offset, length - offset)) {
                    offset += parser.payload_offset;
                    *out_header = parser.header;
                    memcpy(out_payload, parser.buffer, parser.header.length);
                    *out_payload_len = parser.header.length;
                    return true;
                } else {
                    offset = length;
                }
                break;

            case PARSER_STATE_HEADER:
            default:
                parser.state = PARSER_STATE_IDLE;
                break;
        }
    }

    return false;
}

bool protocol_build_packet(uint8_t cmd, uint16_t sequence,
                           const void* payload, uint16_t payload_len,
                           uint8_t* buffer, uint16_t* buffer_len) {
    if (!buffer || !buffer_len || payload_len > PROTOCOL_MAX_PAYLOAD) {
        return false;
    }

    protocol_header_t header = {
        .magic = PROTOCOL_MAGIC_VAL,
        .version = PROTOCOL_VERSION_VAL,
        .cmd = cmd,
        .sequence = sequence,
        .length = payload_len
    };

    uint16_t total_len = PROTOCOL_HEADER_SIZE + payload_len;
    if (total_len > RAW_HID_EPSIZE) {
        return false;
    }

    memcpy(buffer, &header, PROTOCOL_HEADER_SIZE);
    if (payload && payload_len > 0) {
        memcpy(buffer + PROTOCOL_HEADER_SIZE, payload, payload_len);
    }

    *buffer_len = total_len;
    return true;
}

bool protocol_build_ack_packet(uint16_t sequence, uint8_t cmd, uint8_t status,
                               uint8_t* buffer, uint16_t* buffer_len) {
    ack_packet_t ack = {
        .sequence = sequence,
        .cmd = cmd,
        .status = status
    };
    return protocol_build_packet(RESP_ACK, sequence, &ack, sizeof(ack), buffer, buffer_len);
}

bool protocol_build_response_packet(uint16_t sequence, uint8_t resp_code,
                                    const void* data, uint16_t data_len,
                                    uint8_t* buffer, uint16_t* buffer_len) {
    return protocol_build_packet(resp_code, sequence, data, data_len, buffer, buffer_len);
}