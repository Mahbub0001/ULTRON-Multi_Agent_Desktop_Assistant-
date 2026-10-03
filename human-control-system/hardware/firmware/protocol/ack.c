#include <string.h>
#include "protocol.h"
#include "config.h"

#define MAX_PENDING_ACKS 32

typedef struct {
    uint16_t sequence;
    uint8_t cmd;
    uint32_t timestamp;
    uint8_t retries;
    bool pending;
} pending_ack_t;

static pending_ack_t pending_acks[MAX_PENDING_ACKS] = {0};
static uint32_t ack_timestamp_base = 0;

void ack_init(void) {
    memset(pending_acks, 0, sizeof(pending_acks));
    ack_timestamp_base = 0;
}

bool ack_register(uint16_t sequence, uint8_t cmd) {
    for (int i = 0; i < MAX_PENDING_ACKS; i++) {
        if (!pending_acks[i].pending) {
            pending_acks[i].sequence = sequence;
            pending_acks[i].cmd = cmd;
            pending_acks[i].timestamp = ack_timestamp_base;
            pending_acks[i].retries = 0;
            pending_acks[i].pending = true;
            return true;
        }
    }
    return false;
}

bool ack_complete(uint16_t sequence, uint8_t status) {
    for (int i = 0; i < MAX_PENDING_ACKS; i++) {
        if (pending_acks[i].pending && pending_acks[i].sequence == sequence) {
            pending_acks[i].pending = false;
            return true;
        }
    }
    return false;
}

bool ack_is_pending(uint16_t sequence) {
    for (int i = 0; i < MAX_PENDING_ACKS; i++) {
        if (pending_acks[i].pending && pending_acks[i].sequence == sequence) {
            return true;
        }
    }
    return false;
}

void ack_tick(uint32_t current_time) {
    ack_timestamp_base = current_time;

    for (int i = 0; i < MAX_PENDING_ACKS; i++) {
        if (pending_acks[i].pending) {
            if (current_time - pending_acks[i].timestamp >= ACK_TIMEOUT_MS) {
                if (pending_acks[i].retries < 3) {
                    pending_acks[i].retries++;
                    pending_acks[i].timestamp = current_time;
                    protocol_send_ack(pending_acks[i].sequence, pending_acks[i].cmd, RESP_NACK);
                } else {
                    pending_acks[i].pending = false;
                }
            }
        }
    }
}

uint8_t ack_get_pending_count(void) {
    uint8_t count = 0;
    for (int i = 0; i < MAX_PENDING_ACKS; i++) {
        if (pending_acks[i].pending) count++;
    }
    return count;
}

void ack_clear_all(void) {
    for (int i = 0; i < MAX_PENDING_ACKS; i++) {
        pending_acks[i].pending = false;
    }
}

void ack_set_timestamp_base(uint32_t timestamp) {
    ack_timestamp_base = timestamp;
}

uint32_t ack_get_timestamp_base(void) {
    return ack_timestamp_base;
}