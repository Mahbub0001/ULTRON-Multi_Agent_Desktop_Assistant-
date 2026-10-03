#pragma once

#include <stdint.h>
#include <stdbool.h>
#include "config.h"
#include "hid_reports.h"

void system_init(void);
void system_press(uint8_t usage);
void system_release(uint8_t usage);
void system_tap(uint8_t usage);
void system_send_report(void);

bool system_is_pressed(uint8_t usage);