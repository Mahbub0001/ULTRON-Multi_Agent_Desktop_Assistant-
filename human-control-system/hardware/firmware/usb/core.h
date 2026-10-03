#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "config.h"

#ifdef __cplusplus
extern "C" {
#endif

void usb_init(void);
void usb_task(void);

void usb_isr(void);

bool usb_configured(void);
uint8_t usb_get_state(void);

void usb_send_callback(uint8_t endpoint, const uint8_t* data, uint16_t length);
bool usb_endpoint_ready(uint8_t endpoint);

#ifdef __cplusplus
}
#endif